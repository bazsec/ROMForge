//! Single-instance, synchronous Libretro development host.
//! Native cores are trusted code. Loading an arbitrary library executes it.
use anyhow::{Result, ensure};
use libloading::Library;
use std::{
    cell::RefCell,
    collections::HashMap,
    ffi::{CStr, CString, c_char, c_void},
    marker::PhantomData,
    path::Path,
    rc::Rc,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

#[repr(C)]
struct SystemInfo {
    name: *const c_char,
    version: *const c_char,
    extensions: *const c_char,
    need_fullpath: bool,
    block_extract: bool,
}
#[repr(C)]
struct GameInfo {
    path: *const c_char,
    data: *const c_void,
    size: usize,
    meta: *const c_char,
}
static ACTIVE: AtomicBool = AtomicBool::new(false);
static FRAMES: AtomicUsize = AtomicUsize::new(0);
static SAMPLES: AtomicUsize = AtomicUsize::new(0);

#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct Geometry {
    pub base_width: u32,
    pub base_height: u32,
    pub max_width: u32,
    pub max_height: u32,
    pub aspect_ratio: f32,
}
#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct Timing {
    pub fps: f64,
    pub sample_rate: f64,
}
#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct AvInfo {
    pub geometry: Geometry,
    pub timing: Timing,
}
#[repr(C)]
struct Variable {
    key: *const c_char,
    value: *const c_char,
}

pub struct VideoFrame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}
struct Host {
    format: u32,
    system_directory: CString,
    video: VideoFrame,
    audio: Vec<i16>,
    buttons: u16,
    variables: HashMap<Vec<u8>, CString>,
    geometry: Option<Geometry>,
    error: Option<&'static str>,
}
impl Default for Host {
    fn default() -> Self {
        Self {
            format: 0,
            system_directory: CString::new(".").unwrap(),
            video: VideoFrame {
                width: 0,
                height: 0,
                rgba: Vec::with_capacity(1024 * 1024 * 4),
            },
            audio: Vec::with_capacity(32768),
            buttons: 0,
            variables: HashMap::new(),
            geometry: None,
            error: None,
        }
    }
}
thread_local! { static HOST: RefCell<Host> = RefCell::new(Host::default()); }

unsafe extern "C" fn environment(command: u32, data: *mut c_void) -> bool {
    if data.is_null() {
        return false;
    }
    HOST.with(|host| {
        let Ok(mut host) = host.try_borrow_mut() else {
            return false;
        };
        // SAFETY: trusted Libretro cores provide the documented type for each command.
        unsafe {
            match command {
                3 => {
                    *data.cast::<bool>() = true;
                    true
                } // retain duplicate video frames
                9 => {
                    *data.cast::<*const c_char>() = host.system_directory.as_ptr();
                    true
                }
                10 => {
                    let format = *data.cast::<u32>();
                    if format > 2 {
                        return false;
                    }
                    host.format = format;
                    true
                }
                15 => {
                    let variable = &mut *data.cast::<Variable>();
                    if variable.key.is_null() {
                        return false;
                    }
                    variable.value = host
                        .variables
                        .get(CStr::from_ptr(variable.key).to_bytes())
                        .map_or(std::ptr::null(), |v| v.as_ptr());
                    !variable.value.is_null()
                }
                16 => {
                    // Legacy options: "Description; default|alternative". Keep owned C strings.
                    for i in 0..4096 {
                        let variable = &*data.cast::<Variable>().add(i);
                        if variable.key.is_null() {
                            return true;
                        }
                        if variable.value.is_null() {
                            return false;
                        }
                        let text = CStr::from_ptr(variable.value).to_string_lossy();
                        if let Some((_, choices)) = text.split_once(';')
                            && let Ok(value) =
                                CString::new(choices.trim().split('|').next().unwrap_or(""))
                        {
                            host.variables
                                .insert(CStr::from_ptr(variable.key).to_bytes().to_vec(), value);
                        }
                    }
                    false
                }
                18 => {
                    *data.cast::<bool>() = false;
                    true
                }
                37 => {
                    host.geometry = Some(*data.cast::<Geometry>());
                    true
                }
                52 => {
                    *data.cast::<u32>() = 0;
                    true
                } // legacy core options
                11 => true, // input descriptions are not displayed
                _ => false,
            }
        }
    })
}

/// Decode one pixel to RGBA bytes, independent of the host's endianness.
fn rgba(pixel: u32, format: u32) -> [u8; 4] {
    if format == 1 {
        return [(pixel >> 16) as u8, (pixel >> 8) as u8, pixel as u8, 255];
    }
    let (r, g, b) = if format == 2 {
        ((pixel >> 11) & 31, (pixel >> 5) & 63, pixel & 31)
    } else {
        ((pixel >> 10) & 31, (pixel >> 5) & 31, pixel & 31)
    };
    [
        ((r << 3) | (r >> 2)) as u8,
        if format == 2 {
            ((g << 2) | (g >> 4)) as u8
        } else {
            ((g << 3) | (g >> 2)) as u8
        },
        ((b << 3) | (b >> 2)) as u8,
        255,
    ]
}
unsafe extern "C" fn video(data: *const c_void, width: u32, height: u32, pitch: usize) {
    if data.is_null() {
        return;
    }
    HOST.with(|host| {
        let Ok(mut host) = host.try_borrow_mut() else {
            return;
        };
        if data as usize == usize::MAX || width == 0 || height == 0 || width > 1024 || height > 1024
        {
            host.error = Some("invalid or unsupported video frame");
            return;
        }
        let bytes = if host.format == 1 { 4 } else { 2 };
        if pitch < width as usize * bytes || pitch > 1024 * 16 {
            host.error = Some("invalid video pitch");
            return;
        }
        let format = host.format;
        host.video
            .rgba
            .resize(width as usize * height as usize * 4, 0);
        for y in 0..height as usize {
            for x in 0..width as usize {
                // SAFETY: the trusted core supplies pitch * height readable bytes for this callback.
                let pixel = unsafe {
                    let ptr = data.cast::<u8>().add(y * pitch + x * bytes);
                    if bytes == 4 {
                        ptr.cast::<u32>().read_unaligned()
                    } else {
                        ptr.cast::<u16>().read_unaligned() as u32
                    }
                };
                let offset = (y * width as usize + x) * 4;
                host.video.rgba[offset..offset + 4].copy_from_slice(&rgba(pixel, format));
            }
        }
        host.video.width = width;
        host.video.height = height;
        FRAMES.fetch_add(1, Ordering::Relaxed);
    });
}
unsafe extern "C" fn audio(left: i16, right: i16) {
    HOST.with(|h| {
        if let Ok(mut host) = h.try_borrow_mut()
            && host.audio.len() + 2 <= 32768
        {
            host.audio.extend_from_slice(&[left, right]);
        }
    });
    SAMPLES.fetch_add(1, Ordering::Relaxed);
}
unsafe extern "C" fn audio_batch(data: *const i16, frames: usize) -> usize {
    if data.is_null() || frames > 16384 {
        return 0;
    }
    HOST.with(|h| {
        if let Ok(mut host) = h.try_borrow_mut()
            && host.audio.len() + frames * 2 <= 32768
        {
            // SAFETY: Libretro supplies frames * 2 interleaved samples valid during this callback.
            host.audio
                .extend_from_slice(unsafe { std::slice::from_raw_parts(data, frames * 2) });
        }
    });
    SAMPLES.fetch_add(frames, Ordering::Relaxed);
    frames
}
unsafe extern "C" fn poll() {}
unsafe extern "C" fn input(port: u32, device: u32, index: u32, id: u32) -> i16 {
    if port != 0 || device != 1 || index != 0 || id >= 16 {
        return 0;
    }
    HOST.with(|h| h.try_borrow().map_or(0, |h| ((h.buttons >> id) & 1) as i16))
}

pub struct Statistics {
    pub video_frames: usize,
    pub audio_frames: usize,
}
/// A session keeps the ROM allocation and native library alive until unload.
/// Rc marker prevents moving a core to another thread or sharing it.
pub struct Session {
    library: Library,
    _rom: Vec<u8>,
    _path: CString,
    initialized: bool,
    loaded: bool,
    _thread: PhantomData<Rc<()>>,
}
impl Session {
    /// # Safety
    /// The library must be a trusted, ABI-compatible Libretro core with synchronous callbacks.
    pub unsafe fn load(path: &Path, rom: Vec<u8>, extension: &str) -> Result<Self> {
        ensure!(
            !extension.contains(['/', '\\', '\0']),
            "invalid content extension"
        );
        ensure!(
            !ACTIVE.swap(true, Ordering::SeqCst),
            "only one Libretro session is allowed per process"
        );
        // Library load failure must release the global session guard.
        let library = match unsafe { Library::new(path) } {
            Ok(l) => l,
            Err(e) => {
                ACTIVE.store(false, Ordering::SeqCst);
                return Err(e.into());
            }
        };
        let mut session = Self {
            library,
            _rom: rom,
            _path: CString::new(format!("game.{extension}"))?,
            initialized: false,
            loaded: false,
            _thread: PhantomData,
        };
        let core_directory = std::fs::canonicalize(path)?
            .parent()
            .unwrap_or(Path::new("."))
            .to_path_buf();
        let resources = core_directory
            .parent()
            .unwrap_or(&core_directory)
            .join("Resources");
        let system_directory = if resources.is_dir() {
            resources
        } else {
            core_directory
        };
        let system_directory = CString::new(system_directory.as_os_str().as_encoded_bytes())?;
        HOST.with(|host| {
            *host.borrow_mut() = Host {
                system_directory,
                ..Host::default()
            }
        });
        unsafe {
            let api = session
                .library
                .get::<unsafe extern "C" fn() -> u32>(b"retro_api_version\0")?;
            ensure!(api() == 1, "unsupported Libretro API");
            // Resolve teardown and execution symbols before initialization.
            session
                .library
                .get::<unsafe extern "C" fn()>(b"retro_deinit\0")?;
            session
                .library
                .get::<unsafe extern "C" fn()>(b"retro_unload_game\0")?;
            session
                .library
                .get::<unsafe extern "C" fn()>(b"retro_run\0")?;
            let mut info = SystemInfo {
                name: std::ptr::null(),
                version: std::ptr::null(),
                extensions: std::ptr::null(),
                need_fullpath: false,
                block_extract: false,
            };
            session
                .library
                .get::<unsafe extern "C" fn(*mut SystemInfo)>(b"retro_get_system_info\0")?(
                &mut info,
            );
            ensure!(
                !info.need_fullpath,
                "core requires a filesystem ROM; in-memory-only host refuses extraction"
            );
            session
                .library
                .get::<unsafe extern "C" fn(unsafe extern "C" fn(u32, *mut c_void) -> bool)>(
                    b"retro_set_environment\0",
                )?(environment);
            session.library.get::<unsafe extern "C" fn(
                unsafe extern "C" fn(*const c_void, u32, u32, usize),
            )>(b"retro_set_video_refresh\0")?(video);
            session
                .library
                .get::<unsafe extern "C" fn(unsafe extern "C" fn(i16, i16))>(
                    b"retro_set_audio_sample\0",
                )?(audio);
            session.library.get::<unsafe extern "C" fn(
                unsafe extern "C" fn(*const i16, usize) -> usize,
            )>(b"retro_set_audio_sample_batch\0")?(audio_batch);
            session
                .library
                .get::<unsafe extern "C" fn(unsafe extern "C" fn())>(b"retro_set_input_poll\0")?(
                poll,
            );
            session
                .library
                .get::<unsafe extern "C" fn(unsafe extern "C" fn(u32, u32, u32, u32) -> i16)>(
                    b"retro_set_input_state\0",
                )?(input);
            FRAMES.store(0, Ordering::Relaxed);
            SAMPLES.store(0, Ordering::Relaxed);
            session
                .library
                .get::<unsafe extern "C" fn()>(b"retro_init\0")?();
            session.initialized = true;
            let game = GameInfo {
                path: session._path.as_ptr(),
                data: session._rom.as_ptr().cast(),
                size: session._rom.len(),
                meta: std::ptr::null(),
            };
            ensure!(
                session
                    .library
                    .get::<unsafe extern "C" fn(*const GameInfo) -> bool>(b"retro_load_game\0")?(
                    &game
                ),
                "core rejected ROM"
            );
            session.loaded = true;
            if let Ok(set_device) = session
                .library
                .get::<unsafe extern "C" fn(u32, u32)>(b"retro_set_controller_port_device\0")
            {
                set_device(0, 1);
            }
        }
        Ok(session)
    }
    pub fn av_info(&self) -> Result<AvInfo> {
        let mut info = AvInfo::default();
        // SAFETY: API v1 function fills a repr(C) output structure.
        unsafe {
            self.library
                .get::<unsafe extern "C" fn(*mut AvInfo)>(b"retro_get_system_av_info\0")?(
                &mut info,
            );
        }
        if let Some(geometry) = HOST.with(|h| h.borrow().geometry) {
            info.geometry = geometry;
        }
        ensure!(
            info.timing.fps.is_finite() && (20.0..=240.0).contains(&info.timing.fps),
            "invalid core frame rate"
        );
        ensure!(
            info.timing.sample_rate.is_finite()
                && (8000.0..=192000.0).contains(&info.timing.sample_rate),
            "invalid core audio rate"
        );
        Ok(info)
    }
    pub fn set_buttons(&mut self, buttons: u16) {
        HOST.with(|h| h.borrow_mut().buttons = buttons);
    }
    pub fn output<T>(&self, read: impl FnOnce(&VideoFrame, &[i16]) -> Result<T>) -> Result<T> {
        HOST.with(|h| {
            let host = h.borrow();
            if let Some(error) = host.error {
                anyhow::bail!(error);
            }
            read(&host.video, &host.audio)
        })
    }
    pub fn save_state(&self) -> Result<Vec<u8>> {
        // SAFETY: the core is loaded and idle; it receives a buffer of its requested size.
        unsafe {
            let len = self
                .library
                .get::<unsafe extern "C" fn() -> usize>(b"retro_serialize_size\0")?(
            );
            ensure!(
                len > 0 && len <= 64 * 1024 * 1024,
                "Save states are unavailable for this game."
            );
            let mut bytes = vec![0; len];
            ensure!(
                self.library
                    .get::<unsafe extern "C" fn(*mut c_void, usize) -> bool>(
                        b"retro_serialize\0"
                    )?(bytes.as_mut_ptr().cast(), len),
                "Could not save the current state."
            );
            Ok(bytes)
        }
    }
    pub fn load_state(&mut self, bytes: &[u8]) -> Result<()> {
        ensure!(
            !bytes.is_empty() && bytes.len() <= 64 * 1024 * 1024,
            "Invalid save state size."
        );
        let backup = self.save_state()?;
        // SAFETY: synchronous restore receives a valid bounded slice. Restore backup on rejection.
        unsafe {
            let restore = self
                .library
                .get::<unsafe extern "C" fn(*const c_void, usize) -> bool>(
                    b"retro_unserialize\0",
                )?;
            if !restore(bytes.as_ptr().cast(), bytes.len()) {
                ensure!(
                    restore(backup.as_ptr().cast(), backup.len()),
                    "State load failed and the previous state could not be restored. Restart the game."
                );
                anyhow::bail!("This save state could not be loaded.");
            }
        }
        Ok(())
    }
    pub fn save_ram(&self) -> Result<Vec<u8>> {
        // SAFETY: the memory buffer is borrowed only while the core is idle and loaded.
        unsafe {
            let len = self
                .library
                .get::<unsafe extern "C" fn(u32) -> usize>(b"retro_get_memory_size\0")?(
                0
            );
            ensure!(len <= 16 * 1024 * 1024, "invalid save RAM size");
            if len == 0 {
                return Ok(Vec::new());
            }
            let data = self
                .library
                .get::<unsafe extern "C" fn(u32) -> *mut c_void>(b"retro_get_memory_data\0")?(
                0
            );
            ensure!(!data.is_null(), "missing save RAM");
            Ok(std::slice::from_raw_parts(data.cast::<u8>(), len).to_vec())
        }
    }
    pub fn restore_ram(&mut self, bytes: &[u8]) -> Result<()> {
        // SAFETY: size is checked against the core-owned writable SRAM allocation.
        unsafe {
            let len = self
                .library
                .get::<unsafe extern "C" fn(u32) -> usize>(b"retro_get_memory_size\0")?(
                0
            );
            ensure!(
                len == bytes.len(),
                "save RAM size differs from the current core"
            );
            if len == 0 {
                return Ok(());
            }
            let data = self
                .library
                .get::<unsafe extern "C" fn(u32) -> *mut c_void>(b"retro_get_memory_data\0")?(
                0
            );
            ensure!(!data.is_null(), "missing save RAM");
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), data.cast::<u8>(), len);
        }
        Ok(())
    }
    pub fn run_frames(&mut self, count: usize) -> Result<Statistics> {
        // SAFETY: load validated the ABI; ROM/library remain alive and Session is single-threaded.
        unsafe {
            let run = self.library.get::<unsafe extern "C" fn()>(b"retro_run\0")?;
            for _ in 0..count {
                HOST.with(|host| host.borrow_mut().audio.clear());
                run();
            }
        }
        Ok(Statistics {
            video_frames: FRAMES.load(Ordering::Relaxed),
            audio_frames: SAMPLES.load(Ordering::Relaxed),
        })
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        // SAFETY: symbols were resolved before initialization; teardown precedes library/ROM drop.
        unsafe {
            if self.loaded
                && let Ok(f) = self
                    .library
                    .get::<unsafe extern "C" fn()>(b"retro_unload_game\0")
            {
                f();
            }
            if self.initialized
                && let Ok(f) = self
                    .library
                    .get::<unsafe extern "C" fn()>(b"retro_deinit\0")
            {
                f();
            }
        }
        ACTIVE.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decodes_all_software_formats() {
        assert_eq!(rgba(0xf800, 2), [255, 0, 0, 255]);
        assert_eq!(rgba(0x03e0, 0), [0, 255, 0, 255]);
        assert_eq!(rgba(0x123456, 1), [0x12, 0x34, 0x56, 255]);
        assert_eq!(rgba(0xffff, 2), [255, 255, 255, 255]);
    }
    #[test]
    fn video_honors_pitch_and_null_duplicates() {
        HOST.with(|h| *h.borrow_mut() = Host::default());
        let pixels: [u16; 4] = [0x7c00, 0, 0x001f, 0];
        unsafe {
            video(pixels.as_ptr().cast(), 1, 2, 4);
            video(std::ptr::null(), 1, 2, 4);
        }
        HOST.with(|h| assert_eq!(h.borrow().video.rgba, [255, 0, 0, 255, 0, 0, 255, 255]));
    }
}
