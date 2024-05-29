use std::rc::Rc;

use ash::{
    prelude::VkResult,
    vk::{self, FenceCreateInfo, PipelineStageFlags, PresentInfoKHR, SemaphoreCreateInfo},
};

use crate::{command, Device, Queue, Swapchain};

#[derive(Clone)]
pub struct Fence {
    device: Rc<Device>,
    pub handle: vk::Fence,
}

impl Fence {
    pub fn new(device: &Rc<Device>) -> VkResult<Rc<Self>> {
        let create_info = FenceCreateInfo::default();
        let handle = unsafe { device.create_fence(&create_info, None)? };
        Ok(Rc::new(Self { device: device.clone(), handle }))
    }

    pub fn wait(&self) -> VkResult<()> {
        let fences = [self.handle];
        unsafe { self.device.wait_for_fences(&fences, true, u64::MAX) }
    }

    pub fn reset(&self) -> VkResult<()> {
        let fences = [self.handle];
        unsafe { self.device.reset_fences(&fences) }
    }
}

impl Drop for Fence {
    fn drop(&mut self) {
        unsafe { self.device.destroy_fence(self.handle, None) }
    }
}

#[derive(Clone)]
pub struct Semaphore {
    device: Rc<Device>,
    pub handle: vk::Semaphore,
}

impl Semaphore {
    pub fn new(device: &Rc<Device>) -> VkResult<Rc<Self>> {
        let create_info = SemaphoreCreateInfo::default();
        let handle = unsafe { device.create_semaphore(&create_info, None)? };
        Ok(Rc::new(Self { device: device.clone(), handle }))
    }
}

impl Drop for Semaphore {
    fn drop(&mut self) {
        unsafe { self.device.destroy_semaphore(self.handle, None) }
    }
}

#[derive(Default)]
pub struct Task {
    semaphores: Vec<Rc<Semaphore>>,
    fences: Vec<Rc<Fence>>,
    cmds: Vec<Rc<command::Buffer>>
}

pub struct SubmitInfo<'a> {
    pub device: &'a Device,
    pub queue: &'a Queue,
    pub cmd: &'a Rc<command::Buffer>,
    pub wait: &'a [(Rc<Semaphore>, PipelineStageFlags)],
    pub signal: &'a [Rc<Semaphore>],
    pub fence: Rc<Fence>,
}

impl Task {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn run(device: &Rc<Device>, queue: &Queue, cmd: &Rc<command::Buffer>) -> VkResult<()> {
        let mut task = Task::new();
        let fence = Fence::new(device)?;
        task.submit(SubmitInfo {
            device,
            queue,
            cmd,
            wait: &[],
            signal: &[],
            fence: fence.clone(),
        })?;
        fence.wait()?;
        Ok(())
    }

    pub fn acquire_next_image(
        &mut self,
        device: &Device,
        swapchain: &Swapchain,
        signal: Rc<Semaphore>,
    ) -> VkResult<(u32, bool)> {
        let result = unsafe {
            device.extensions.swapchain.acquire_next_image(
                swapchain.handle,
                u64::MAX,
                signal.handle,
                vk::Fence::null(),
            )
        };
        self.semaphores.push(signal);
        match result {
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => Ok((0, true)),
            x => x,
        }
    }

    pub fn submit(&mut self, info: SubmitInfo) -> VkResult<()> {
        let stages = info
            .wait
            .iter()
            .map(|(_, stage)| *stage)
            .collect::<Vec<_>>();
        let wait_semaphores = info
            .wait
            .iter()
            .map(|(semaphore, _)| semaphore.handle)
            .collect::<Vec<_>>();
        let buffers = [info.cmd.handle];
        let signal_semaphores = info
            .signal
            .iter()
            .map(|semamphore| semamphore.handle)
            .collect::<Vec<_>>();

        let submit_info = vk::SubmitInfo::builder()
            .wait_dst_stage_mask(&stages)
            .wait_semaphores(&wait_semaphores)
            .command_buffers(&buffers)
            .signal_semaphores(&signal_semaphores);

        unsafe {
            info.device
                .queue_submit(info.queue.handle, &[*submit_info], info.fence.handle)?
        };

        self.semaphores.extend_from_slice(
            &info
                .wait
                .iter()
                .map(|(semaphore, _)| semaphore.clone())
                .collect::<Vec<_>>(),
        );
        self.semaphores
            .extend_from_slice(info.signal);
        self.fences.push(info.fence);
        self.cmds.push(info.cmd.clone());

        Ok(())
    }

    pub fn present(
        &mut self,
        device: &Device,
        swapchain: &Swapchain,
        image_index: u32,
        wait: &[Rc<Semaphore>],
    ) -> VkResult<bool> {
        let wait_semaphores = wait.iter().map(|wait| wait.handle).collect::<Vec<_>>();
        let swapchains = [swapchain.handle];
        let image_indices = [image_index];
        let present_info = PresentInfoKHR::builder()
            .wait_semaphores(&wait_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_indices);

        let result = unsafe {
            device
                .extensions
                .swapchain
                .queue_present(device.queues.present.handle, &present_info)
        };

        self.semaphores.extend_from_slice(wait);

        match result {
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => Ok(true),
            Err(vk::Result::SUBOPTIMAL_KHR) => Ok(true),
            x => x,
        }
    }
}
