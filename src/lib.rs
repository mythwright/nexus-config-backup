use std::ffi::{c_char, c_ulong, CStr};
use std::mem::MaybeUninit;
use std::ptr::NonNull;
use nexus_rs::raw_structs::{AddonAPI, AddonDefinition, AddonVersion, EAddonFlags, ELogLevel, LPVOID};
use walkdir::WalkDir;
use windows::core::s;
use windows::Win32::Foundation::{HINSTANCE};
use windows::Win32::System::SystemServices;

#[no_mangle]
unsafe extern "C" fn DllMain(
    _hinst_dll: HINSTANCE,
    fdw_reason: c_ulong,
    _lpv_reserveded: LPVOID,
) -> bool {
    match fdw_reason {
        SystemServices::DLL_PROCESS_ATTACH => {}
        _ => {}
    }
    true
}

static mut API: MaybeUninit<&'static AddonAPI> = MaybeUninit::uninit();

unsafe extern "C" fn load(api: *mut AddonAPI) {
    let api = &*api;
    API.write(api);


    let dir = CStr::from_ptr((api.get_addon_directory)(s!("").0 as _))
        .to_str()
        .unwrap()
        .to_string();

    for e in WalkDir::new(dir) {
        log(ELogLevel::DEBUG, e.unwrap().path().display().to_string());
    }
    
}

pub fn log(level: ELogLevel, s: String) {
    unsafe {
        let api = API.assume_init();
        (api.log)(
            level,
            (s + "\0").as_ptr() as _,
        );
    }
}

unsafe extern "C" fn unload() {}

#[no_mangle]
pub extern "C" fn GetAddonDef() -> *mut AddonDefinition {
    static AD: AddonDefinition = AddonDefinition {
        signature: -32410,
        apiversion: nexus_rs::raw_structs::NEXUS_API_VERSION,
        name: b"Addon Config Backup Tool\0".as_ptr() as *const c_char,
        version: AddonVersion {
            major: 0,
            minor: 0,
            build: 1,
            revision: 0,
        },
        author: s!("Zyian").0 as _,
        description: s!("A small tool to help keep your addons backed up in case of nuking your GW2 install folder").0 as _,
        load,
        unload: Some(unsafe { NonNull::new_unchecked(unload as _) }),
        flags: EAddonFlags::None,
        provider: nexus_rs::raw_structs::EUpdateProvider::GitHub,
        update_link: Some(unsafe {
            NonNull::new_unchecked(s!("https://github.com/mythwright/nexus-config-backup").0 as _)
        }),
    };

    &AD as *const _ as _
}