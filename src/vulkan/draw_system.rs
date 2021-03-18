use ash::{
    extensions::{
        ext::DebugReport,
        khr::{Surface, Swapchain},
    },
    version::{DeviceV1_0, EntryV1_0, InstanceV1_0},
};
use ash::{vk, Device, Entry, Instance};

use std::ffi::CString;

use std::sync::{Arc, Weak};

use nalgebra_glm as glm;

use anyhow::Result;

use super::SwapchainProperties;

fn read_shader_from_file<P>(path: P) -> Result<Vec<u32>>
where
    P: AsRef<std::path::Path>,
{
    use std::{fs::File, io::Read};

    let mut buf = Vec::new();
    let mut file = File::open(path)?;
    file.read_to_end(&mut buf)?;

    let mut cursor = std::io::Cursor::new(buf);

    let spv = ash::util::read_spv(&mut cursor)?;
    Ok(spv)
}

fn create_shader_module(device: &Device, code: &[u32]) -> vk::ShaderModule {
    let create_info = vk::ShaderModuleCreateInfo::builder().code(code).build();
    unsafe { device.create_shader_module(&create_info, None).unwrap() }
}

pub struct GfaestusCmdBuf {
    command_buffer: vk::CommandBuffer,
}

impl GfaestusCmdBuf {
    pub fn frame(
        device: &Device,
        pool: vk::CommandPool,
        render_pass: vk::RenderPass,
        framebuffer: &vk::Framebuffer,
        swapchain_props: SwapchainProperties,
        // vertex_buffer: vk::Buffer,
        // pipeline_layout: vk::PipelineLayout,
        // descriptor_sets: &[vk::DescriptorSet],
        // graphics_pipeline: vk::Pipeline,
    ) -> Result<vk::CommandBuffer> {
        let alloc_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1)
            .build();

        let buffers = unsafe { device.allocate_command_buffers(&alloc_info) }?;

        for (i, buf) in buffers.iter().enumerate() {
            let buf = *buf;
            // let framebuffer = framebuffers[i];

            {
                let cmd_buf_begin_info = vk::CommandBufferBeginInfo::builder()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
                    .build();

                unsafe {
                    device.begin_command_buffer(buf, &cmd_buf_begin_info)
                }?;
            }

            {
                let clear_values = [
                    vk::ClearValue {
                        color: vk::ClearColorValue {
                            float32: [0.0, 0.0, 0.0, 1.0],
                        },
                    },
                    vk::ClearValue {
                        depth_stencil: vk::ClearDepthStencilValue {
                            depth: 1.0,
                            stencil: 0,
                        },
                    },
                ];

                let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
                    .render_pass(render_pass)
                    .framebuffer(*framebuffer)
                    .render_area(vk::Rect2D {
                        offset: vk::Offset2D { x: 0, y: 0 },
                        extent: swapchain_props.extent,
                    })
                    .clear_values(&clear_values)
                    .build();

                unsafe {
                    device.cmd_begin_render_pass(
                        buf,
                        &render_pass_begin_info,
                        vk::SubpassContents::INLINE,
                    )
                };
            }

            // TODO bind pipeline
            // TODO bind buffers
            // TODO draw

            unsafe { device.cmd_end_render_pass(buf) };

            unsafe { device.end_command_buffer(buf) }?;
        }

        Ok(buffers[0])
    }
}

pub struct NodeDrawAsh {
    vk_context: Weak<super::VkContext>,
    descriptor_set_pool: Weak<vk::DescriptorPool>,

    render_pass: vk::RenderPass,
    descriptor_set_layout: vk::DescriptorSetLayout,

    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,

    vertex_buffer: vk::Buffer,
    vertex_buffer_memory: vk::DeviceMemory,

    uniform_buffer: vk::Buffer,
    uniform_buffer_memory: vk::DeviceMemory,

    descriptor_set: vk::DescriptorSet,
}

/*
impl Drop for NodeDrawAsh {
    fn drop(&mut self) {
        let device =
        unsafe {
            device.destroy_
        }
    }
}
*/

// pub struct NodesUBO {
//     matrix: glm::Mat4,
// }

pub struct NodeUniform {
    view_transform: glm::Mat4,
}

impl NodeUniform {
    fn get_descriptor_set_layout_binding() -> vk::DescriptorSetLayoutBinding {
        use vk::ShaderStageFlags as Stages;

        vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(Stages::VERTEX | Stages::FRAGMENT)
            .build()
    }
}

impl NodeDrawAsh {
    fn create_descriptor_set_layout(
        device: &Device,
    ) -> vk::DescriptorSetLayout {
        let ubo_binding = NodeUniform::get_descriptor_set_layout_binding();
        let bindings = [ubo_binding];

        let layout_info = vk::DescriptorSetLayoutCreateInfo::builder()
            .bindings(&bindings)
            .build();

        unsafe {
            device
                .create_descriptor_set_layout(&layout_info, None)
                .unwrap()
        }
    }

    fn create_pipeline(
        device: &Device,
        swapchain_props: SwapchainProperties,
        msaa_samples: vk::SampleCountFlags,
        render_pass: vk::RenderPass,
    ) -> (vk::Pipeline, vk::PipelineLayout) {
        let vert_src =
            read_shader_from_file("shaders/nodes_simple.vert.spv").unwrap();
        let frag_src =
            read_shader_from_file("shaders/nodes_simple.vert.spv").unwrap();

        let vert_module = create_shader_module(device, &vert_src);
        let frag_module = create_shader_module(device, &frag_src);

        let entry_point = CString::new("main").unwrap();

        let vert_state_info = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(vert_module)
            .name(&entry_point)
            .build();

        let frag_state_info = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(frag_module)
            .name(&entry_point)
            .build();

        let shader_state_infos = [vert_state_info, frag_state_info];

        let vert_binding_descs = [Vertex::get_binding_desc()];
        let vert_attr_descs = Vertex::get_attribute_descs();
        let vert_input_info = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(&vert_binding_descs)
            .vertex_attribute_descriptions(&vert_attr_descs)
            .build();

        let input_assembly_info =
            vk::PipelineInputAssemblyStateCreateInfo::builder()
                .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
                .primitive_restart_enable(false)
                .build();

        let viewport = vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: swapchain_props.extent.width as f32,
            height: swapchain_props.extent.height as f32,
            min_depth: 0.0,
            max_depth: 0.0,
        };
        let viewports = [viewport];

        let scissor = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: swapchain_props.extent,
        };
        let scissors = [scissor];

        let viewport_info = vk::PipelineViewportStateCreateInfo::builder()
            .viewports(&viewports)
            .scissors(&scissors)
            .build();

        let rasterizer_info =
            vk::PipelineRasterizationStateCreateInfo::builder()
                .depth_clamp_enable(false)
                .rasterizer_discard_enable(false)
                .polygon_mode(vk::PolygonMode::FILL)
                .line_width(1.0)
                .cull_mode(vk::CullModeFlags::NONE)
                .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
                .depth_bias_enable(false)
                .depth_bias_constant_factor(0.0)
                .depth_bias_clamp(0.0)
                .depth_bias_slope_factor(0.0)
                .build();

        // let depth_stencil_info = todo!();

        let multisampling_info =
            vk::PipelineMultisampleStateCreateInfo::builder()
                .sample_shading_enable(false)
                .rasterization_samples(msaa_samples)
                .min_sample_shading(1.0)
                .alpha_to_coverage_enable(false)
                .alpha_to_one_enable(false)
                .build();

        let color_blend_attachment =
            vk::PipelineColorBlendAttachmentState::builder()
                .color_write_mask(vk::ColorComponentFlags::all())
                .blend_enable(true)
                .src_color_blend_factor(vk::BlendFactor::ONE)
                .dst_color_blend_factor(vk::BlendFactor::ZERO)
                .color_blend_op(vk::BlendOp::ADD)
                .src_alpha_blend_factor(vk::BlendFactor::ONE)
                .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
                .alpha_blend_op(vk::BlendOp::ADD)
                .build();
        let color_blend_attachments = [color_blend_attachment];

        let color_blending_info =
            vk::PipelineColorBlendStateCreateInfo::builder()
                .logic_op_enable(false)
                .logic_op(vk::LogicOp::COPY)
                .attachments(&color_blend_attachments)
                .blend_constants([0.0, 0.0, 0.0, 0.0])
                .build();

        let descriptor_set_layout = Self::create_descriptor_set_layout(device);
        let layout = {
            let layouts = [descriptor_set_layout];
            let layout_info = vk::PipelineLayoutCreateInfo::builder()
                .set_layouts(&layouts)
                .build();

            unsafe {
                device.create_pipeline_layout(&layout_info, None).unwrap()
            }
        };

        let pipeline_info = vk::GraphicsPipelineCreateInfo::builder()
            .stages(&shader_state_infos)
            .vertex_input_state(&vert_input_info)
            .input_assembly_state(&input_assembly_info)
            .viewport_state(&viewport_info)
            .rasterization_state(&rasterizer_info)
            .multisample_state(&multisampling_info)
            .color_blend_state(&color_blending_info)
            .layout(layout)
            .render_pass(render_pass)
            .subpass(0)
            .build();

        let pipeline_infos = [pipeline_info];

        let pipeline = unsafe {
            device
                .create_graphics_pipelines(
                    vk::PipelineCache::null(),
                    &pipeline_infos,
                    None,
                )
                .unwrap()[0]
        };

        unsafe {
            device.destroy_shader_module(vert_module, None);
            device.destroy_shader_module(frag_module, None);
        }

        (pipeline, layout)
    }

    pub fn new(
        vk_context: &Arc<super::VkContext>,
        desc_pool: &Arc<vk::DescriptorPool>,

        render_pass: vk::RenderPass,
    ) -> Result<Self> {
        let vk_context = Arc::downgrade(vk_context);
        let descriptor_pool = Arc::downgrade(desc_pool);
        unimplemented!();
    }

    fn descriptor_set_layout(device: &Device) -> vk::DescriptorSetLayout {
        // let ubo_binding = Unif

        let layout_info = vk::DescriptorSetLayoutCreateInfo::builder()
            .bindings(&[])
            .build();

        unsafe {
            device
                .create_descriptor_set_layout(&layout_info, None)
                .unwrap()
        }
    }
}

#[derive(Clone, Copy)]
struct Vertex {
    position: [f32; 2],
}

impl Vertex {
    fn get_binding_desc() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(std::mem::size_of::<Vertex>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()
    }

    fn get_attribute_descs() -> [vk::VertexInputAttributeDescription; 1] {
        let pos_desc = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(0)
            .format(vk::Format::R32G32_SFLOAT)
            .offset(0)
            .build();

        [pos_desc]
    }
}
