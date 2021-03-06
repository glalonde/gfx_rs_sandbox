extern crate compute_test;

#[macro_use]
extern crate log;
use compute_test::utils;

extern crate gfx_backend_vulkan as back;
extern crate gfx_hal as hal;

use hal::{buffer, command, memory, pool, pso, queue};
use hal::{Compute, DescriptorPool, Device, Instance, PhysicalDevice, QueueFamily};

fn run() {
    let instance = back::Instance::create("gfx-rs compute", 1);
    let mut adapter = instance
        .enumerate_adapters()
        .into_iter()
        .find(|a| {
            a.queue_families
                .iter()
                .any(|family| family.supports_compute())
        })
        .expect("Failed to find a GPU with compute support!");

    info!("Using device: {}", adapter.info.name);

    let memory_properties = adapter.physical_device.memory_properties();
    let (mut device, mut queue_group) = adapter.open_with::<_, Compute>(1, |_family| true).unwrap();

    let spirv = include_bytes!("../../assets/gen/shaders/compute_test.compute.spv");
    let shader = device.create_shader_module(spirv).unwrap();
    let limits = adapter.physical_device.limits();
    info!("Noncoherent atom size: {}", limits.non_coherent_atom_size);

    // Make pipeline
    let (pipeline_layout, pipeline, set_layout, mut desc_pool) = {
        let set_layout = device.create_descriptor_set_layout(
            &[pso::DescriptorSetLayoutBinding {
                binding: 0,
                ty: pso::DescriptorType::StorageBuffer,
                count: 1,
                stage_flags: pso::ShaderStageFlags::COMPUTE,
                immutable_samplers: false,
            }],
            &[],
        );

        let pipeline_layout = device.create_pipeline_layout(Some(&set_layout), &[]);
        let entry_point = pso::EntryPoint {
            entry: "main",
            module: &shader,
            specialization: pso::Specialization::default(),
        };
        let pipeline = device
            .create_compute_pipeline(
                &pso::ComputePipelineDesc::new(entry_point, &pipeline_layout),
                None,
            )
            .expect("Error creating compute pipeline!");

        let desc_pool = device.create_descriptor_pool(
            1,
            &[pso::DescriptorRangeDesc {
                ty: pso::DescriptorType::StorageBuffer,
                count: 1,
            }],
        );
        (pipeline_layout, pipeline, set_layout, desc_pool)
    };

    let numbers = vec![1, 2, 3, 4, 5, 6, 7];
    let stride = std::mem::size_of::<u32>() as u64;
    info!("Creating staging buffer.");

    // Create staging buffers, copy data.
    let (staging_buffer, staging_memory) = utils::create_buffer::<back::Backend, u32>(
        &mut device,
        &memory_properties.memory_types,
        memory::Properties::CPU_VISIBLE | memory::Properties::COHERENT,
        buffer::Usage::TRANSFER_SRC | buffer::Usage::TRANSFER_DST,
        &numbers,
    );

    info!("Creating device buffer.");

    // Create device buffer
    let (device_buffer, device_memory) = utils::empty_buffer::<back::Backend, u32>(
        &mut device,
        &memory_properties.memory_types,
        memory::Properties::DEVICE_LOCAL,
        buffer::Usage::TRANSFER_SRC | buffer::Usage::TRANSFER_DST | buffer::Usage::STORAGE,
        numbers.len(),
    );
    info!("Creating descriptor pool.");

    let desc_set = desc_pool.allocate_set(&set_layout).unwrap();
    device.write_descriptor_sets(Some(pso::DescriptorSetWrite {
        set: &desc_set,
        binding: 0,
        array_offset: 0,
        descriptors: Some(pso::Descriptor::Buffer(&device_buffer, None..None)),
    }));
    info!("Creating command pool.");

    let mut command_pool =
        device.create_command_pool_typed(&queue_group, pool::CommandPoolCreateFlags::empty(), 16);
    let fence = device.create_fence(false);
    let submission = queue::Submission::new().submit(Some({
        let mut command_buffer = command_pool.acquire_command_buffer(false);
        command_buffer.copy_buffer(
            &staging_buffer,
            &device_buffer,
            &[command::BufferCopy {
                src: 0,
                dst: 0,
                size: stride * numbers.len() as u64,
            }],
        );
        command_buffer.pipeline_barrier(
            pso::PipelineStage::TRANSFER..pso::PipelineStage::COMPUTE_SHADER,
            memory::Dependencies::empty(),
            Some(memory::Barrier::Buffer {
                states: buffer::Access::TRANSFER_WRITE
                    ..buffer::Access::SHADER_READ | buffer::Access::SHADER_WRITE,
                target: &device_buffer,
            }),
        );
        command_buffer.bind_compute_pipeline(&pipeline);
        command_buffer.bind_compute_descriptor_sets(&pipeline_layout, 0, &[desc_set], &[]);
        command_buffer.dispatch([numbers.len() as u32, 1, 1]);
        command_buffer.pipeline_barrier(
            pso::PipelineStage::COMPUTE_SHADER..pso::PipelineStage::TRANSFER,
            memory::Dependencies::empty(),
            Some(memory::Barrier::Buffer {
                states: buffer::Access::SHADER_READ | buffer::Access::SHADER_WRITE
                    ..buffer::Access::TRANSFER_READ,
                target: &device_buffer,
            }),
        );
        command_buffer.copy_buffer(
            &device_buffer,
            &staging_buffer,
            &[command::BufferCopy {
                src: 0,
                dst: 0,
                size: stride * numbers.len() as u64,
            }],
        );
        command_buffer.finish()
    }));
    queue_group.queues[0].submit(submission, Some(&fence));
    device.wait_for_fence(&fence, !0);

    {
        let mut output = vec![0; numbers.len()];
        utils::read_buffer::<back::Backend, u32>(&device, &staging_memory, &mut output);
        for val in output {
            info!("out: {}", val);
        }
    }

    device.destroy_command_pool(command_pool.into_raw());
    device.destroy_descriptor_pool(desc_pool);
    device.destroy_descriptor_set_layout(set_layout);
    device.destroy_shader_module(shader);
    device.destroy_buffer(device_buffer);
    device.destroy_buffer(staging_buffer);
    device.destroy_fence(fence);
    device.destroy_pipeline_layout(pipeline_layout);
    device.free_memory(device_memory);
    device.free_memory(staging_memory);
    device.destroy_compute_pipeline(pipeline);
}

fn main() {
    let _ = compute_test::logging::init_logger(log::Level::Trace);
    run();
}
