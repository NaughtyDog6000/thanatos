use std::rc::Rc;

use ash::{
    prelude::VkResult,
    vk::{
        self, BorderColor, CompareOp, ComponentMapping, DeviceMemory, Extent2D, Extent3D, Filter, Format, ImageAspectFlags, ImageCreateInfo, ImageSubresourceRange, ImageTiling, ImageType, ImageUsageFlags, ImageViewCreateInfo, ImageViewType, MemoryAllocateInfo, MemoryPropertyFlags, SampleCountFlags, SamplerAddressMode, SamplerCreateInfo, SamplerMipmapMode, SharingMode
    },
};

use crate::{buffer::find_memory_type, Context, Device};

pub struct Image {
    device: Rc<Device>,
    pub handle: vk::Image,
    pub memory: DeviceMemory,
}

impl Image {
    pub fn new(
        ctx: &Context,
        format: Format,
        extent: Extent2D,
        usage: ImageUsageFlags,
    ) -> VkResult<Self> {
        let create_info = ImageCreateInfo::builder()
            .image_type(ImageType::TYPE_2D)
            .format(format)
            .extent(Extent3D {
                width: extent.width,
                height: extent.height,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .samples(SampleCountFlags::TYPE_1)
            .tiling(ImageTiling::OPTIMAL)
            .usage(usage)
            .sharing_mode(SharingMode::EXCLUSIVE);
        let handle = unsafe { ctx.device.create_image(&create_info, None)? };

        let requirements = unsafe { ctx.device.get_image_memory_requirements(handle) };
        let type_index = find_memory_type(ctx, requirements, MemoryPropertyFlags::DEVICE_LOCAL)
            .expect("No memory types found");

        let alloc_info = MemoryAllocateInfo::builder()
            .allocation_size(requirements.size)
            .memory_type_index(type_index as u32);
        let memory = unsafe { ctx.device.allocate_memory(&alloc_info, None)? };
        unsafe { ctx.device.bind_image_memory(handle, memory, 0)? };

        Ok(Self { device: ctx.device.clone(), handle, memory })
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        unsafe { self.device.destroy_image(self.handle, None) }
        unsafe { self.device.free_memory(self.memory, None) }
    }
}

pub struct ImageView {
    device: Rc<Device>,
    pub handle: vk::ImageView,
    pub extent: Extent2D,
}

impl ImageView {
    pub fn new(
        device: &Rc<Device>,
        image: vk::Image,
        format: Format,
        aspect: ImageAspectFlags,
        extent: Extent2D,
    ) -> VkResult<Self> {
        let create_info = ImageViewCreateInfo::builder()
            .image(image)
            .view_type(ImageViewType::TYPE_2D)
            .format(format)
            .components(ComponentMapping::default())
            .subresource_range(
                ImageSubresourceRange::builder()
                    .aspect_mask(aspect)
                    .base_mip_level(0)
                    .level_count(1)
                    .base_array_layer(0)
                    .layer_count(1)
                    .build(),
            );
        let handle = unsafe { device.create_image_view(&create_info, None)? };
        Ok(Self { device: device.clone(), handle, extent })
    }
}

impl Drop for ImageView {
    fn drop(&mut self) {
        println!("Dropping!");
        unsafe { self.device.destroy_image_view(self.handle, None) };
    }
}

pub struct Sampler {
    device: Rc<Device>,
    pub handle: vk::Sampler
}

impl Sampler {
    pub fn new(device: &Rc<Device>) -> VkResult<Self> {
        let create_info = SamplerCreateInfo::builder()
            .mag_filter(Filter::LINEAR)
            .min_filter(Filter::LINEAR)
            .address_mode_u(SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_v(SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_w(SamplerAddressMode::CLAMP_TO_EDGE)
            .anisotropy_enable(false)
            .border_color(BorderColor::INT_OPAQUE_BLACK)
            .unnormalized_coordinates(true)
            .compare_enable(false)
            .compare_op(CompareOp::ALWAYS)
            .mipmap_mode(SamplerMipmapMode::NEAREST)
            .mip_lod_bias(0.0)
            .min_lod(0.0)
            .max_lod(0.0);

        let handle = unsafe { device.create_sampler(&create_info, None)? };
        Ok(Self { device: device.clone(), handle })
    }
}

impl Drop for Sampler {
    fn drop(&mut self) {
        unsafe { self.device.destroy_sampler(self.handle, None) };
    }
}
