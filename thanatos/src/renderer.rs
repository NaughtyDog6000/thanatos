use std::{collections::VecDeque, mem::size_of, rc::Rc};

use crate::{
    assets::{self, Material, MaterialId, MeshId},
    camera::Camera,
    transform::Transform,
    window::Window,
    World,
};
use anyhow::Result;
use bytemuck::offset_of;
use glam::{Vec2, Vec3, Vec4};
use hephaestus::{
    buffer::Static,
    descriptor,
    image::{Image, ImageView},
    pipeline::{
        self, clear_colour, clear_depth, Framebuffer, ImageLayout, PipelineBindPoint, RenderPass,
        ShaderModule, Subpass, Viewport,
    },
    task::{Fence, Semaphore, SubmitInfo, Task},
    vertex::{self, AttributeType},
    BufferUsageFlags, Context, DescriptorType, Extent2D, Format, ImageAspectFlags, ImageUsageFlags,
    PipelineStageFlags, VkResult,
};
use log::info;
use styx::Element;
use tecs::EntityId;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
}

impl Vertex {
    pub fn info() -> vertex::Info {
        vertex::Info::new(size_of::<Self>())
            .attribute(AttributeType::Vec3, 0)
            .attribute(AttributeType::Vec3, offset_of!(Vertex, normal))
    }
}

struct Frame {
    task: Task,
    fence: Rc<Fence>,
}

impl Drop for Frame {
    fn drop(&mut self) {
        self.fence.wait().unwrap();
    }
}

pub struct RenderObject {
    pub mesh: MeshId,
    pub material: MaterialId,
}

pub struct Renderer {
    render_pass: RenderPass,
    pipeline: pipeline::Graphics,
    ui: styx::Renderer,
    framebuffers: Vec<Framebuffer>,
    semaphores: Vec<Rc<Semaphore>>,
    frame_index: usize,
    tasks: VecDeque<Frame>,
    camera_layout: Rc<descriptor::Layout>,
    object_layout: Rc<descriptor::Layout>,
    depth_images: Vec<Rc<Image>>,
    depth_views: Vec<Rc<ImageView>>,
    pub ctx: Context,
}

impl Renderer {
    pub const FRAMES_IN_FLIGHT: usize = 3;

    pub fn new(window: &Window) -> Result<Self> {
        let size = window.window.inner_size();
        let ctx = Context::new("thanatos", &window.window, (size.width, size.height))?;

        let vertex = ShaderModule::new(
            &ctx.device,
            &std::fs::read("assets/shaders/shader.vert.spv").unwrap(),
        )?;

        let fragment = ShaderModule::new(
            &ctx.device,
            &std::fs::read("assets/shaders/shader.frag.spv").unwrap(),
        )?;

        let render_pass = {
            let mut builder = RenderPass::builder();
            let colour = builder.attachment(
                ctx.swapchain.as_ref().unwrap().format,
                ImageLayout::UNDEFINED,
                ImageLayout::PRESENT_SRC_KHR,
            );
            let depth = builder.attachment(
                Format::D32_SFLOAT,
                ImageLayout::UNDEFINED,
                ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
            );
            builder.subpass(
                Subpass::new(PipelineBindPoint::GRAPHICS)
                    .colour(colour, ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .depth(depth, ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL),
            );
            builder.subpass(
                Subpass::new(PipelineBindPoint::GRAPHICS)
                    .colour(colour, ImageLayout::COLOR_ATTACHMENT_OPTIMAL),
            );
            builder.build(&ctx.device)?
        };

        let camera_layout = descriptor::Layout::new(&ctx, &[DescriptorType::UNIFORM_BUFFER], 1000)?;
        let object_layout =
            descriptor::Layout::new(&ctx, &[DescriptorType::UNIFORM_BUFFER; 2], 1000)?;

        let pipeline = pipeline::Graphics::builder()
            .vertex(&vertex)
            .vertex_info(Vertex::info())
            .fragment(&fragment)
            .render_pass(&render_pass)
            .subpass(0)
            .viewport(Viewport::Dynamic)
            .layouts(vec![&camera_layout, &object_layout])
            .depth()
            .build(&ctx.device)?;

        let ui = styx::Renderer::new(&ctx, &render_pass, 1)?;

        let (depth_images, depth_views) = Self::create_depth_images(&ctx)?;

        let framebuffers = ctx
            .swapchain
            .as_ref()
            .unwrap()
            .views
            .iter()
            .zip(&depth_views)
            .map(|(colour, depth)| render_pass.get_framebuffer(&ctx.device, &[colour, depth]))
            .collect::<VkResult<Vec<Framebuffer>>>()?;

        let semaphores = (0..Self::FRAMES_IN_FLIGHT)
            .map(|_| Semaphore::new(&ctx.device))
            .collect::<VkResult<Vec<Rc<Semaphore>>>>()?;

        Ok(Self {
            ctx,
            render_pass,
            pipeline,
            ui,
            framebuffers,
            semaphores,
            frame_index: 0,
            tasks: VecDeque::new(),
            camera_layout,
            object_layout,
            depth_images,
            depth_views,
        })
    }

    pub fn add(self) -> impl FnOnce(World) -> World {
        move |world| world.with_resource(self).with_ticker(Self::draw)
    }

    fn create_depth_images(ctx: &Context) -> VkResult<(Vec<Rc<Image>>, Vec<Rc<ImageView>>)> {
        let depth_images = ctx
            .swapchain
            .as_ref()
            .unwrap()
            .views
            .iter()
            .map(|_| {
                Image::new(
                    ctx,
                    Format::D32_SFLOAT,
                    ctx.swapchain.as_ref().unwrap().extent,
                    ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
                )
            })
            .collect::<VkResult<Vec<_>>>()?;

        let depth_views = depth_images
            .iter()
            .map(|image| {
                ImageView::new(
                    &ctx.device,
                    &image,
                    Format::D32_SFLOAT,
                    ImageAspectFlags::DEPTH,
                    ctx.swapchain.as_ref().unwrap().extent,
                )
            })
            .collect::<VkResult<Vec<_>>>()?;

        Ok((depth_images, depth_views))
    }

    pub fn recreate_swapchain(&mut self, size: (u32, u32)) -> VkResult<()> {
        unsafe { self.ctx.device.device_wait_idle()? }
        self.ctx.surface.extent = Extent2D {
            width: size.0,
            height: size.1,
        };
        self.ctx.recreate_swapchain().unwrap();

        self.framebuffers.clear();
        self.depth_views.clear();
        self.depth_images.clear();

        let (depth_images, depth_views) = Self::create_depth_images(&self.ctx)?;
        self.depth_images = depth_images;
        self.depth_views = depth_views;

        self.framebuffers = self
            .ctx
            .swapchain
            .as_ref()
            .unwrap()
            .views
            .iter()
            .zip(&self.depth_views)
            .map(|(colour, depth)| {
                self.render_pass
                    .get_framebuffer(&self.ctx.device, &[colour, depth])
            })
            .collect::<VkResult<Vec<Framebuffer>>>()?;

        Ok(())
    }

    pub fn draw(world: &World) {
        let mut renderer = world.get_mut::<Renderer>().unwrap();
        if renderer.tasks.len() > Renderer::FRAMES_IN_FLIGHT {
            let frame = renderer.tasks.pop_front().unwrap();
            drop(frame);
        }

        let mut task = Task::new();
        let image_available = Semaphore::new(&renderer.ctx.device).unwrap();
        let render_finished =
            renderer.semaphores[renderer.frame_index % Renderer::FRAMES_IN_FLIGHT].clone();
        let in_flight = Fence::new(&renderer.ctx.device).unwrap();
        let (image_index, suboptimal) = task
            .acquire_next_image(
                &renderer.ctx.device,
                renderer.ctx.swapchain.as_ref().unwrap(),
                image_available.clone(),
            )
            .unwrap();

        let window = world.get::<Window>().unwrap();
        let size = window.window.inner_size();

        if suboptimal {
            info!("Recreating swapchain");
            renderer
                .recreate_swapchain((size.width, size.height))
                .unwrap();
            return;
        }

        let camera = world.get::<Camera>().unwrap();
        let camera_buffer = Static::new(
            &renderer.ctx,
            bytemuck::cast_slice::<f32, u8>(&camera.get_matrix().to_cols_array()),
            BufferUsageFlags::UNIFORM_BUFFER,
        )
        .unwrap();
        let camera_set = renderer
            .camera_layout
            .alloc()
            .unwrap()
            .write_buffer(0, &camera_buffer)
            .finish();

        let clear_values = [clear_colour([0.0, 0.0, 0.0, 1.0]), clear_depth(1.0)];

        let assets = world.get::<assets::Manager>().unwrap();
        let (entities, render_objects) = world.query::<(EntityId, &RenderObject)>();
        let object_sets = entities
            .iter()
            .zip(render_objects.iter())
            .map(|(id, render_object)| {
                let transform = world
                    .get_component::<Transform>(*id)
                    .map(|x| *x)
                    .unwrap_or_default();
                let material = assets.get_material(render_object.material).unwrap();
                let transform_buffer = Static::new(
                    &renderer.ctx,
                    bytemuck::cast_slice::<f32, u8>(&transform.matrix().to_cols_array()),
                    BufferUsageFlags::UNIFORM_BUFFER,
                )
                .unwrap();
                let material_buffer = Static::new(
                    &renderer.ctx,
                    bytemuck::cast_slice::<Material, u8>(&[*material]),
                    BufferUsageFlags::UNIFORM_BUFFER,
                )
                .unwrap();
                let set = renderer
                    .object_layout
                    .alloc()
                    .unwrap()
                    .write_buffer(0, &transform_buffer)
                    .write_buffer(1, &material_buffer)
                    .finish();
                (transform_buffer, material_buffer, set)
            })
            .collect::<Vec<_>>();

        let mut ui_box = styx::Box {
            colour: Vec4::new(1.0, 0.0, 0.0, 1.0),
        };
        let mut scene = styx::Scene::new();
        ui_box.paint(
            styx::Area {
                origin: Vec2::ZERO,
                size: Vec2::new(800.0, 600.0),
            },
            &mut scene,
        );
        let frame = renderer
            .ui
            .prepare(
                &renderer.ctx,
                &scene,
                Vec2::new(size.width as f32, size.height as f32),
            )
            .unwrap();

        let cmd = renderer
            .ctx
            .command_pool
            .alloc()
            .unwrap()
            .begin()
            .unwrap()
            .begin_render_pass(
                &renderer.render_pass,
                renderer.framebuffers.get(image_index as usize).unwrap(),
                &clear_values,
            )
            .bind_graphics_pipeline(&renderer.pipeline)
            .set_viewport(size.width, size.height)
            .set_scissor(size.width, size.height)
            .bind_descriptor_set(&camera_set, 0);

        let cmd = render_objects.iter().zip(object_sets.iter()).fold(
            cmd,
            |cmd, (object, (_, _, set))| {
                let mesh = assets.get_mesh(object.mesh).unwrap();
                cmd.bind_vertex_buffer(&mesh.vertex_buffer, 0)
                    .bind_index_buffer(&mesh.index_buffer)
                    .bind_descriptor_set(set, 1)
                    .draw_indexed(mesh.num_indices, 1, 0, 0, 0)
            },
        );

        let cmd = renderer.ui.draw(frame, cmd);

        let cmd = cmd.end_render_pass().end().unwrap();

        task.submit(SubmitInfo {
            device: &renderer.ctx.device,
            queue: &renderer.ctx.device.queues.graphics,
            cmd: &cmd,
            wait: &[(image_available, PipelineStageFlags::TOP_OF_PIPE)],
            signal: &[render_finished.clone()],
            fence: in_flight.clone(),
        })
        .unwrap();

        let suboptimal = task
            .present(
                &renderer.ctx.device,
                renderer.ctx.swapchain.as_ref().unwrap(),
                image_index,
                &[render_finished],
            )
            .unwrap();

        if suboptimal {
            info!("Recreating swapchain");
            renderer
                .recreate_swapchain((size.width, size.height))
                .unwrap();
        }

        renderer.tasks.push_back(Frame {
            task,
            fence: in_flight,
        });

        renderer.frame_index += 1;
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe { self.ctx.device.device_wait_idle().unwrap() }
    }
}
