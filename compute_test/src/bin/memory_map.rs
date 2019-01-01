extern crate compute_test;

#[macro_use]
extern crate log;
use compute_test::utils;

extern crate gfx_backend_vulkan as back;
extern crate gfx_hal as hal;

use hal::{buffer, memory};
use hal::{Compute, Device, Instance, PhysicalDevice, QueueFamily};

fn run() {
    let instance = back::Instance::create("gfx-rs compute", 1);
    let adapter = instance
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
    let (mut device, mut _queue_group) =
        adapter.open_with::<_, Compute>(1, |_family| true).unwrap();

    let limits = adapter.physical_device.limits();
    info!("Noncoherent atom size: {}", limits.non_coherent_atom_size);

    // Although the data we're sending is not a multiple of noncoherent atom size, the underlying
    // buffer will be.
    let numbers: Vec<u32> = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
    info!("Creating staging buffer.");

    // Create staging buffers, copy data.
    let mut buffer_memory = utils::create_buffer::<back::Backend, u32>(
        &mut device,
        &memory_properties.memory_types,
        memory::Properties::CPU_VISIBLE | memory::Properties::COHERENT,
        buffer::Usage::TRANSFER_SRC | buffer::Usage::TRANSFER_DST,
        &numbers,
    );
    info!("Created empty buffer");
    utils::fill_buffer::<back::Backend, u32>(&mut device, &mut buffer_memory, &numbers);

    // Read the numbers back out.
    let mut numbers_out: Vec<u32> = Vec::with_capacity(numbers.len());
    utils::read_buffer::<back::Backend, u32>(
        &mut device,
        &mut buffer_memory,
        &mut numbers_out,
        numbers.len(),
    );
    assert!(numbers_out.len() == numbers.len());
    for x in numbers.iter().zip(numbers_out.iter()) {
        assert!(x.0 == x.1);
    }

    unsafe {
        device.destroy_buffer(buffer_memory.buffer);
        device.free_memory(buffer_memory.memory);
    }
}

fn main() {
    let _ = compute_test::logging::init_logger(log::Level::Trace);
    run();
}
