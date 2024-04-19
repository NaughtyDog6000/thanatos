pub mod buffer;
pub mod command;
pub mod descriptor;
pub mod image;
pub mod pipeline;
pub mod task;
pub mod vertex;

use std::{
    collections::HashSet,
    ffi::{c_char, CStr, CString},
    ops::Deref,
    rc::Rc,
};

pub use ash::prelude::VkResult;
pub use ash::vk::{
    AccessFlags, BufferUsageFlags, ClearColorValue, ClearValue, DescriptorType, Extent2D, Extent3D,
    Format, ImageAspectFlags, ImageUsageFlags, MemoryPropertyFlags, Offset2D, Offset3D,
    PipelineStageFlags,
};
use ash::{
    vk::{
        self, ApplicationInfo, ColorSpaceKHR, CompositeAlphaFlagsKHR, DeviceCreateInfo,
        DeviceQueueCreateInfo, Image, InstanceCreateInfo, PhysicalDeviceFeatures,
        PhysicalDeviceProperties, PresentModeKHR, QueueFamilyProperties, QueueFlags, SharingMode,
        SurfaceCapabilitiesKHR, SurfaceFormatKHR, SwapchainCreateInfoKHR, SwapchainKHR,
    },
    Entry,
};

use image::ImageView;
use log::{error, warn};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};

pub struct InstanceExtensions {
    pub surface: ash::extensions::khr::Surface,
}

impl InstanceExtensions {
    pub fn new(entry: &Entry, instance: &ash::Instance) -> Self {
        let surface = ash::extensions::khr::Surface::new(entry, instance);

        Self { surface }
    }
}

pub struct Instance {
    pub inner: ash::Instance,
    pub extensions: InstanceExtensions,
}

impl Deref for Instance {
    type Target = ash::Instance;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

pub struct PhysicalDevice {
    pub handle: vk::PhysicalDevice,
    pub properties: PhysicalDeviceProperties,
    pub features: PhysicalDeviceFeatures,
    pub queue_families: Vec<QueueFamilyProperties>,
}

pub struct Surface {
    instance: Rc<Instance>,
    pub handle: vk::SurfaceKHR,
    pub extent: Extent2D,
    pub capabilities: SurfaceCapabilitiesKHR,
    pub formats: Vec<SurfaceFormatKHR>,
    pub present_modes: Vec<PresentModeKHR>,
}

impl Surface {
    pub fn new<T: HasRawDisplayHandle + HasRawWindowHandle>(
        entry: &Entry,
        instance: &Rc<Instance>,
        physical: &PhysicalDevice,
        window: T,
        extent: (u32, u32),
    ) -> VkResult<Self> {
        unsafe {
            let handle = ash_window::create_surface(
                entry,
                instance,
                window.raw_display_handle(),
                window.raw_window_handle(),
                None,
            )?;

            let capabilities = instance
                .extensions
                .surface
                .get_physical_device_surface_capabilities(physical.handle, handle)?;
            let formats = instance
                .extensions
                .surface
                .get_physical_device_surface_formats(physical.handle, handle)?;
            let present_modes = instance
                .extensions
                .surface
                .get_physical_device_surface_present_modes(physical.handle, handle)?;

            Ok(Surface {
                instance: instance.clone(),
                handle,
                capabilities,
                formats,
                present_modes,
                extent: Extent2D {
                    width: extent.0,
                    height: extent.1,
                },
            })
        }
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            self.instance
                .extensions
                .surface
                .destroy_surface(self.handle, None)
        }
    }
}

impl Instance {
    const EXTENSIONS: &'static [&'static CStr] = &[ash::extensions::khr::Surface::name()];

    const LAYERS: &'static [&'static CStr] = &[c"VK_LAYER_KHRONOS_validation"];

    pub fn new<T: HasRawDisplayHandle>(entry: &Entry, name: &CStr, window: T) -> VkResult<Self> {
        let app_info = ApplicationInfo::builder()
            .engine_name(name)
            .engine_version(vk::make_api_version(0, 1, 0, 0))
            .application_name(name)
            .application_version(vk::make_api_version(0, 1, 0, 0))
            .api_version(vk::API_VERSION_1_3);

        let available = entry.enumerate_instance_layer_properties()?;
        let layers = Self::LAYERS
            .iter()
            .filter(|wanted| {
                let found = available
                    .iter()
                    .any(|layer| unsafe { CStr::from_ptr(layer.layer_name.as_ptr()) } == **wanted);
                if !found {
                    warn!("Missing validation layer: {}", wanted.to_str().unwrap())
                }
                found
            })
            .map(|name| name.as_ptr() as *const c_char)
            .collect::<Vec<_>>();

        let available = entry.enumerate_instance_extension_properties(None)?;
        let presentation_extensions =
            ash_window::enumerate_required_extensions(window.raw_display_handle())?;
        println!(
            "{:?}",
            available
                .iter()
                .map(|extension| unsafe { CStr::from_ptr(extension.extension_name.as_ptr()) })
                .collect::<Vec<_>>()
        );
        let extensions = Self::EXTENSIONS
            .iter()
            .filter(|wanted| {
                let found = available
                    .iter()
                    .any(|extension| unsafe { CStr::from_ptr(extension.extension_name.as_ptr()) } == **wanted);
                if !found {
                    error!("Missing extension: {}", wanted.to_str().unwrap())
                }
                found
            })
            .map(|name| name.as_ptr() as *const c_char)
            .chain(presentation_extensions.iter().copied())
            .collect::<Vec<_>>();

        let create_info = InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_extension_names(&extensions)
            .enabled_layer_names(&layers);

        let inner = unsafe { entry.create_instance(&create_info, None)? };
        let extensions = InstanceExtensions::new(entry, &inner);
        Ok(Self { inner, extensions })
    }

    pub unsafe fn get_physical_device(&self) -> VkResult<PhysicalDevice> {
        let devices = self.enumerate_physical_devices()?;
        let handle = *devices.first().expect("No device found");
        let properties = self.get_physical_device_properties(handle);
        let features = self.get_physical_device_features(handle);
        let queue_families = self.get_physical_device_queue_family_properties(handle);
        Ok(PhysicalDevice {
            handle,
            properties,
            features,
            queue_families,
        })
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        unsafe { self.destroy_instance(None) }
    }
}

pub struct Queue {
    pub handle: vk::Queue,
    pub index: u32,
}

impl Queue {
    pub fn new(device: &ash::Device, index: u32) -> Self {
        let handle = unsafe { device.get_device_queue(index, 0) };
        Self { handle, index }
    }
}

pub struct Queues {
    pub graphics: Queue,
    pub present: Queue,
}

pub struct DeviceExtensions {
    pub swapchain: ash::extensions::khr::Swapchain,
}

pub struct Device {
    pub inner: ash::Device,
    pub extensions: DeviceExtensions,
    pub physical: PhysicalDevice,
    pub queues: Queues,
}

impl Deref for Device {
    type Target = ash::Device;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

pub struct Swapchain {
    device: Rc<Device>,
    pub handle: SwapchainKHR,
    pub images: Vec<Image>,
    pub views: Vec<Rc<ImageView>>,
    pub format: Format,
    pub extent: Extent2D,
}

impl Swapchain {
    pub fn new(device: &Rc<Device>, surface: &Surface) -> VkResult<Self> {
        let format = surface
            .formats
            .iter()
            .find(|format| {
                format.format == Format::B8G8R8A8_SRGB
                    && format.color_space == ColorSpaceKHR::SRGB_NONLINEAR
            })
            .unwrap_or_else(|| surface.formats.first().unwrap());

        let present_mode = surface
            .present_modes
            .iter()
            .copied()
            .find(|mode| *mode == PresentModeKHR::MAILBOX)
            .unwrap_or(PresentModeKHR::FIFO);

        let extent = if surface.capabilities.current_extent.width == u32::MAX {
            Extent2D {
                width: surface.extent.width.clamp(
                    surface.capabilities.min_image_extent.width,
                    surface.capabilities.max_image_extent.width,
                ),
                height: surface.extent.height.clamp(
                    surface.capabilities.min_image_extent.height,
                    surface.capabilities.max_image_extent.height,
                ),
            }
        } else {
            surface.capabilities.current_extent
        };

        let image_count = if surface.capabilities.max_image_count == 0 {
            surface.capabilities.min_image_count + 1
        } else {
            (surface.capabilities.min_image_count + 1).min(surface.capabilities.max_image_count)
        };

        let create_info = SwapchainCreateInfoKHR::builder()
            .surface(surface.handle)
            .min_image_count(image_count)
            .image_format(format.format)
            .image_color_space(format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(ImageUsageFlags::COLOR_ATTACHMENT)
            .pre_transform(surface.capabilities.current_transform)
            .composite_alpha(CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true);

        let indices = [device.queues.graphics.index, device.queues.present.index];
        let create_info = if device.queues.graphics.index == device.queues.present.index {
            create_info.image_sharing_mode(SharingMode::EXCLUSIVE)
        } else {
            create_info
                .image_sharing_mode(SharingMode::CONCURRENT)
                .queue_family_indices(&indices)
        };

        let handle = unsafe {
            device
                .extensions
                .swapchain
                .create_swapchain(&create_info, None)?
        };

        let images = unsafe { device.extensions.swapchain.get_swapchain_images(handle)? };
        let views = images
            .iter()
            .map(|image| {
                ImageView::new_from_handle(
                    device,
                    *image,
                    format.format,
                    ImageAspectFlags::COLOR,
                    extent,
                )
            })
            .collect::<VkResult<Vec<_>>>()?;

        Ok(Swapchain {
            device: device.clone(),
            handle,
            images,
            views,
            format: format.format,
            extent,
        })
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        unsafe {
            self.device
                .extensions
                .swapchain
                .destroy_swapchain(self.handle, None)
        };
    }
}

impl Device {
    const EXTENSIONS: &'static [&'static CStr] = &[ash::extensions::khr::Swapchain::name()];

    pub fn new(instance: &Instance, physical: PhysicalDevice, surface: &Surface) -> VkResult<Self> {
        let priorities = &[1.0];

        let graphics_index = physical
            .queue_families
            .iter()
            .position(|family| family.queue_flags.contains(QueueFlags::GRAPHICS))
            .expect("No graphics capable queue families") as u32;
        let present_index = physical
            .queue_families
            .iter()
            .enumerate()
            .position(|(i, _)| unsafe {
                instance
                    .extensions
                    .surface
                    .get_physical_device_surface_support(physical.handle, i as u32, surface.handle)
                    .unwrap()
            })
            .expect("No presentation capable queue families") as u32;

        let indices = HashSet::from([graphics_index, present_index]);
        let queue_create_infos = indices
            .into_iter()
            .map(|index| {
                DeviceQueueCreateInfo::builder()
                    .queue_family_index(index)
                    .queue_priorities(priorities)
                    .build()
            })
            .collect::<Vec<_>>();

        let available = unsafe { instance.enumerate_device_extension_properties(physical.handle)? };
        let extensions = Self::EXTENSIONS
            .iter()
            .filter(|wanted| {
                let found = available
                    .iter()
                    .any(|extension| unsafe { CStr::from_ptr(extension.extension_name.as_ptr()) } == **wanted);
                if !found {
                    error!("Missing extension: {}", wanted.to_str().unwrap())
                }
                found
            })
            .map(|name| name.as_ptr() as *const c_char)
            .collect::<Vec<_>>();

        let create_info = DeviceCreateInfo::builder()
            .enabled_extension_names(&extensions)
            .queue_create_infos(&queue_create_infos);

        let inner = unsafe { instance.create_device(physical.handle, &create_info, None)? };

        let queues = Queues {
            graphics: Queue::new(&inner, graphics_index),
            present: Queue::new(&inner, present_index),
        };

        let swapchain = ash::extensions::khr::Swapchain::new(instance, &inner);
        let extensions = DeviceExtensions { swapchain };

        Ok(Self {
            inner,
            extensions,
            physical,
            queues,
        })
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe { self.destroy_device(None) }
    }
}

pub struct Context {
    pub swapchain: Option<Swapchain>,
    pub command_pool: Rc<command::Pool>,
    pub device: Rc<Device>,
    pub surface: Surface,
    pub instance: Rc<Instance>,
    pub entry: Entry,
}

impl Context {
    pub fn new<T: HasRawWindowHandle + HasRawDisplayHandle>(
        name: &str,
        window: T,
        extent: (u32, u32),
    ) -> VkResult<Self> {
        let entry = unsafe { Entry::load() }.unwrap_or_else(|_| {
            println!("Failed to load vulkan dll, using linked vulkan");
            Entry::linked()
        });
        let name = CString::new(name).unwrap();
        let instance = Rc::new(Instance::new(&entry, &name, &window)?);
        let physical = unsafe { instance.get_physical_device()? };
        let surface = Surface::new(&entry, &instance, &physical, window, extent)?;
        let device = Rc::new(Device::new(&instance, physical, &surface)?);
        let swapchain = Some(Swapchain::new(&device, &surface)?);
        let command_pool = command::Pool::new(&device, &device.queues.graphics)?;

        Ok(Self {
            entry,
            instance,
            surface,
            device,
            swapchain,
            command_pool,
        })
    }

    fn refresh_surface(&mut self) -> VkResult<()> {
        unsafe {
            self.surface.capabilities = self
                .instance
                .extensions
                .surface
                .get_physical_device_surface_capabilities(
                    self.device.physical.handle,
                    self.surface.handle,
                )?;
            self.surface.formats = self
                .instance
                .extensions
                .surface
                .get_physical_device_surface_formats(
                    self.device.physical.handle,
                    self.surface.handle,
                )?;
            self.surface.present_modes = self
                .instance
                .extensions
                .surface
                .get_physical_device_surface_present_modes(
                    self.device.physical.handle,
                    self.surface.handle,
                )?;
            Ok(())
        }
    }

    pub fn recreate_swapchain(&mut self) -> VkResult<()> {
        self.refresh_surface()?;
        drop(self.swapchain.take());
        self.swapchain = Some(Swapchain::new(&self.device, &self.surface)?);
        Ok(())
    }
}
