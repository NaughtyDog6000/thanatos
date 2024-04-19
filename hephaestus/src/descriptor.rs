use std::{any::Any, rc::Rc};

use ash::{
    prelude::VkResult,
    vk::{
        self, DescriptorBufferInfo, DescriptorImageInfo, DescriptorPoolCreateFlags, DescriptorPoolCreateInfo, DescriptorPoolSize, DescriptorSetAllocateInfo, DescriptorSetLayoutBinding, DescriptorSetLayoutCreateInfo, DescriptorType, ImageLayout, ShaderStageFlags, WriteDescriptorSet
    },
};

use crate::{buffer, image::{ImageView, Sampler}, Context, Device};

#[derive(Clone)]
pub struct Layout {
    device: Rc<Device>,
    pub layout: vk::DescriptorSetLayout,
    pub pool: vk::DescriptorPool,
    pub bindings: Vec<DescriptorType>,
}

pub struct Set {
    layout: Rc<Layout>,
    pub handle: vk::DescriptorSet,
    resources: Vec<Rc<dyn Any>>,
}

impl Layout {
    pub fn new(ctx: &Context, bindings: &[DescriptorType], capacity: usize) -> VkResult<Rc<Self>> {
        let binding_infos = bindings
            .iter()
            .enumerate()
            .map(|(i, ty)| {
                DescriptorSetLayoutBinding::builder()
                    .binding(i as u32)
                    .descriptor_type(*ty)
                    .descriptor_count(1)
                    .stage_flags(ShaderStageFlags::ALL)
                    .build()
            })
            .collect::<Vec<_>>();
        let create_info = DescriptorSetLayoutCreateInfo::builder().bindings(&binding_infos);
        let layout = unsafe {
            ctx.device
                .create_descriptor_set_layout(&create_info, None)?
        };

        let pool_sizes = bindings
            .iter()
            .map(|ty| {
                DescriptorPoolSize::builder()
                    .ty(*ty)
                    .descriptor_count(capacity as u32)
                    .build()
            })
            .collect::<Vec<_>>();

        let create_info = DescriptorPoolCreateInfo::builder()
            .flags(DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET)
            .pool_sizes(&pool_sizes)
            .max_sets(capacity as u32);
        let pool = unsafe { ctx.device.create_descriptor_pool(&create_info, None)? };

        Ok(Rc::new(Self {
            device: ctx.device.clone(),
            layout,
            pool,
            bindings: bindings.to_vec(),
        }))
    }

    pub fn alloc(self: &Rc<Self>) -> VkResult<Set> {
        let set_layouts = [self.layout];
        let alloc_info = DescriptorSetAllocateInfo::builder()
            .descriptor_pool(self.pool)
            .set_layouts(&set_layouts);
        let handle = unsafe { self.device.allocate_descriptor_sets(&alloc_info)?[0] };
        Ok(Set {
            handle,
            layout: self.clone(),
            resources: Vec::new(),
        })
    }
}

impl Drop for Layout {
    fn drop(&mut self) {
        unsafe { self.device.destroy_descriptor_pool(self.pool, None) }
        unsafe { self.device.destroy_descriptor_set_layout(self.layout, None) }
    }
}

impl Set {
    pub fn write_buffer<T: buffer::Buffer + 'static>(mut self, binding: usize, buffer: &Rc<T>) -> Self {
        let buffer_info = DescriptorBufferInfo {
            buffer: buffer.buffer(),
            offset: 0,
            range: buffer.size() as u64,
        };
        let buffer_infos = [buffer_info];

        let write_info = WriteDescriptorSet::builder()
            .dst_set(self.handle)
            .dst_binding(binding as u32)
            .dst_array_element(0)
            .descriptor_type(self.layout.bindings[binding])
            .buffer_info(&buffer_infos);
        unsafe {
            self.layout
                .device
                .update_descriptor_sets(&[*write_info], &[])
        }

        self.resources.push(buffer.clone());
        self
    }

    pub fn write_image(mut self, binding: usize, view: &Rc<ImageView>, sampler: &Rc<Sampler>, layout: ImageLayout) -> Self {
        let image_info = DescriptorImageInfo {
            image_layout: layout,
            image_view: view.handle,
            sampler: sampler.handle
        };  

        let image_infos = [image_info];

        let write_info = WriteDescriptorSet::builder()
            .dst_set(self.handle)
            .dst_binding(binding as u32)
            .dst_array_element(0)
            .descriptor_type(self.layout.bindings[binding])
            .image_info(&image_infos);
        unsafe {
            self.layout
                .device
                .update_descriptor_sets(&[*write_info], &[])
        }

        self.resources.push(view.clone());
        self.resources.push(sampler.clone());
        self
    }

    pub fn finish(self) -> Rc<Self> {
        Rc::new(self)
    }
}

impl Drop for Set {
    fn drop(&mut self) {
        unsafe {
            self.layout
                .device
                .free_descriptor_sets(self.layout.pool, &[self.handle])
                .unwrap()
        }
    }
}
