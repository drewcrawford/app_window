//SPDX-License-Identifier: MPL-2.0
use libc::{MFD_ALLOW_SEALING, MFD_CLOEXEC, c_char, memfd_create};
use memmap2::MmapMut;
use std::fs::File;
use std::os::fd::{AsFd, AsRawFd, FromRawFd};
use std::sync::{Arc, Mutex};
use wayland_client::protocol::wl_buffer::WlBuffer;
use wayland_client::protocol::wl_shm::{Format, WlShm};
use wayland_client::QueueHandle;
use zune_png::zune_core::result::DecodingResult;
use crate::sys::window::WindowInternal;
use super::{App, BufferReleaseInfo, ReleaseOpt};

#[derive(Debug, Clone)]
pub struct AllocatedBuffer {
    pub buffer: WlBuffer,
    pub width: i32,
    pub height: i32,
}

impl AllocatedBuffer {
    pub(super) fn new(
        width: i32,
        height: i32,
        shm: &WlShm,
        queue_handle: &QueueHandle<App>,
        window_internal: Arc<Mutex<WindowInternal>>,
    ) -> AllocatedBuffer {
        logwise::debuginternal_sync!("Creating shm buffer width {width}, height {height}",width=width,height=height);
        let file = unsafe {
            memfd_create(
                b"mem_fd\0" as *const _ as *const c_char,
                MFD_ALLOW_SEALING | MFD_CLOEXEC,
            )
        };
        if file < 0 {
            panic!(
                "Failed to create memfd: {err}",
                err = unsafe { *libc::__errno_location() }
            );
        }
        let file = unsafe { File::from_raw_fd(file) };

        let r = unsafe { libc::ftruncate(file.as_raw_fd(), (width * height * 4) as i64) };
        if r < 0 {
            panic!(
                "Failed to truncate memfd: {err}",
                err = unsafe { *libc::__errno_location() }
            );
        }

        let mut mmap = unsafe { MmapMut::map_mut(&file) }.unwrap();
        const DEFAULT_COLOR: [u8; 4] = [0, 0, 0xFF, 0xFF];
        for pixel in mmap.chunks_exact_mut(4) {
            pixel.copy_from_slice(&DEFAULT_COLOR); //I guess due to endiannness we are actually BGRA?
        }

        let pool = shm.create_pool(file.as_fd(), width * height * 4, queue_handle, ());
        let mmap = Arc::new(mmap);
        let release_opt = Arc::new(Mutex::new(Some(
            ReleaseOpt {
                _file: file,
                _mmap: mmap.clone(),
                allocated_buffer: None,
                window_internal: window_internal.clone(),
            }
        )));
        let release_info = BufferReleaseInfo {
            opt: release_opt.clone(),
            decor: false,
        };

        let buf = pool.create_buffer(
            0,
            width,
            height,
            width * 4,
            Format::Argb8888,
            queue_handle,
            release_info,
        );
        let allocated_buffer = AllocatedBuffer {
            buffer: buf,
            width,
            height,
        };
        release_opt.lock().unwrap().as_mut().unwrap().allocated_buffer = Some(allocated_buffer.clone());
        allocated_buffer
    }
}

pub(super) fn create_shm_buffer_decor(
    shm: &WlShm,
    queue_handle: &QueueHandle<App>,
    window_internal: Arc<Mutex<WindowInternal>>,
) -> AllocatedBuffer {
    let decor = include_bytes!("../../../linux_assets/decor.png");
    let mut decode_decor = zune_png::PngDecoder::new(decor);
    let decode = decode_decor.decode().expect("Can't decode decor");
    let dimensions = decode_decor.get_dimensions().unwrap();
    let decor = match decode {
        DecodingResult::U8(d) => d,
        _ => todo!(),
    };
    let file = unsafe {
        memfd_create(
            b"decor\0" as *const _ as *const c_char,
            MFD_ALLOW_SEALING | MFD_CLOEXEC,
        )
    };
    if file < 0 {
        panic!(
            "Failed to create memfd: {err}",
            err = unsafe { *libc::__errno_location() }
        );
    }
    let file = unsafe { File::from_raw_fd(file) };

    let r = unsafe { libc::ftruncate(file.as_raw_fd(), (dimensions.0 * dimensions.1 * 4) as i64) };
    if r < 0 {
        panic!(
            "Failed to truncate memfd: {err}",
            err = unsafe { *libc::__errno_location() }
        );
    }

    let mut mmap = unsafe { MmapMut::map_mut(&file) }.unwrap();
    for (pixel, decor_pixel) in mmap.chunks_exact_mut(4).zip(decor.chunks_exact(4)) {
        pixel.copy_from_slice(decor_pixel);
    }
    let pool = shm.create_pool(
        file.as_fd(),
        dimensions.0 as i32 * dimensions.1 as i32 * 4,
        queue_handle,
        (),
    );
    let release_opt = Arc::new(Mutex::new(Some(
        ReleaseOpt {
            _file: file,
            _mmap: Arc::new(mmap),
            allocated_buffer: None,
            window_internal: window_internal.clone(),
        }
    )));
    let release_info = BufferReleaseInfo {
        opt: release_opt.clone(),
        decor: true,
    };

    let buf = pool.create_buffer(
        0,
        dimensions.0 as i32,
        dimensions.1 as i32,
        dimensions.0 as i32 * 4,
        Format::Argb8888,
        queue_handle,
        release_info,
    );
    let allocated_buffer = AllocatedBuffer {
        buffer: buf,
        width: dimensions.0 as i32,
        height: dimensions.1 as i32,
    };
    release_opt.lock().unwrap().as_mut().unwrap().allocated_buffer = Some(allocated_buffer.clone());
    allocated_buffer
}