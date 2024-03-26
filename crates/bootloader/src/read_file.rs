use core::ops::Deref;
use core::ops::DerefMut;
use uefi::prelude::*;
use uefi::proto::media::file::File;

pub fn load_file_from_disk(
    name: &str,
    image: Handle,
    st: &SystemTable<Boot>,
) -> Result<&'static mut [u8], uefi::Error> {
    let mut file_system_raw =
        locate_and_open_protocol::<uefi::proto::media::fs::SimpleFileSystem>(image, st)?;
    let file_system = file_system_raw.deref_mut();

    let mut root = file_system.open_volume()?;
    let mut buf = [0u16; 256];

    let filename =
        uefi::CStr16::from_str_with_buf(name, &mut buf).expect("Failed to convert string to utf16");

    let file_handle = root.open(
        filename,
        uefi::proto::media::file::FileMode::Read,
        uefi::proto::media::file::FileAttribute::empty(),
    )?;

    let mut file = match file_handle.into_type()? {
        uefi::proto::media::file::FileType::Regular(f) => f,
        uefi::proto::media::file::FileType::Dir(_) => panic!(),
    };

    let file_info = file
        .get_boxed_info::<uefi::proto::media::file::FileInfo>()
        .unwrap();
    let file_size = usize::try_from(file_info.file_size()).unwrap();

    let file_ptr = st.boot_services().allocate_pages(
        uefi::table::boot::AllocateType::AnyPages,
        uefi::table::boot::MemoryType::LOADER_DATA,
        ((file_size - 1) / 4096) + 1,
    )? as *mut u8;

    unsafe { core::ptr::write_bytes(file_ptr, 0, file_size) };
    let file_slice = unsafe { core::slice::from_raw_parts_mut(file_ptr, file_size) };
    file.read(file_slice).unwrap();

    Ok(file_slice)
}

fn locate_and_open_protocol<P: uefi::proto::ProtocolPointer>(
    image: Handle,
    st: &SystemTable<Boot>,
) -> Result<uefi::table::boot::ScopedProtocol<P>, uefi::Error> {
    let this = st.boot_services();
    let device_path = open_device_path_protocol(image, st)?;
    let mut device_path = device_path.deref();

    let fs_handle = this.locate_device_path::<P>(&mut device_path)?;

    let opened_handle = unsafe {
        this.open_protocol::<P>(
            uefi::table::boot::OpenProtocolParams {
                handle: fs_handle,
                agent: image,
                controller: None,
            },
            uefi::table::boot::OpenProtocolAttributes::Exclusive,
        )
    }?;

    Ok(opened_handle)
}

fn open_device_path_protocol(
    image: Handle,
    st: &SystemTable<Boot>,
) -> Result<uefi::table::boot::ScopedProtocol<uefi::proto::device_path::DevicePath>, uefi::Error> {
    let this = st.boot_services();
    let device_handle = unsafe {
        this.open_protocol::<uefi::proto::loaded_image::LoadedImage>(
            uefi::table::boot::OpenProtocolParams {
                handle: image,
                agent: image,
                controller: None,
            },
            uefi::table::boot::OpenProtocolAttributes::Exclusive,
        )
    }?
    .deref()
    .device()
    .unwrap();

    let device_path = unsafe {
        this.open_protocol::<uefi::proto::device_path::DevicePath>(
            uefi::table::boot::OpenProtocolParams {
                handle: device_handle,
                agent: image,
                controller: None,
            },
            uefi::table::boot::OpenProtocolAttributes::Exclusive,
        )
    }?;

    Ok(device_path)
}
