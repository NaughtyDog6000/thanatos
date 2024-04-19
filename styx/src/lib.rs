use std::{mem::size_of, rc::Rc};

use anyhow::Result;
use glam::{Vec2, Vec3, Vec4};
use hephaestus::{
    buffer::Static,
    command, descriptor,
    pipeline::{Graphics, RenderPass, ShaderModule, Viewport},
    vertex::{self, AttributeType},
    BufferUsageFlags, Context, DescriptorType,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Constraint<T> {
    pub min: T,
    pub max: T,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Area {
    pub origin: Vec2,
    pub size: Vec2,
}

impl Area {
    pub fn points(&self) -> [Vec2; 4] {
        [
            self.origin,
            self.origin + Vec2::X * self.size.x,
            self.origin + self.size,
            self.origin + Vec2::Y * self.size.y,
        ]
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rectangle {
    area: Area,
    radius: f32,
    colour: Vec4,
}

#[derive(Default)]
pub struct Layer {
    rectangles: Vec<Rectangle>,
}

pub struct Scene {
    layers: Vec<Layer>,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            layers: vec![Layer::default()],
        }
    }

    pub fn rectangle(&mut self, rectangle: Rectangle) {
        self.layers.last_mut().unwrap().rectangles.push(rectangle)
    }

    pub fn vertices(&self) -> (Vec<Vec3>, Vec<u32>) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for (depth, layer) in self.layers.iter().enumerate() {
            for rectangle in &layer.rectangles {
                indices.append(
                    &mut [0, 1, 2, 2, 3, 0]
                        .into_iter()
                        .map(|x| x + vertices.len() as u32)
                        .collect(),
                );

                vertices.extend_from_slice(
                    &rectangle
                        .area
                        .points()
                        .into_iter()
                        .map(|point| Vec3::new(point.x, point.y, 0.0))
                        .collect::<Vec<_>>(),
                );
            }
        }

        (vertices, indices)
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: Vec3,
}

impl Vertex {
    pub fn info() -> vertex::Info {
        vertex::Info::new(size_of::<Self>()).attribute(AttributeType::Vec3, 0)
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Default, bytemuck::Pod, bytemuck::Zeroable)]
struct RectangleData {
    pub colour: Vec4,
    pub area: Vec4,
    pub radius: Vec4,
}

pub struct Renderer {
    pipeline: Graphics,
    layout: Rc<descriptor::Layout>,
}

pub struct Frame {
    vertex_buffer: Rc<Static>,
    index_buffer: Rc<Static>,
    num_indices: u32,
    set: Rc<descriptor::Set>,
}

impl Renderer {
    pub fn new(ctx: &Context, render_pass: &RenderPass, subpass: usize) -> Result<Self> {
        let ui_vertex =
            ShaderModule::new(&ctx.device, &std::fs::read("assets/shaders/ui.vert.spv")?)?;

        let ui_fragment =
            ShaderModule::new(&ctx.device, &std::fs::read("assets/shaders/ui.frag.spv")?)?;

        let layout = descriptor::Layout::new(
            &ctx,
            &[
                DescriptorType::STORAGE_BUFFER,
                DescriptorType::UNIFORM_BUFFER,
            ],
            1000,
        )?;

        let pipeline = Graphics::builder()
            .vertex(&ui_vertex)
            .vertex_info(Vertex::info())
            .fragment(&ui_fragment)
            .render_pass(&render_pass)
            .subpass(subpass as u32)
            .viewport(Viewport::Dynamic)
            .layouts(vec![&layout])
            .build(&ctx.device)?;

        Ok(Self { pipeline, layout })
    }

    pub fn prepare(&self, ctx: &Context, scene: &Scene, viewport: Vec2) -> Result<Frame> {
        let (vertices, indices) = scene.vertices();
        let num_indices = indices.len() as u32;
        let vertex_buffer = Static::new(
            ctx,
            bytemuck::cast_slice::<Vec3, u8>(&vertices),
            BufferUsageFlags::VERTEX_BUFFER,
        )?;
        let index_buffer = Static::new(
            ctx,
            bytemuck::cast_slice::<u32, u8>(&indices),
            BufferUsageFlags::INDEX_BUFFER,
        )?;
        let rectangles = scene
            .layers
            .iter()
            .flat_map(|layer| &layer.rectangles)
            .map(|rectangle| RectangleData {
                colour: rectangle.colour,
                area: Vec4::new(
                    rectangle.area.origin.x,
                    rectangle.area.origin.y,
                    rectangle.area.size.x,
                    rectangle.area.size.y,
                ),
                radius: Vec4::new(rectangle.radius, 0.0, 0.0, 0.0),
            })
            .collect::<Vec<_>>();
        let rectangle_buffer = Static::new(
            ctx,
            bytemuck::cast_slice::<RectangleData, u8>(&rectangles),
            BufferUsageFlags::STORAGE_BUFFER,
        )?;

        let viewport_buffer = Static::new(
            ctx,
            bytemuck::cast_slice::<Vec2, u8>(&[viewport]),
            BufferUsageFlags::UNIFORM_BUFFER,
        )?;

        let set = self
            .layout
            .alloc()?
            .write_buffer(0, &rectangle_buffer)
            .write_buffer(1, &viewport_buffer)
            .finish();
        Ok(Frame {
            vertex_buffer,
            index_buffer,
            num_indices,
            set,
        })
    }

    pub fn draw<'a>(&'a self, frame: Frame, cmd: command::Recorder<'a>) -> command::Recorder<'a> {
        cmd.next_subpass()
            .bind_graphics_pipeline(&self.pipeline)
            .bind_vertex_buffer(&frame.vertex_buffer, 0)
            .bind_index_buffer(&frame.index_buffer)
            .bind_descriptor_set(&frame.set, 0)
            .draw_indexed(frame.num_indices, 1, 0, 0, 0)
    }
}

pub trait Element {
    fn layout(&mut self, constraint: Constraint<Vec2>) -> Vec2;
    fn paint(&mut self, area: Area, scene: &mut Scene);
}

pub struct Box {
    pub colour: Vec4,
}

impl Element for Box {
    fn layout(&mut self, constraint: Constraint<Vec2>) -> Vec2 {
        constraint.max
    }

    fn paint(&mut self, area: Area, scene: &mut Scene) {
        scene.rectangle(Rectangle {
            area,
            colour: self.colour,
            radius: 16.0,
        })
    }
}
