use gfx_hal::Backend;
use prelude::*;

pub struct BufferMemory<B: Backend> {
    pub buffer: B::Buffer,
    pub memory: B::Memory,
    pub size: u64,
}

/// Creates an empty buffer of a certain type and size.
pub fn empty_buffer<B: Backend, Item>(
    device: &B::Device,
    memory_types: &[MemoryType],
    properties: Properties,
    usage: buffer::Usage,
    item_count: usize,
) -> BufferMemory<B> {
    let item_count = item_count; // NOTE: Change
    let stride = ::std::mem::size_of::<Item>() as u64;
    let buffer_len = item_count as u64 * stride;
    let unbound_buffer = device.create_buffer(buffer_len, usage).unwrap();
    let req = device.get_buffer_requirements(&unbound_buffer);
    let upload_type = memory_types
        .iter()
        .enumerate()
        .position(|(id, ty)| req.type_mask & (1 << id) != 0 && ty.properties.contains(properties))
        .unwrap()
        .into();

    let buffer_memory = device.allocate_memory(upload_type, req.size).unwrap();
    let buffer = device
        .bind_buffer_memory(&buffer_memory, 0, unbound_buffer)
        .unwrap();
    BufferMemory::<B> {
        buffer: buffer,
        memory: buffer_memory,
        size: req.size,
    }
}

/// Pushes data into a buffer.
pub fn fill_buffer<B: Backend, Item: Copy>(
    device: &B::Device,
    buffer_memory: &mut BufferMemory<B>,
    items: &[Item],
) {
    let item_count = items.len();
    let stride = ::std::mem::size_of::<Item>() as u64;
    let buffer_len = item_count as u64 * stride;
    assert!(buffer_len <= buffer_memory.size);

    let mut dest = device
        .acquire_mapping_writer::<Item>(&buffer_memory.memory, 0..buffer_memory.size)
        .unwrap();
    dest[0..item_count].copy_from_slice(items);
    device.release_mapping_writer(dest);
}

pub fn read_buffer<B: Backend, Item: Copy>(
    device: &B::Device,
    buffer_memory: &mut BufferMemory<B>,
    items: &mut Vec<Item>,
    item_count: usize,
) {
    let stride = ::std::mem::size_of::<Item>() as u64;
    let buffer_len = item_count as u64 * stride;
    assert!(buffer_len <= buffer_memory.size);
    let source = device
        .acquire_mapping_reader::<Item>(&buffer_memory.memory, 0..buffer_memory.size)
        .unwrap();
    items.extend_from_slice(&source[0..item_count]);
    device.release_mapping_reader(source);
}

/// Creates a buffer and immediately fills it.
pub fn create_buffer<B: Backend, Item: Copy>(
    device: &B::Device,
    memory_types: &[MemoryType],
    properties: Properties,
    usage: buffer::Usage,
    items: &[Item],
) -> BufferMemory<B> {
    let mut buffer_memory =
        empty_buffer::<B, Item>(device, memory_types, properties, usage, items.len());

    fill_buffer::<B, Item>(device, &mut buffer_memory, items);

    buffer_memory
}

/// Reinterpret an instance of T as a slice of u32s that can be uploaded as push
/// constants.
pub fn push_constant_data<T>(data: &T) -> &[u32] {
    let size = push_constant_size::<T>();
    let ptr = data as *const T as *const u32;

    unsafe { ::std::slice::from_raw_parts(ptr, size) }
}

/// Determine the number of push constants required to store T.
/// Panics if T is not a multiple of 4 bytes - the size of a push constant.
pub fn push_constant_size<T>() -> usize {
    const PUSH_CONSTANT_SIZE: usize = ::std::mem::size_of::<u32>();
    let type_size = ::std::mem::size_of::<T>();

    // We want to ensure that the type we upload as a series of push constants
    // is actually representable as a series of u32 push constants.
    assert!(type_size % PUSH_CONSTANT_SIZE == 0);

    type_size / PUSH_CONSTANT_SIZE
}

/// Create an image, image memory, and image view with the given properties.
pub fn create_image<B: Backend>(
    device: &B::Device,
    memory_types: &[MemoryType],
    width: u32,
    height: u32,
    format: Format,
    usage: img::Usage,
    aspects: Aspects,
) -> (B::Image, B::Memory, B::ImageView) {
    let kind = img::Kind::D2(width, height, 1, 1);

    let unbound_image = device
        .create_image(
            kind,
            1,
            format,
            img::Tiling::Optimal,
            usage,
            ViewCapabilities::empty(),
        )
        .expect("Failed to create unbound image");

    let image_req = device.get_image_requirements(&unbound_image);

    let device_type = memory_types
        .iter()
        .enumerate()
        .position(|(id, memory_type)| {
            image_req.type_mask & (1 << id) != 0
                && memory_type.properties.contains(Properties::DEVICE_LOCAL)
        })
        .unwrap()
        .into();

    let image_memory = device
        .allocate_memory(device_type, image_req.size)
        .expect("Failed to allocate image");

    let image = device
        .bind_image_memory(&image_memory, 0, unbound_image)
        .expect("Failed to bind image");

    let image_view = device
        .create_image_view(
            &image,
            img::ViewKind::D2,
            format,
            Swizzle::NO,
            img::SubresourceRange {
                aspects,
                levels: 0..1,
                layers: 0..1,
            },
        )
        .expect("Failed to create image view");

    (image, image_memory, image_view)
}
