use std::{any::Any, rc::Rc};

use ash::{
    prelude::VkResult,
    vk::{
        self, BufferCopy, ClearValue, CommandBufferAllocateInfo, CommandBufferBeginInfo,
        CommandBufferLevel, CommandPoolCreateInfo, Extent2D, IndexType, Offset2D,
        PipelineBindPoint, PipelineLayout, Rect2D, RenderPassBeginInfo, SubpassContents, Viewport,
    },
};

use crate::{
    buffer, descriptor,
    pipeline::{Framebuffer, Graphics, RenderPass},
    Device, Queue,
};

pub struct Region {
    pub from_offset: usize,
    pub to_offset: usize,
    pub size: usize,
}

pub struct Buffer {
    device: Rc<Device>,
    pool: Rc<Pool>,
    pub handle: vk::CommandBuffer,
    resources: Vec<Rc<dyn Any>>,
}

impl Buffer {
    pub fn begin<'a>(self) -> VkResult<Recorder<'a>> {
        let begin_info = CommandBufferBeginInfo::default();
        unsafe { self.device.begin_command_buffer(self.handle, &begin_info)? };
        Ok(Recorder {
            buffer: self,
            pipeline: None,
        })
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            self.device
                .free_command_buffers(self.pool.handle, &[self.handle])
        }
    }
}

pub enum Pipeline<'a> {
    Graphics(&'a Graphics),
}

impl Pipeline<'_> {
    pub fn bind_point(&self) -> PipelineBindPoint {
        match self {
            Self::Graphics(_) => PipelineBindPoint::GRAPHICS,
        }
    }

    pub fn layout(&self) -> PipelineLayout {
        match self {
            Self::Graphics(pipeline) => pipeline.layout,
        }
    }
}

pub struct Recorder<'a> {
    buffer: Buffer,
    pipeline: Option<Pipeline<'a>>,
}

impl<'a> Recorder<'a> {
    pub fn end(self) -> VkResult<Rc<Buffer>> {
        unsafe { self.buffer.device.end_command_buffer(self.buffer.handle)? };
        Ok(Rc::new(self.buffer))
    }

    pub fn begin_render_pass(
        self,
        render_pass: &RenderPass,
        framebuffer: &Framebuffer,
        clear_values: &[ClearValue],
    ) -> Self {
        let render_pass_begin = RenderPassBeginInfo::builder()
            .render_pass(render_pass.handle)
            .framebuffer(framebuffer.handle)
            .render_area(Rect2D {
                offset: Offset2D { x: 0, y: 0 },
                extent: framebuffer.extent,
            })
            .clear_values(clear_values);
        unsafe {
            self.buffer.device.cmd_begin_render_pass(
                self.buffer.handle,
                &render_pass_begin,
                SubpassContents::INLINE,
            )
        };
        self
    }

    pub fn end_render_pass(self) -> Self {
        unsafe { self.buffer.device.cmd_end_render_pass(self.buffer.handle) };
        self
    }

    pub fn bind_graphics_pipeline(mut self, pipeline: &'a Graphics) -> Self {
        self.pipeline = Some(Pipeline::Graphics(pipeline));
        unsafe {
            self.buffer.device.cmd_bind_pipeline(
                self.buffer.handle,
                PipelineBindPoint::GRAPHICS,
                pipeline.handle,
            )
        };
        self
    }

    pub fn draw(
        self,
        vertices: u32,
        instances: u32,
        first_vertex: u32,
        first_instance: u32,
    ) -> Self {
        unsafe {
            self.buffer.device.cmd_draw(
                self.buffer.handle,
                vertices,
                instances,
                first_vertex,
                first_instance,
            )
        };
        self
    }

    pub fn draw_indexed(
        self,
        indices: u32,
        instances: u32,
        first_index: u32,
        vertex_offset: i32,
        first_instance: u32,
    ) -> Self {
        unsafe {
            self.buffer.device.cmd_draw_indexed(
                self.buffer.handle,
                indices,
                instances,
                first_index,
                vertex_offset,
                first_instance,
            )
        }
        self
    }

    pub fn set_viewport(self, width: u32, height: u32) -> Self {
        let viewport = Viewport::builder()
            .x(0.0)
            .y(0.0)
            .width(width as f32)
            .height(height as f32)
            .min_depth(0.0)
            .max_depth(1.0)
            .build();
        let viewports = [viewport];
        unsafe {
            self.buffer
                .device
                .cmd_set_viewport(self.buffer.handle, 0, &viewports)
        }
        self
    }

    pub fn set_scissor(self, width: u32, height: u32) -> Self {
        let scissor = Rect2D::builder()
            .offset(Offset2D { x: 0, y: 0 })
            .extent(Extent2D { width, height })
            .build();
        let scissors = [scissor];
        unsafe {
            self.buffer
                .device
                .cmd_set_scissor(self.buffer.handle, 0, &scissors)
        }
        self
    }

    pub fn bind_vertex_buffer<T: buffer::Buffer + 'static>(
        mut self,
        buffer: &Rc<T>,
        binding: u32,
    ) -> Self {
        unsafe {
            self.buffer.device.cmd_bind_vertex_buffers(
                self.buffer.handle,
                binding,
                &[buffer.buffer()],
                &[0],
            )
        }
        self.buffer.resources.push(buffer.clone());
        self
    }

    pub fn bind_index_buffer<T: buffer::Buffer + 'static>(mut self, buffer: &Rc<T>) -> Self {
        unsafe {
            self.buffer.device.cmd_bind_index_buffer(
                self.buffer.handle,
                buffer.buffer(),
                0,
                IndexType::UINT32,
            )
        }
        self.buffer.resources.push(buffer.clone());
        self
    }

    pub fn bind_descriptor_set(mut self, set: &Rc<descriptor::Set>, index: usize) -> Self {
        let pipeline = self.pipeline.as_ref().expect("No pipeline bound");
        unsafe {
            self.buffer.device.cmd_bind_descriptor_sets(
                self.buffer.handle,
                pipeline.bind_point(),
                pipeline.layout(),
                index as u32,
                &[set.handle],
                &[],
            )
        }
        self.buffer.resources.push(set.clone());
        self
    }

    pub fn copy_buffer<A: buffer::Buffer, B: buffer::Buffer>(
        self,
        from: &A,
        to: &B,
        region: Region,
    ) -> Self {
        let region = BufferCopy::builder()
            .src_offset(region.from_offset as u64)
            .dst_offset(region.to_offset as u64)
            .size(region.size as u64);
        unsafe {
            self.buffer.device.cmd_copy_buffer(
                self.buffer.handle,
                from.buffer(),
                to.buffer(),
                &[*region],
            )
        }
        self
    }

    pub fn next_subpass(self) -> Self {
        unsafe {
            self.buffer
                .device
                .cmd_next_subpass(self.buffer.handle, SubpassContents::INLINE)
        }
        self
    }
}

pub struct Pool {
    device: Rc<Device>,
    pub handle: vk::CommandPool,
}

impl Pool {
    pub fn new(device: &Rc<Device>, queue: &Queue) -> VkResult<Rc<Self>> {
        let create_info = CommandPoolCreateInfo::builder().queue_family_index(queue.index);
        let handle = unsafe { device.create_command_pool(&create_info, None)? };
        Ok(Rc::new(Self {
            handle,
            device: device.clone(),
        }))
    }

    pub fn alloc(self: &Rc<Self>) -> VkResult<Buffer> {
        let alloc_info = CommandBufferAllocateInfo::builder()
            .command_pool(self.handle)
            .level(CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);
        let handles = unsafe { self.device.allocate_command_buffers(&alloc_info)? };
        let handle = *handles.first().unwrap();
        Ok(Buffer {
            resources: Vec::new(),
            handle,
            pool: self.clone(),
            device: self.device.clone(),
        })
    }
}

impl Drop for Pool {
    fn drop(&mut self) {
        unsafe { self.device.destroy_command_pool(self.handle, None) }
    }
}
