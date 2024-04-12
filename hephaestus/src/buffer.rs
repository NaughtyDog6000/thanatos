use core::slice;
use std::{ffi::c_void, rc::Rc};

use ash::{
    prelude::VkResult,
    vk::{
        self, BufferCreateInfo, BufferUsageFlags, MemoryAllocateInfo, MemoryMapFlags,
        MemoryPropertyFlags, MemoryRequirements, SharingMode,
    },
};

use crate::{
    command::Region,
    task::{Fence, SubmitInfo, Task},
    Context, Device,
};

pub trait Buffer {
    fn buffer(&self) -> vk::Buffer;
    fn memory(&self) -> vk::DeviceMemory;
    fn size(&self) -> usize;
}

impl<T: Buffer> Buffer for Rc<T> {
    fn size(&self) -> usize {
        T::size(self)
    }

    fn memory(&self) -> vk::DeviceMemory {
        T::memory(self)
    }

    fn buffer(&self) -> vk::Buffer {
        T::buffer(self)
    }
}

pub(crate) fn find_memory_type(
    ctx: &Context,
    requirements: MemoryRequirements,
    wanted: MemoryPropertyFlags,
) -> Option<usize> {
    let properties = unsafe {
        ctx.instance
            .get_physical_device_memory_properties(ctx.device.physical.handle)
    };

    properties
        .memory_types
        .iter()
        .enumerate()
        .position(|(i, ty)| {
            (requirements.memory_type_bits & (1 << i)) != 0 && ty.property_flags.contains(wanted)
        })
}

pub struct Dynamic {
    device: Rc<Device>,
    pub handle: vk::Buffer,
    pub memory: vk::DeviceMemory,
    pub size: usize,
}

impl Dynamic {
    pub fn new(ctx: &Context, size: usize, usage: BufferUsageFlags) -> VkResult<Rc<Self>> {
        let create_info = BufferCreateInfo::builder()
            .size(size as u64)
            .usage(usage)
            .sharing_mode(SharingMode::EXCLUSIVE);
        let handle = unsafe { ctx.device.create_buffer(&create_info, None)? };

        let requirements = unsafe { ctx.device.get_buffer_memory_requirements(handle) };

        let type_index = find_memory_type(
            ctx,
            requirements,
            MemoryPropertyFlags::HOST_VISIBLE | MemoryPropertyFlags::HOST_COHERENT,
        )
        .expect("No suitable memory types");

        let alloc_info = MemoryAllocateInfo::builder()
            .allocation_size(requirements.size)
            .memory_type_index(type_index as u32);
        let memory = unsafe { ctx.device.allocate_memory(&alloc_info, None)? };
        unsafe { ctx.device.bind_buffer_memory(handle, memory, 0)? };

        Ok(Rc::new(Self {
            device: ctx.device.clone(),
            handle,
            memory,
            size,
        }))
    }

    pub fn write(&self, data: &[u8]) -> VkResult<()> {
        let memory: *mut c_void = unsafe {
            self.device.map_memory(self.memory, 0, data.len() as u64, MemoryMapFlags::default())?
        };
        let memory: *mut u8 = memory.cast();
        unsafe { slice::from_raw_parts_mut(memory, data.len()).copy_from_slice(data) };
        unsafe { self.device.unmap_memory(self.memory) };

        Ok(())
    }
}

impl Drop for Dynamic {
    fn drop(&mut self) {
        unsafe { self.device.destroy_buffer(self.handle, None) }
        unsafe { self.device.free_memory(self.memory, None) }
    }
}

impl Buffer for Dynamic {
    fn buffer(&self) -> vk::Buffer {
        self.handle
    }

    fn memory(&self) -> vk::DeviceMemory {
        self.memory
    }

    fn size(&self) -> usize {
        self.size
    }
}

pub struct Static {
    device: Rc<Device>,
    pub handle: vk::Buffer,
    pub memory: vk::DeviceMemory,
    pub size: usize,
}

impl Static {
    pub fn new(ctx: &Context, data: &[u8], usage: BufferUsageFlags) -> VkResult<Rc<Self>> {
        let size = data.len();
        let create_info = BufferCreateInfo::builder()
            .size(size as u64)
            .usage(usage | BufferUsageFlags::TRANSFER_DST)
            .sharing_mode(SharingMode::EXCLUSIVE);
        let handle = unsafe { ctx.device.create_buffer(&create_info, None)? };

        let requirements = unsafe { ctx.device.get_buffer_memory_requirements(handle) };
        let type_index = find_memory_type(ctx, requirements, MemoryPropertyFlags::DEVICE_LOCAL)
            .expect("No suitable memory types");

        let alloc_info = MemoryAllocateInfo::builder()
            .allocation_size(requirements.size)
            .memory_type_index(type_index as u32);
        let memory = unsafe { ctx.device.allocate_memory(&alloc_info, None)? };
        unsafe { ctx.device.bind_buffer_memory(handle, memory, 0)? };

        let staging = Dynamic::new(ctx, size, BufferUsageFlags::TRANSFER_SRC)?;
        staging.write(data)?;

        let buffer = Self {
            device: ctx.device.clone(),
            handle,
            memory,
            size,
        };

        let cmd = ctx
            .command_pool
            .alloc()?
            .begin()?
            .copy_buffer(
                &staging,
                &buffer,
                Region {
                    from_offset: 0,
                    to_offset: 0,
                    size,
                },
            )
            .end()?;

        let mut task = Task::new();
        let fence = Fence::new(&ctx.device)?;
        task.submit(SubmitInfo {
            cmd: &cmd,
            fence: fence.clone(),
            device: &ctx.device,
            queue: &ctx.device.queues.graphics,
            wait: &[],
            signal: &[],
        })?;
        fence.wait()?;

        Ok(Rc::new(buffer))
    }
}

impl Drop for Static {
    fn drop(&mut self) {
        unsafe { self.device.destroy_buffer(self.handle, None) }
        unsafe { self.device.free_memory(self.memory, None) }
    }
}

impl Buffer for Static {
    fn buffer(&self) -> vk::Buffer {
        self.handle
    }

    fn memory(&self) -> vk::DeviceMemory {
        self.memory
    }

    fn size(&self) -> usize {
        self.size
    }
}
