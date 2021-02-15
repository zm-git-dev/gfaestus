use vulkano::format::Format;
use vulkano::framebuffer::{Framebuffer, FramebufferAbstract, RenderPassAbstract, Subpass};
use vulkano::image::{ImageUsage, SwapchainImage};
use vulkano::instance::debug::{DebugCallback, MessageSeverity, MessageType};
use vulkano::instance::{Instance, PhysicalDevice};
use vulkano::{
    buffer::cpu_pool::CpuBufferPoolChunk,
    device::{Device, DeviceExtensions, RawDeviceExtensions},
    memory::pool::StdMemoryPool,
};
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer, CpuBufferPool, ImmutableBuffer},
    image::{AttachmentImage, Dimensions},
};
use vulkano::{
    command_buffer::{AutoCommandBuffer, AutoCommandBufferBuilder, DynamicState, SubpassContents},
    pipeline::vertex::TwoBuffersDefinition,
};
use vulkano::{
    descriptor::{descriptor_set::PersistentDescriptorSet, PipelineLayoutAbstract},
    device::Queue,
};

use vulkano::pipeline::{viewport::Viewport, GraphicsPipeline, GraphicsPipelineAbstract};

use vulkano::swapchain::{
    self, AcquireError, ColorSpace, FullscreenExclusive, PresentMode, SurfaceTransform, Swapchain,
    SwapchainCreationError,
};
use vulkano::sync::{self, FlushError, GpuFuture};

use vulkano_win::VkSurfaceBuild;

use std::sync::Arc;

use crossbeam::channel;

use anyhow::{Context, Result};

use nalgebra_glm as glm;

use crate::geometry::*;
use crate::gfa::*;
use crate::ui::events::{keyboard_input, mouse_wheel_input};
use crate::ui::{UICmd, UIState, UIThread};
use crate::view;
use crate::view::View;

use crate::input::*;

use crate::layout::physics;
use crate::layout::*;

use super::{PoolChunk, SubPoolChunk};

mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "shaders/nodes/vertex.vert",
    }
}

mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/nodes/fragment.frag",
    }
}

pub struct NodeDrawSystem {
    gfx_queue: Arc<Queue>,
    vertex_buffer_pool: CpuBufferPool<Vertex>,
    color_buffer_pool: CpuBufferPool<Color>,
    uniform_buffer_pool: CpuBufferPool<vs::ty::View>,
    pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
    // command_buffer_builder: AutoCommandBuffer,
}

impl NodeDrawSystem {
    pub fn new<R>(gfx_queue: Arc<Queue>, subpass: Subpass<R>) -> NodeDrawSystem
    where
        R: RenderPassAbstract + Send + Sync + 'static,
    {
        let _ = include_str!("../../shaders/nodes/fragment.frag");
        let _ = include_str!("../../shaders/nodes/vertex.vert");

        let vs = vs::Shader::load(gfx_queue.device().clone()).unwrap();
        let fs = fs::Shader::load(gfx_queue.device().clone()).unwrap();

        let vertex_buffer_pool: CpuBufferPool<Vertex> =
            CpuBufferPool::vertex_buffer(gfx_queue.device().clone());
        let color_buffer_pool: CpuBufferPool<Color> =
            CpuBufferPool::vertex_buffer(gfx_queue.device().clone());

        let uniform_buffer_pool = CpuBufferPool::<vs::ty::View>::new(
            gfx_queue.device().clone(),
            BufferUsage::uniform_buffer(),
        );

        let pipeline = {
            Arc::new(
                GraphicsPipeline::start()
                    .vertex_input(TwoBuffersDefinition::<Vertex, Color>::new())
                    .vertex_shader(vs.main_entry_point(), ())
                    .triangle_list()
                    .viewports_dynamic_scissors_irrelevant(1)
                    .fragment_shader(fs.main_entry_point(), ())
                    .render_pass(subpass)
                    .blend_alpha_blending()
                    .build(gfx_queue.device().clone())
                    .unwrap(),
            ) as Arc<_>
        };

        NodeDrawSystem {
            gfx_queue,
            pipeline,
            vertex_buffer_pool,
            color_buffer_pool,
            uniform_buffer_pool,
        }
    }

    pub fn draw<VI, CI>(
        &self,
        dynamic_state: &DynamicState,
        viewport_dims: [f32; 2],
        vertices: VI,
        colors: CI,
        view: View,
        offset: Point,
    ) -> Result<AutoCommandBuffer>
    where
        VI: IntoIterator<Item = Vertex>,
        VI::IntoIter: ExactSizeIterator,
        CI: IntoIterator<Item = Color>,
        CI::IntoIter: ExactSizeIterator,
    {
        let mut builder: AutoCommandBufferBuilder = AutoCommandBufferBuilder::secondary_graphics(
            self.gfx_queue.device().clone(),
            self.gfx_queue.family(),
            self.pipeline.clone().subpass(),
        )?;

        #[rustfmt::skip]
        let transformation = {
            let model_mat = glm::mat4(
                1.0, 0.0, 0.0, offset.x,
                0.0, 1.0, 0.0, offset.y,
                0.0, 0.0, 1.0, 0.0,
                0.0, 0.0, 0.0, 1.0
            );

            let view_mat = view.to_scaled_matrix();

            let width = viewport_dims[0];
            let height = viewport_dims[1];

            let viewport_mat = view::viewport_scale(width, height);

            let matrix = viewport_mat * view_mat * model_mat;

            let view_data = view::mat4_to_array(&matrix);

            let uniform_view = vs::ty::View { view: view_data };
            self.uniform_buffer_pool.next(uniform_view)
        }?;

        let layout = self.pipeline.descriptor_set_layout(0).unwrap();

        let set = Arc::new(
            PersistentDescriptorSet::start(layout.clone())
                .add_buffer(transformation)
                .unwrap()
                .build()
                .unwrap(),
        );

        let vertex_buffer = self.vertex_buffer_pool.chunk(vertices)?;
        let color_buffer = self.color_buffer_pool.chunk(colors)?;

        builder.draw(
            self.pipeline.clone(),
            dynamic_state,
            vec![Arc::new(vertex_buffer), Arc::new(color_buffer)],
            set.clone(),
            (),
        )?;

        let builder = builder.build()?;

        Ok(builder)
    }
}