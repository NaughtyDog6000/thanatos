use ash::{
    prelude::VkResult,
    vk::{
        self, AttachmentDescription, AttachmentLoadOp, AttachmentReference, AttachmentStoreOp,
        ColorComponentFlags, CullModeFlags, Extent2D, Format, FramebufferCreateInfo, FrontFace,
        GraphicsPipelineCreateInfo, Offset2D, Pipeline, PipelineCache,
        PipelineColorBlendAttachmentState, PipelineColorBlendStateCreateInfo,
        PipelineDepthStencilStateCreateInfo, PipelineDynamicStateCreateInfo,
        PipelineInputAssemblyStateCreateInfo, PipelineLayout, PipelineLayoutCreateInfo,
        PipelineMultisampleStateCreateInfo, PipelineRasterizationStateCreateInfo,
        PipelineShaderStageCreateInfo, PipelineVertexInputStateCreateInfo,
        PipelineViewportStateCreateInfo, PolygonMode, PrimitiveTopology, Rect2D,
        RenderPassCreateInfo, Result, SampleCountFlags, ShaderModuleCreateInfo, ShaderStageFlags,
        SubpassDescription, Viewport,
    },
};
use log::error;

pub use ash::vk::{ImageLayout, PipelineBindPoint};

use crate::{Device, ImageView};

pub struct ShaderModule {
    pub handle: vk::ShaderModule,
}

impl ShaderModule {
    pub fn new(device: &Device, code: &[u8]) -> VkResult<ShaderModule> {
        let code = bytemuck::cast_slice::<u8, u32>(code);
        let create_info = ShaderModuleCreateInfo::builder().code(code);
        let handle = unsafe { device.create_shader_module(&create_info, None)? };
        Ok(Self { handle })
    }

    pub fn destroy(self, device: &Device) {
        unsafe { device.destroy_shader_module(self.handle, None) };
    }
}

pub struct Framebuffer {
    pub handle: vk::Framebuffer,
    pub extent: Extent2D
}

impl Framebuffer {
    pub fn destroy(self, device: &Device) {
        unsafe { device.destroy_framebuffer(self.handle, None) };
    }
}

pub struct RenderPass {
    pub handle: vk::RenderPass,
}

impl RenderPass {
    pub fn builder() -> RenderPassBuilder {
        RenderPassBuilder::default()
    }

    pub fn get_framebuffer(
        &self,
        device: &Device,
        attachments: &[&ImageView],
    ) -> VkResult<Framebuffer> {
        let extent = attachments.first().unwrap().extent;
        if !attachments
            .iter()
            .all(|attachment| attachment.extent == extent)
        {
            error!("Inconsistent image view extents in framebuffer");
            return Err(Result::ERROR_UNKNOWN);
        }

        let attachments = attachments
            .iter()
            .map(|attachment| attachment.handle)
            .collect::<Vec<_>>();
        let create_info = FramebufferCreateInfo::builder()
            .render_pass(self.handle)
            .attachments(&attachments)
            .width(extent.width)
            .height(extent.height)
            .layers(1);

        let handle = unsafe { device.create_framebuffer(&create_info, None)? };
        Ok(Framebuffer { handle, extent })
    }

    pub fn destroy(self, device: &Device) {
        unsafe { device.destroy_render_pass(self.handle, None) }
    }
}

pub struct Subpass {
    bind_point: PipelineBindPoint,
    colour: Vec<AttachmentReference>,
}

impl Subpass {
    pub fn new(bind_point: PipelineBindPoint) -> Self {
        Self {
            bind_point,
            colour: Vec::new(),
        }
    }

    pub fn colour(mut self, attachment: AttachmentId, layout: ImageLayout) -> Self {
        self.colour.push(AttachmentReference {
            attachment: attachment.0,
            layout,
        });
        self
    }
}

#[derive(Default)]
pub struct RenderPassBuilder {
    attachments: Vec<AttachmentDescription>,
    subpasses: Vec<Subpass>,
}

#[derive(Clone, Copy)]
pub struct AttachmentId(u32);

impl RenderPassBuilder {
    pub fn attachment(
        &mut self,
        format: Format,
        initial_layout: ImageLayout,
        final_layout: ImageLayout,
    ) -> AttachmentId {
        let attachment = AttachmentDescription::builder()
            .format(format)
            .samples(SampleCountFlags::TYPE_1)
            .load_op(AttachmentLoadOp::CLEAR)
            .store_op(AttachmentStoreOp::STORE)
            .stencil_load_op(AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(AttachmentStoreOp::DONT_CARE)
            .initial_layout(initial_layout)
            .final_layout(final_layout)
            .build();
        self.attachments.push(attachment);
        AttachmentId(self.attachments.len() as u32 - 1)
    }

    pub fn subpass(&mut self, subpass: Subpass) {
        self.subpasses.push(subpass);
    }

    pub fn build(self, device: &Device) -> VkResult<RenderPass> {
        let subpasses = self
            .subpasses
            .iter()
            .map(|subpass| {
                SubpassDescription::builder()
                    .pipeline_bind_point(subpass.bind_point)
                    .color_attachments(&subpass.colour)
                    .build()
            })
            .collect::<Vec<_>>();
        let create_info = RenderPassCreateInfo::builder()
            .attachments(&self.attachments)
            .subpasses(&subpasses);
        let handle = unsafe { device.create_render_pass(&create_info, None)? };

        Ok(RenderPass { handle })
    }
}

pub struct Graphics {
    pub layout: PipelineLayout,
    pub handle: Pipeline,
}

impl Graphics {
    pub fn builder<'a>() -> GraphicsBuilder<'a> {
        GraphicsBuilder::default()
    }

    pub fn destroy(self, device: &Device) {
        unsafe { device.destroy_pipeline(self.handle, None) };
        unsafe { device.destroy_pipeline_layout(self.layout, None) };
    }
}

#[derive(Default)]
pub struct GraphicsBuilder<'a> {
    vertex: Option<&'a ShaderModule>,
    fragment: Option<&'a ShaderModule>,
    viewport: Option<(f32, f32)>,
    render_pass: Option<&'a RenderPass>,
    subpass: Option<u32>,
}

impl<'a> GraphicsBuilder<'a> {
    pub fn vertex(mut self, shader: &'a ShaderModule) -> Self {
        self.vertex = Some(shader);
        self
    }

    pub fn fragment(mut self, shader: &'a ShaderModule) -> Self {
        self.fragment = Some(shader);
        self
    }

    pub fn viewport(mut self, width: f32, height: f32) -> Self {
        self.viewport = Some((width, height));
        self
    }

    pub fn render_pass(mut self, render_pass: &'a RenderPass) -> Self {
        self.render_pass = Some(render_pass);
        self
    }

    pub fn subpass(mut self, subpass: u32) -> Self {
        self.subpass = Some(subpass);
        self
    }

    pub fn build(self, device: &Device) -> VkResult<Graphics> {
        let vertex_stage = PipelineShaderStageCreateInfo::builder()
            .stage(ShaderStageFlags::VERTEX)
            .module(self.vertex.expect("Missing vertex shader").handle)
            .name(c"main")
            .build();
        let fragment_stage = PipelineShaderStageCreateInfo::builder()
            .stage(ShaderStageFlags::FRAGMENT)
            .module(self.fragment.expect("Missing fragment shader").handle)
            .name(c"main")
            .build();
        let stages = [vertex_stage, fragment_stage];

        let dynamic_state = PipelineDynamicStateCreateInfo::default();
        let vertex_input = PipelineVertexInputStateCreateInfo::default();
        let input_assembly = PipelineInputAssemblyStateCreateInfo::builder()
            .topology(PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);

        let viewport = self.viewport.expect("Missing viewport");
        let scissor = Rect2D::builder()
            .offset(Offset2D::default())
            .extent(Extent2D {
                width: viewport.0 as u32,
                height: viewport.1 as u32,
            })
            .build();
        let viewport = Viewport::builder()
            .x(0.0)
            .y(0.0)
            .width(viewport.0)
            .height(viewport.1)
            .min_depth(0.0)
            .max_depth(1.0)
            .build();

        let viewports = [viewport];
        let scissors = [scissor];
        let viewport_state = PipelineViewportStateCreateInfo::builder()
            .viewports(&viewports)
            .scissors(&scissors);

        let raster = PipelineRasterizationStateCreateInfo::builder()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(CullModeFlags::BACK)
            .front_face(FrontFace::CLOCKWISE)
            .depth_bias_enable(false);

        let multisampling = PipelineMultisampleStateCreateInfo::builder()
            .sample_shading_enable(false)
            .rasterization_samples(SampleCountFlags::TYPE_1);

        let depth_stencil = PipelineDepthStencilStateCreateInfo::default();

        let attachment = PipelineColorBlendAttachmentState::builder()
            .color_write_mask(ColorComponentFlags::RGBA)
            .blend_enable(false)
            .build();
        let attachments = [attachment];

        let blending = PipelineColorBlendStateCreateInfo::builder()
            .logic_op_enable(false)
            .attachments(&attachments);

        let create_info = PipelineLayoutCreateInfo::default();
        let layout = unsafe { device.create_pipeline_layout(&create_info, None)? };

        let create_info = GraphicsPipelineCreateInfo::builder()
            .stages(&stages)
            .vertex_input_state(&vertex_input)
            .input_assembly_state(&input_assembly)
            .viewport_state(&viewport_state)
            .rasterization_state(&raster)
            .multisample_state(&multisampling)
            .depth_stencil_state(&depth_stencil)
            .color_blend_state(&blending)
            .dynamic_state(&dynamic_state)
            .layout(layout)
            .render_pass(self.render_pass.expect("Missing renderpass").handle)
            .subpass(self.subpass.expect("Missing subpass"))
            .build();

        let result = unsafe {
            device.create_graphics_pipelines(PipelineCache::null(), &[create_info], None)
        };
        match result {
            Ok(handles) => Ok(Graphics {
                handle: *handles.first().unwrap(),
                layout,
            }),
            Err((_, result)) => Err(result),
        }
    }
}