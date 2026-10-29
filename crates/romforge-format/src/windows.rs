//! Windows executable resources. Inspect files as data, never execute them.
use anyhow::{Context, Result, ensure};
use std::{os::windows::ffi::OsStrExt, path::Path, ptr};
use windows_sys::Win32::{Foundation::FreeLibrary, System::LibraryLoader::*};

pub const PAYLOAD: u16 = 101;
pub const CORE: u16 = 102;
pub const BUILD: u16 = 103;
pub const NOTICES: u16 = 104;
pub const DATA: u16 = 10;
pub const ICON: u16 = 3;
pub const GROUP_ICON: u16 = 14;
const MAX_RESOURCE: usize = 128 * 1024 * 1024;

fn wide(path: &Path) -> Result<Vec<u16>> {
    let mut value: Vec<u16> = path.as_os_str().encode_wide().collect();
    ensure!(!value.contains(&0), "Path contains a NUL character");
    value.push(0);
    Ok(value)
}
unsafe fn copy_resource(
    module: windows_sys::Win32::Foundation::HMODULE,
    kind: u16,
    id: u16,
) -> Result<Vec<u8>> {
    // SAFETY: integer resource identifiers are represented by low-valued pointers.
    let resource = unsafe { FindResourceW(module, id as usize as _, kind as usize as _) };
    ensure!(!resource.is_null(), "Executable resource {id} is missing");
    let size = unsafe { SizeofResource(module, resource) } as usize;
    ensure!(
        size > 0 && size <= MAX_RESOURCE,
        "Invalid executable resource size"
    );
    let loaded = unsafe { LoadResource(module, resource) };
    ensure!(!loaded.is_null(), "Could not load executable resource");
    let bytes = unsafe { LockResource(loaded) } as *const u8;
    ensure!(!bytes.is_null(), "Could not read executable resource");
    // The module owns this memory; copy it before unloading the file mapping.
    Ok(unsafe { std::slice::from_raw_parts(bytes, size) }.to_vec())
}
pub fn current(id: u16) -> Result<Vec<u8>> {
    let module = unsafe { GetModuleHandleW(ptr::null()) };
    ensure!(!module.is_null(), "Current executable unavailable");
    unsafe { copy_resource(module, DATA, id) }
}
pub fn read(path: &Path, kind: u16, id: u16) -> Result<Vec<u8>> {
    let path = wide(path)?;
    let module = unsafe {
        LoadLibraryExW(
            path.as_ptr(),
            ptr::null_mut(),
            LOAD_LIBRARY_AS_DATAFILE | LOAD_LIBRARY_AS_IMAGE_RESOURCE,
        )
    };
    ensure!(
        !module.is_null(),
        "Could not inspect executable: {}",
        std::io::Error::last_os_error()
    );
    let result = unsafe { copy_resource(module, kind, id) };
    unsafe { FreeLibrary(module) };
    result
}
pub fn embed(path: &Path, resources: &[(u16, u16, Vec<u8>)]) -> Result<()> {
    let path = wide(path)?;
    let handle = unsafe { BeginUpdateResourceW(path.as_ptr(), 0) };
    ensure!(
        !handle.is_null(),
        "Could not update executable resources: {}",
        std::io::Error::last_os_error()
    );
    for (kind, id, bytes) in resources {
        let result = (|| -> Result<()> {
            ensure!(
                !bytes.is_empty() && bytes.len() <= MAX_RESOURCE,
                "Invalid executable resource size"
            );
            let length = u32::try_from(bytes.len()).context("Resource too large")?;
            ensure!(
                unsafe {
                    UpdateResourceW(
                        handle,
                        *kind as usize as _,
                        *id as usize as _,
                        0,
                        bytes.as_ptr().cast(),
                        length,
                    )
                } != 0,
                "Could not embed resource: {}",
                std::io::Error::last_os_error()
            );
            Ok(())
        })();
        if let Err(error) = result {
            unsafe { EndUpdateResourceW(handle, 1) };
            return Err(error);
        }
    }
    ensure!(
        unsafe { EndUpdateResourceW(handle, 0) } != 0,
        "Could not commit executable resources: {}",
        std::io::Error::last_os_error()
    );
    Ok(())
}
