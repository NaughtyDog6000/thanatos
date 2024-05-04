use std::{collections::VecDeque, mem::size_of, rc::Rc};

use crate::{
    assets::{self, Material, MaterialId, MeshId},
    camera::Camera,
    event::Event,
    transform::Transform,
    window::{Mouse, Window},
    World,
};
use anyhow::Result;
use bytemuck::offset_of;
use glam::{Vec2, Vec3};
use hephaestus::{
    buffer::Static,
    descriptor,
    image::{Image, ImageInfo, ImageView},
    pipeline::{
        self, clear_colour, clear_depth, AttachmentInfo, Framebuffer, ImageLayout,
        PipelineBindPoint, RenderPass, ShaderModule, Subpass, Viewport,
    },
    task::{Fence, Semaphore, SubmitInfo, Task},
    vertex::{self, AttributeType},
    AttachmentLoadOp, AttachmentStoreOp, BufferUsageFlags, Context, DescriptorType, Extent2D,
    Format, ImageAspectFlags, ImageUsageFlags, PipelineStageFlags, SampleCountFlags, VkResult,
};
use log::info;
use serde::{Deserialize, Serialize};
use styx::{components, Element, Font, FontSettings, Signals};
use tecs::EntityId;
use winit::event::MouseButton;

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

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct RenderObject {
    pub mesh: MeshId,
    pub material: MaterialId,
}

#[derive(Clone, Copy)]
pub enum Anchor {
    TopLeft,
    Cursor,
    Center,
    BottomRight,
}

pub struct Ui {
    pub font: Rc<Font>,
    pub signals: Signals,
    elements: Vec<(Anchor, Box<dyn Element>)>,
    events: Vec<styx::Event>,
}

impl Ui {
    pub fn new() -> Self {
        let font = Rc::new(
            Font::from_bytes(
                std::fs::read("assets/fonts/JetBrainsMono-Medium.ttf").unwrap(),
                FontSettings::default(),
            )
            .unwrap(),
        );

        Self {
            font,
            signals: Signals::default(),
            events: Vec::new(),
            elements: Vec::new(),
        }
    }

    pub fn add<T: Element + 'static>(&mut self, anchor: Anchor, element: T) {
        self.elements.push((anchor, Box::new(element)))
    }

    pub fn event(world: &World, event: &Event) {
        let event = match event {
            Event::MousePress(button) => {
                let mouse = world.get::<Mouse>().unwrap();
                match button {
                    MouseButton::Left => styx::Event::Click(mouse.position),
                    MouseButton::Right => styx::Event::RightClick(mouse.position),
                    _ => return
                }
            }
            _ => return,
        };

        let mut ui = world.get_mut::<Ui>().unwrap();
        ui.events.push(event)
    }

    pub fn paint(&mut self, world: &World) -> styx::Scene {
        let window = world.get::<Window>().unwrap();
        let mouse = world.get::<Mouse>().unwrap();
        let window_size = window.window.inner_size();
        let window_size = Vec2::new(window_size.width as f32, window_size.height as f32);

        let constraint = styx::Constraint {
            min: Vec2::ZERO,
            max: window_size,
        };

        self.signals.clear();
        let mut scene = styx::Scene::new();
        self.elements.iter_mut().for_each(|(anchor, element)| {
            let size = element.layout(constraint);
            let origin = match anchor {
                Anchor::TopLeft => Vec2::ZERO,
                Anchor::Center => (window_size - size) / 2.0,
                Anchor::Cursor => mouse.position,
                Anchor::BottomRight => window_size - size,
            };

            element.paint(
                styx::Area { origin, size },
                &mut scene,
                &self.events,
                &mut self.signals,
            );
        });

        self.events.clear();
        self.elements.clear();

        scene
    }
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
    images: Vec<(Rc<Image>, Rc<Image>)>,
    views: Vec<(Rc<ImageView>, Rc<ImageView>)>,
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

        let samples = ctx.device.physical.get_samples();

        let render_pass = {
            let mut builder = RenderPass::builder();
            let colour = builder.attachment(
                ctx.swapchain.as_ref().unwrap().format,
                AttachmentInfo {
                    initial_layout: ImageLayout::UNDEFINED,
                    final_layout: ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                    load_op: AttachmentLoadOp::CLEAR,
                    store_op: AttachmentStoreOp::DONT_CARE,
                    samples,
                },
            );

            let depth = builder.attachment(
                Format::D32_SFLOAT,
                AttachmentInfo {
                    initial_layout: ImageLayout::UNDEFINED,
                    final_layout: ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                    load_op: AttachmentLoadOp::CLEAR,
                    store_op: AttachmentStoreOp::DONT_CARE,
                    samples,
                },
            );

            let resolve = builder.attachment(
                ctx.swapchain.as_ref().unwrap().format,
                AttachmentInfo {
                    initial_layout: ImageLayout::UNDEFINED,
                    final_layout: ImageLayout::PRESENT_SRC_KHR,
                    load_op: AttachmentLoadOp::DONT_CARE,
                    store_op: AttachmentStoreOp::STORE,
                    samples: SampleCountFlags::TYPE_1,
                },
            );

            builder.subpass(
                Subpass::new(PipelineBindPoint::GRAPHICS)
                    .colour(colour, ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .depth(depth, ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                    .resolve(resolve, ImageLayout::COLOR_ATTACHMENT_OPTIMAL),
            );
            builder.subpass(
                Subpass::new(PipelineBindPoint::GRAPHICS)
                    .colour(colour, ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .resolve(resolve, ImageLayout::COLOR_ATTACHMENT_OPTIMAL),
            );
            builder.build(&ctx.device)?
        };

        let camera_layout = descriptor::Layout::new(&ctx, &[DescriptorType::UNIFORM_BUFFER], 1000)?;
        let object_layout =
            descriptor::Layout::new(&ctx, &[DescriptorType::STORAGE_BUFFER; 2], 1000)?;

        let pipeline = pipeline::Graphics::builder()
            .vertex(&vertex)
            .vertex_info(Vertex::info())
            .fragment(&fragment)
            .render_pass(&render_pass)
            .subpass(0)
            .viewport(Viewport::Dynamic)
            .layouts(vec![&camera_layout, &object_layout])
            .depth()
            .multisampled(samples)
            .build(&ctx.device)?;

        let ui = styx::Renderer::new(&ctx, &render_pass, 1)?;

        let (images, views) = Self::create_images(&ctx)?;

        let framebuffers = ctx
            .swapchain
            .as_ref()
            .unwrap()
            .views
            .iter()
            .zip(&views)
            .map(|(resolve, (colour, depth))| {
                render_pass.get_framebuffer(&ctx.device, &[colour, depth, resolve])
            })
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
            images,
            views,
        })
    }

    pub fn add(self) -> impl FnOnce(World) -> World {
        move |world| {
            world
                .with_resource(self)
                .with_resource(Ui::new())
                .with_ticker(Self::draw)
                .with_handler(Ui::event)
        }
    }

    fn create_images(
        ctx: &Context,
    ) -> VkResult<(
        Vec<(Rc<Image>, Rc<Image>)>,
        Vec<(Rc<ImageView>, Rc<ImageView>)>,
    )> {
        let swapchain = ctx.swapchain.as_ref().unwrap();
        let samples = ctx.device.physical.get_samples();
        let images = ctx
            .swapchain
            .as_ref()
            .unwrap()
            .views
            .iter()
            .map(|_| {
                Ok((
                    Image::new(
                        ctx,
                        ImageInfo {
                            format: swapchain.format,
                            extent: swapchain.extent,
                            usage: ImageUsageFlags::COLOR_ATTACHMENT,
                            samples,
                        },
                    )?,
                    Image::new(
                        ctx,
                        ImageInfo {
                            format: Format::D32_SFLOAT,
                            extent: swapchain.extent,
                            usage: ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
                            samples,
                        },
                    )?,
                ))
            })
            .collect::<VkResult<Vec<_>>>()?;

        let views = images
            .iter()
            .map(|(colour, depth)| {
                Ok((
                    ImageView::new(
                        &ctx.device,
                        &colour,
                        swapchain.format,
                        ImageAspectFlags::COLOR,
                        swapchain.extent,
                    )?,
                    ImageView::new(
                        &ctx.device,
                        &depth,
                        Format::D32_SFLOAT,
                        ImageAspectFlags::DEPTH,
                        ctx.swapchain.as_ref().unwrap().extent,
                    )?,
                ))
            })
            .collect::<VkResult<Vec<_>>>()?;

        Ok((images, views))
    }

    pub fn recreate_swapchain(&mut self, size: (u32, u32)) -> VkResult<()> {
        unsafe { self.ctx.device.device_wait_idle()? }
        self.ctx.surface.extent = Extent2D {
            width: size.0,
            height: size.1,
        };
        self.ctx.recreate_swapchain().unwrap();

        self.framebuffers.clear();
        self.views.clear();
        self.images.clear();

        let (images, views) = Self::create_images(&self.ctx)?;
        self.images = images;
        self.views = views;

        self.framebuffers = self
            .ctx
            .swapchain
            .as_ref()
            .unwrap()
            .views
            .iter()
            .zip(&self.views)
            .map(|(resolve, (colour, depth))| {
                self.render_pass
                    .get_framebuffer(&self.ctx.device, &[colour, depth, resolve])
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

        let transforms = entities
            .iter()
            .map(|id| {
                world
                    .get_component::<Transform>(*id)
                    .map(|x| *x)
                    .unwrap_or_default()
            })
            .flat_map(|transform| transform.matrix().to_cols_array())
            .collect::<Vec<f32>>();
        let transform_buffer = Static::new(
            &renderer.ctx,
            bytemuck::cast_slice::<f32, u8>(&transforms),
            BufferUsageFlags::STORAGE_BUFFER,
        )
        .unwrap();

        let materials = render_objects
            .iter()
            .map(|object| *assets.get_material(object.material).unwrap())
            .collect::<Vec<Material>>();
        let material_buffer = Static::new(
            &renderer.ctx,
            bytemuck::cast_slice::<Material, u8>(&materials),
            BufferUsageFlags::STORAGE_BUFFER,
        )
        .unwrap();

        let set = renderer
            .object_layout
            .alloc()
            .unwrap()
            .write_buffer(0, &transform_buffer)
            .write_buffer(1, &material_buffer)
            .finish();

        let (vertices, indices) = render_objects.iter().fold(
            (Vec::new(), Vec::new()),
            |(mut vertices, mut indices), object| {
                let mesh = assets.get_mesh(object.mesh).unwrap();
                vertices.extend_from_slice(&mesh.vertices);
                indices.extend_from_slice(&mesh.indices);
                (vertices, indices)
            },
        );

        let mut index_offset = 0;
        let mut vertex_offset = 0;

        let draws = render_objects.iter().flat_map(|object| {
            let mesh = assets.get_mesh(object.mesh).unwrap();
            let draw = [mesh.indices.len() as u32, 1, index_offset, vertex_offset, 0];
            index_offset += mesh.indices.len() as u32;
            vertex_offset += mesh.vertices.len() as u32;
            draw
        }).collect::<Vec<u32>>();
        let draw_buffer = Static::new(&renderer.ctx, bytemuck::cast_slice::<u32, u8>(&draws), BufferUsageFlags::INDIRECT_BUFFER).unwrap();

        let vertex_buffer = Static::new(
            &renderer.ctx,
            bytemuck::cast_slice::<Vertex, u8>(&vertices),
            BufferUsageFlags::VERTEX_BUFFER,
        )
        .unwrap();
        let index_buffer = Static::new(
            &renderer.ctx,
            bytemuck::cast_slice::<u32, u8>(&indices),
            BufferUsageFlags::INDEX_BUFFER,
        )
        .unwrap();

        let scene = world.get_mut::<Ui>().unwrap().paint(&world);
        let frame = if !scene.is_empty() {
            Some(
                renderer
                    .ui
                    .prepare(
                        &renderer.ctx,
                        &scene,
                        Vec2::new(size.width as f32, size.height as f32),
                    )
                    .unwrap(),
            )
        } else {
            None
        };

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
            .bind_descriptor_set(&camera_set, 0)
            .bind_descriptor_set(&set, 1)
            .bind_vertex_buffer(&vertex_buffer, 0)
            .bind_index_buffer(&index_buffer).draw_indexed_indirect(&draw_buffer, 0, draws.len() as u32 / 5, 20);

        let cmd = match frame {
            Some(frame) => renderer.ui.draw(frame, cmd),
            None => cmd.next_subpass(),
        };

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
