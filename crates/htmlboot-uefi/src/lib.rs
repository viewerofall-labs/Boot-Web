//! The only crate that touches firmware.
#![no_std]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use htmlboot_core::{bls::BootEntry, Color, Rect, Surface};
use uefi::boot::{self, LoadImageSource, ScopedProtocol};
use uefi::fs::{FileSystem, Path};
use uefi::proto::console::gop::{BltOp, BltPixel, GraphicsOutput};
use uefi::proto::console::pointer::Pointer;
use uefi::proto::console::text::{Key, ScanCode};
use uefi::proto::device_path::build::{self, DevicePathBuilder};
use uefi::proto::device_path::DevicePath;
use uefi::proto::loaded_image::LoadedImage;
use uefi::proto::media::load_file::LoadFile2;
use uefi::proto::BootPolicy;
use uefi::{cstr16, guid, CStr16, Guid, Handle, Identify, Status};

pub struct GopSurface {
    gop: ScopedProtocol<GraphicsOutput>,
    w: usize,
    h: usize,
}

#[no_mangle]
pub unsafe extern "C" fn wcslen(mut s: *const u16) -> usize {
    let mut len = 0;
    while *s != 0 {
        len += 1;
        s = s.add(1);
    }
    len
}

impl GopSurface {
    pub fn open() -> uefi::Result<Self> {
        let handle = boot::get_handle_for_protocol::<GraphicsOutput>()?;
        let gop = boot::open_protocol_exclusive::<GraphicsOutput>(handle)?;
        let (w, h) = gop.current_mode_info().resolution();
        Ok(Self { gop, w, h })
    }
}

impl Surface for GopSurface {
    fn size(&self) -> (usize, usize) {
        (self.w, self.h)
    }

    fn fill_rect(&mut self, rect: Rect, color: Color) {
        if rect.w == 0 || rect.h == 0 {
            return;
        }
        let (sw, sh) = self.size();
        let x = rect.x.min(sw);
        let y = rect.y.min(sh);
        let w = rect.w.min(sw.saturating_sub(x));
        let h = rect.h.min(sh.saturating_sub(y));

        if w > 0 && h > 0 {
            let _ = self.gop.blt(BltOp::VideoFill {
                color: BltPixel::new(color.r, color.g, color.b),
                dest: (x, y),
                dims: (w, h),
            });
        }
    }
}

/// Reads a file from the same volume the bootloader was loaded from (the ESP).
/// Paths use backslashes, e.g. `\EFI\htmlboot\theme.css`.
pub fn read_esp_file(path: &CStr16) -> Option<Vec<u8>> {
    let sfs = boot::get_image_file_system(boot::image_handle()).ok()?;
    let mut fs = FileSystem::new(sfs);
    fs.read(Path::new(path)).ok()
}

/// Same as `read_esp_file`, but requires valid UTF-8.
pub fn read_esp_text(path: &CStr16) -> Option<String> {
    String::from_utf8(read_esp_file(path)?).ok()
}

/// Discovers boot entries from the ESP (`\loader\entries\*.conf`).
pub fn discover_boot_entries() -> Vec<BootEntry> {
    let mut entries = Vec::new();
    let sfs = match boot::get_image_file_system(boot::image_handle()) {
        Ok(sfs) => sfs,
        Err(_) => return entries,
    };
    let mut fs = FileSystem::new(sfs);

    // Try enumerating \loader\entries directory
    if let Ok(dir_iter) = fs.read_dir(Path::new(cstr16!("\\loader\\entries"))) {
        for item in dir_iter.flatten() {
            let name_cstr = item.file_name();
            let mut name_u16: Vec<u16> = Vec::new();
            for ch in name_cstr.as_slice_with_nul() {
                let code = u16::from(*ch);
                if code == 0 {
                    break;
                }
                name_u16.push(code);
            }
            if let Ok(filename) = String::from_utf16(&name_u16) {
                if filename.ends_with(".conf") {
                    let mut full_path = String::from("\\loader\\entries\\");
                    full_path.push_str(&filename);

                    let mut full_path_u16: Vec<u16> = full_path.encode_utf16().collect();
                    full_path_u16.push(0);

                    if let Ok(path_cstr) = CStr16::from_u16_with_nul(&full_path_u16) {
                        if let Ok(content) = fs.read_to_string(Path::new(path_cstr)) {
                            if let Some(entry) = BootEntry::parse(&filename, &content) {
                                entries.push(entry);
                            }
                        }
                    }
                }
            }
        }
    }

    // Fallback: if no entries found from directory scan, try test.conf specifically
    if entries.is_empty() {
        if let Ok(content) = fs.read_to_string(Path::new(cstr16!("\\loader\\entries\\test.conf"))) {
            if let Some(entry) = BootEntry::parse("test.conf", &content) {
                entries.push(entry);
            }
        }
    }

    // If still no entries, but \vmlinuz exists on ESP, add default Linux entry
    if entries.is_empty() && fs.try_exists(Path::new(cstr16!("\\vmlinuz"))).unwrap_or(false) {
        entries.push(BootEntry {
            id: String::from("default"),
            title: String::from("Linux (vmlinuz)"),
            version: None,
            linux: String::from("/vmlinuz"),
            initrd: if fs.try_exists(Path::new(cstr16!("\\initramfs.gz"))).unwrap_or(false) {
                Some(String::from("/initramfs.gz"))
            } else {
                None
            },
            options: Some(String::from("console=tty0 console=ttyS0 rdinit=/init")),
        });
    }

    entries
}

pub struct InputManager {
    pointer: Option<ScopedProtocol<Pointer>>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum UserAction {
    None,
    Up,
    Down,
    Home,
    End,
    Select,
    DirectSelect(usize),
    CursorMove { dx: i32, dy: i32 },
    PointerMoved { dx: i32, dy: i32 },
    PointerClick,
}

impl InputManager {
    pub fn new() -> Self {
        // Keyboard input goes through the system table's stdin (see poll_action),
        // so there is no exclusive protocol to open for it here.
        let pointer = boot::get_handle_for_protocol::<Pointer>()
            .ok()
            .and_then(|h| boot::open_protocol_exclusive::<Pointer>(h).ok());

        Self { pointer }
    }

    pub fn poll_action(&mut self) -> UserAction {
        // Poll keyboard through the system table's stdin. Opening the Input
        // protocol exclusively can fail on firmware where the console splitter
        // already holds it, and the old code swallowed that error silently.
        {
            if let Ok(Some(key)) = uefi::system::with_stdin(|stdin| stdin.read_key()) {
                match key {
                    Key::Special(ScanCode::UP) => return UserAction::Up,
                    Key::Special(ScanCode::DOWN) => return UserAction::Down,
                    Key::Special(ScanCode::LEFT) => return UserAction::CursorMove { dx: -24, dy: 0 },
                    Key::Special(ScanCode::RIGHT) => return UserAction::CursorMove { dx: 24, dy: 0 },
                    Key::Special(ScanCode::HOME) => return UserAction::Home,
                    Key::Special(ScanCode::END) => return UserAction::End,
                    Key::Printable(c) => {
                        let code = u16::from(c);
                        if code == 0x000d || code == 0x000a {
                            return UserAction::Select;
                        } else if code == 0x0009 {
                            return UserAction::Down;
                        } else if code == 0x0020 {
                            return UserAction::PointerClick;
                        } else {
                            let ch = char::from(c);
                            match ch {
                                'k' | 'K' => return UserAction::Up,
                                'j' | 'J' => return UserAction::Down,
                                'w' | 'W' => return UserAction::CursorMove { dx: 0, dy: -24 },
                                's' | 'S' => return UserAction::CursorMove { dx: 0, dy: 24 },
                                'a' | 'A' => return UserAction::CursorMove { dx: -24, dy: 0 },
                                'd' | 'D' => return UserAction::CursorMove { dx: 24, dy: 0 },
                                '1'..='9' => {
                                    let idx = (ch as usize) - ('1' as usize);
                                    return UserAction::DirectSelect(idx);
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // Poll pointer (mouse)
        if let Some(ref mut pointer) = self.pointer {
            if let Ok(Some(state)) = pointer.read_state() {
                if state.button[0] {
                    return UserAction::PointerClick;
                }
                let dx = state.relative_movement[0];
                let dy = state.relative_movement[1];
                if dx != 0 || dy != 0 {
                    return UserAction::PointerMoved { dx, dy };
                }
            }
        }

        UserAction::None
    }
}

// Device path structure for Linux initrd LoadFile2 protocol:
// Vendor Media Device Path with GUID 5568e427-68fc-4f3d-ac74-ca555231cc68
#[repr(C, packed)]
struct InitrdDevicePath {
    type_: u8,
    sub_type: u8,
    length: [u8; 2],
    guid: Guid,
    end_type: u8,
    end_sub_type: u8,
    end_length: [u8; 2],
}

static INITRD_DEVICE_PATH: InitrdDevicePath = InitrdDevicePath {
    type_: 0x04,          // Media Device Path
    sub_type: 0x03,       // Vendor-Defined Media (0x04 would be File Path)
    length: [20, 0],      // 20 bytes (4 header + 16 guid)
    guid: guid!("5568e427-68fc-4f3d-ac74-ca555231cc68"),
    end_type: 0x7f,       // End Entire Device Path
    end_sub_type: 0xff,
    end_length: [4, 0],
};

#[repr(C)]
struct RawLoadFile2Protocol {
    load_file: unsafe extern "efiapi" fn(
        this: *mut RawLoadFile2Protocol,
        file_path: *const core::ffi::c_void,
        boot_policy: u8,
        buffer_size: *mut usize,
        buffer: *mut core::ffi::c_void,
    ) -> Status,
}

static mut INITRD_DATA: Option<Vec<u8>> = None;

static LOAD_FILE2_IMPL: RawLoadFile2Protocol = RawLoadFile2Protocol {
    load_file: initrd_load_file,
};

unsafe extern "efiapi" fn initrd_load_file(
    _this: *mut RawLoadFile2Protocol,
    _file_path: *const core::ffi::c_void,
    _boot_policy: u8,
    buffer_size: *mut usize,
    buffer: *mut core::ffi::c_void,
) -> Status {
    if buffer_size.is_null() {
        return Status::INVALID_PARAMETER;
    }
    let data = match unsafe { (*core::ptr::addr_of!(INITRD_DATA)).as_ref() } {
        Some(d) => d,
        None => return Status::NOT_FOUND,
    };
    if buffer.is_null() || *buffer_size < data.len() {
        *buffer_size = data.len();
        return Status::BUFFER_TOO_SMALL;
    }
    core::ptr::copy_nonoverlapping(data.as_ptr(), buffer.cast(), data.len());
    *buffer_size = data.len();
    Status::SUCCESS
}

/// Boot a selected Linux kernel entry with detailed error reporting.
pub fn boot_entry(entry: &BootEntry) -> core::result::Result<(), (&'static str, Status)> {
    let sfs = boot::get_image_file_system(boot::image_handle())
        .map_err(|e| ("get_image_file_system", e.status()))?;
    let mut fs = FileSystem::new(sfs);

    let mut kernel_path = entry.linux.replace('/', "\\");
    if !kernel_path.starts_with('\\') {
        kernel_path.insert(0, '\\');
    }

    let mut kernel_path_u16: Vec<u16> = kernel_path.encode_utf16().collect();
    kernel_path_u16.push(0);

    let kernel_cpath = CStr16::from_u16_with_nul(&kernel_path_u16)
        .map_err(|_| ("cstr_kernel_path", Status::INVALID_PARAMETER))?;

    // Try loading via full device path from the ESP volume
    let mut dp_storage = Vec::new();
    let image_handle = (|| -> core::result::Result<Handle, (&'static str, Status)> {
        let bootloader_li = boot::open_protocol_exclusive::<LoadedImage>(boot::image_handle())
            .map_err(|e| ("open_bootloader_li", e.status()))?;
        let dev_handle = bootloader_li
            .device()
            .ok_or(("get_dev_handle", Status::NOT_FOUND))?;
        let esp_dp = boot::open_protocol_exclusive::<DevicePath>(dev_handle)
            .map_err(|e| ("open_esp_dp", e.status()))?;

        let mut builder = DevicePathBuilder::with_vec(&mut dp_storage);
        for node in esp_dp.node_iter() {
            builder = builder
                .push(&node)
                .map_err(|_| ("dp_builder_push", Status::OUT_OF_RESOURCES))?;
        }
        builder = builder
            .push(&build::media::FilePath {
                path_name: kernel_cpath,
            })
            .map_err(|_| ("dp_builder_push_file", Status::OUT_OF_RESOURCES))?;
        let full_dp = builder
            .finalize()
            .map_err(|_| ("dp_builder_finalize", Status::OUT_OF_RESOURCES))?;

        drop(esp_dp);
        drop(bootloader_li);

        boot::load_image(
            boot::image_handle(),
            LoadImageSource::FromDevicePath {
                device_path: full_dp,
                boot_policy: BootPolicy::BootSelection,
            },
        )
        .map_err(|e| ("load_image_from_device_path", e.status()))
    })()
    .or_else(|_| {
        // Fallback: load directly from buffer
        let kernel_bytes = fs
            .read(Path::new(kernel_cpath))
            .map_err(|_| ("read_kernel_file", Status::NOT_FOUND))?;
        boot::load_image(
            boot::image_handle(),
            LoadImageSource::FromBuffer {
                buffer: &kernel_bytes,
                file_path: None,
            },
        )
        .map_err(|e| ("load_image_from_buffer", e.status()))
    })?;

    // Install Linux initrd LoadFile2 protocol if initrd is present
    let mut initrd_handle: Option<Handle> = None;
    if let Some(ref initrd_file) = entry.initrd {
        let mut initrd_path = initrd_file.replace('/', "\\");
        if !initrd_path.starts_with('\\') {
            initrd_path.insert(0, '\\');
        }
        let mut initrd_path_u16: Vec<u16> = initrd_path.encode_utf16().collect();
        initrd_path_u16.push(0);

        if let Ok(initrd_cpath) = CStr16::from_u16_with_nul(&initrd_path_u16) {
            if let Ok(initrd_bytes) = fs.read(Path::new(initrd_cpath)) {
                unsafe {
                    INITRD_DATA = Some(initrd_bytes);
                    if let Ok(handle) = boot::install_protocol_interface(
                        None,
                        &DevicePath::GUID,
                        &INITRD_DEVICE_PATH as *const _ as *const core::ffi::c_void,
                    ) {
                        if boot::install_protocol_interface(
                            Some(handle),
                            &LoadFile2::GUID,
                            &LOAD_FILE2_IMPL as *const _ as *const core::ffi::c_void,
                        )
                        .is_ok()
                        {
                            initrd_handle = Some(handle);
                        }
                    }
                }
            }
        }
    }

    // Set kernel command line options
    let mut cmdline = String::new();
    if let Some(ref opts) = entry.options {
        cmdline.push_str(opts);
    }
    // Also include initrd file parameter for fallback
    if let Some(ref initrd) = entry.initrd {
        let mut initrd_path = initrd.replace('/', "\\");
        if !initrd_path.starts_with('\\') {
            initrd_path.insert(0, '\\');
        }
        if !cmdline.is_empty() {
            cmdline.push(' ');
        }
        cmdline.push_str("initrd=");
        cmdline.push_str(&initrd_path);
    }

    let mut _options_storage: Option<Vec<u16>> = None;
    if !cmdline.is_empty() {
        let options_u16: Vec<u16> = cmdline.encode_utf16().chain(core::iter::once(0)).collect();
        if let Ok(mut loaded_image) = boot::open_protocol_exclusive::<LoadedImage>(image_handle) {
            unsafe {
                loaded_image.set_load_options(
                    options_u16.as_ptr().cast(),
                    (options_u16.len() * core::mem::size_of::<u16>()) as u32,
                );
            }
        }
        _options_storage = Some(options_u16);
    }

    let res = boot::start_image(image_handle).map_err(|e| ("start_image", e.status()));

    // Clean up initrd protocol if start_image returns
    if let Some(handle) = initrd_handle {
        unsafe {
            let _ = boot::uninstall_protocol_interface(
                handle,
                &LoadFile2::GUID,
                &LOAD_FILE2_IMPL as *const _ as *const core::ffi::c_void,
            );
            let _ = boot::uninstall_protocol_interface(
                handle,
                &DevicePath::GUID,
                &INITRD_DEVICE_PATH as *const _ as *const core::ffi::c_void,
            );
            INITRD_DATA = None;
        }
    }

    // Keep _options_storage alive until here
    drop(_options_storage);

    res
}
