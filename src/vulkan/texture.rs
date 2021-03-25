use ash::{version::DeviceV1_0, vk, Device};

use anyhow::Result;

#[derive(Clone, Copy)]
pub struct Texture {
    pub image: vk::Image,
    pub memory: vk::DeviceMemory,
    pub view: vk::ImageView,
    pub sampler: Option<vk::Sampler>,
}

impl Texture {
    pub fn new(
        image: vk::Image,
        memory: vk::DeviceMemory,
        view: vk::ImageView,
        sampler: Option<vk::Sampler>,
    ) -> Self {
        Texture {
            image,
            memory,
            view,
            sampler,
        }
    }

    pub fn destroy(&mut self, device: &Device) {
        unsafe {
            if let Some(sampler) = self.sampler.take() {
                device.destroy_sampler(sampler, None);
            }
            device.destroy_image_view(self.view, None);
            device.destroy_image(self.image, None);
            device.free_memory(self.memory, None);
        }
    }

    pub fn create_transient_color(
        vk_context: &super::context::VkContext,
        command_pool: vk::CommandPool,
        transition_queue: vk::Queue,
        swapchain_props: super::SwapchainProperties,
        msaa_samples: vk::SampleCountFlags,
    ) -> Result<Self> {
        let format = swapchain_props.format.format;

        use vk::ImageLayout as Layout;
        use vk::ImageUsageFlags as Usage;

        let (img, mem) = super::GfaestusVk::create_image(
            vk_context,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
            swapchain_props.extent,
            msaa_samples,
            format,
            vk::ImageTiling::OPTIMAL,
            Usage::TRANSIENT_ATTACHMENT | Usage::COLOR_ATTACHMENT,
        )?;

        super::GfaestusVk::transition_image(
            vk_context.device(),
            command_pool,
            transition_queue,
            img,
            format,
            Layout::UNDEFINED,
            Layout::COLOR_ATTACHMENT_OPTIMAL,
        )?;

        let view = super::GfaestusVk::create_image_view(
            vk_context.device(),
            img,
            1,
            format,
            vk::ImageAspectFlags::COLOR,
        )?;

        Ok(Self::new(img, mem, view, None))
    }
}

#[derive(Clone, Copy)]
pub struct Texture1D {
    pub image: vk::Image,
    pub memory: vk::DeviceMemory,
    pub view: vk::ImageView,
}

impl Texture1D {
    pub fn new(
        image: vk::Image,
        memory: vk::DeviceMemory,
        view: vk::ImageView,
    ) -> Self {
        Texture1D {
            image,
            memory,
            view,
        }
    }

    pub fn destroy(&mut self, device: &Device) {
        unsafe {
            device.destroy_image_view(self.view, None);
            device.destroy_image(self.image, None);
            device.free_memory(self.memory, None);
        }
    }

    pub fn create_from_colors(
        app: &super::GfaestusVk,
        // vk_context: &super::context::VkContext,
        command_pool: vk::CommandPool,
        transition_queue: vk::Queue,
        colors: &[rgb::RGB<f32>],
    ) -> Result<Self> {
        use vk::BufferUsageFlags as BufUsage;
        use vk::ImageLayout as Layout;
        use vk::ImageUsageFlags as ImgUsage;
        use vk::MemoryPropertyFlags as MemProps;

        let vk_context = app.vk_context();
        let device = vk_context.device();

        let format = vk::Format::R8G8B8_UNORM;

        let image_size =
            (colors.len() * 3 * std::mem::size_of::<u8>()) as vk::DeviceSize;

        let (buffer, buf_mem, buf_size) = app.create_buffer(
            // vk_context,
            image_size,
            BufUsage::TRANSFER_SRC,
            MemProps::HOST_VISIBLE | MemProps::HOST_COHERENT,
        )?;

        let mut pixels: Vec<u8> = Vec::with_capacity(colors.len() * 3);

        for &color in colors {
            let r = (color.r * 255.0).floor() as u8;
            let g = (color.g * 255.0).floor() as u8;
            let b = (color.b * 255.0).floor() as u8;

            pixels.push(r);
            pixels.push(g);
            pixels.push(b);
        }

        unsafe {
            let ptr = device.map_memory(
                buf_mem,
                0,
                image_size,
                vk::MemoryMapFlags::empty(),
            )?;

            let mut align = ash::util::Align::new(
                ptr,
                std::mem::align_of::<u8>() as _,
                buf_size,
            );
            align.copy_from_slice(&pixels);
            device.unmap_memory(buf_mem);
        }

        let extent = vk::Extent3D {
            width: colors.len() as u32,
            height: 1,
            depth: 1,
        };

        let img_info = vk::ImageCreateInfo::builder()
            .image_type(vk::ImageType::TYPE_2D)
            .extent(extent)
            .mip_levels(1)
            .array_layers(1)
            .format(format)
            .tiling(vk::ImageTiling::LINEAR)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .usage(ImgUsage::TRANSFER_SRC | ImgUsage::TRANSFER_DST)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            // .samples(sample_count)
            .flags(vk::ImageCreateFlags::empty())
            .build();

        let image = unsafe { device.create_image(&img_info, None) }?;
        let mem_reqs = unsafe { device.get_image_memory_requirements(image) };
        let mem_type_ix = super::find_memory_type(
            mem_reqs,
            vk_context.get_mem_properties(),
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        );

        let alloc_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(mem_reqs.size)
            .memory_type_index(mem_type_ix)
            .build();

        let memory = unsafe {
            let mem = device.allocate_memory(&alloc_info, None)?;
            device.bind_image_memory(image, mem, 0)?;
            mem
        };

        {
            // use super::GfaestusVk;

            super::GfaestusVk::transition_image(
                device,
                command_pool,
                transition_queue,
                image,
                format,
                Layout::UNDEFINED,
                Layout::TRANSFER_DST_OPTIMAL,
            )?;

            super::GfaestusVk::copy_buffer_to_image(
                device,
                command_pool,
                transition_queue,
                buffer,
                image,
                vk::Extent2D {
                    width: extent.width,
                    height: 1,
                },
            )?;

            super::GfaestusVk::transition_image(
                device,
                command_pool,
                transition_queue,
                image,
                format,
                Layout::TRANSFER_DST_OPTIMAL,
                Layout::SHADER_READ_ONLY_OPTIMAL,
            )?;
        }

        let view = super::GfaestusVk::create_image_view(
            vk_context.device(),
            image,
            1,
            format,
            vk::ImageAspectFlags::COLOR,
        )?;

        Ok(Self::new(image, memory, view))
    }
}
