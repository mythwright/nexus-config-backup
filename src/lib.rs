use std::ffi::{c_char, c_ulong, c_void, CStr};
use std::fs::File;
use std::io::{Read, Write};
use std::mem::MaybeUninit;
use std::{fs, ptr};
use std::ptr::NonNull;
use arcdps_imgui::{Context, Ui};
use arcdps_imgui::sys::{igSetAllocatorFunctions, igSetCurrentContext};
use nexus_rs::raw_structs::{AddonAPI, AddonDefinition, AddonVersion, EAddonFlags, ELogLevel, LPVOID};
use walkdir::WalkDir;
use windows::core::{PCSTR, s};
use windows::Win32::Foundation::{HINSTANCE};
use windows::Win32::System::SystemServices;
use zip::write::FileOptions;

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
static mut CTX: MaybeUninit<Context> = MaybeUninit::uninit();
static mut UI: MaybeUninit<Ui> = MaybeUninit::uninit();
static SHORTCUT_ID: &str = "QAS_CONFIG_BACKUP";

unsafe extern "C" fn load(api: *mut AddonAPI) {
    let api = &*api;
    API.write(api);

    igSetCurrentContext(api.imgui_context);
    igSetAllocatorFunctions(
        Some(api.imgui_malloc),
        Some(api.imgui_free),
        ptr::null::<c_void>() as *mut _,
    );

    CTX.write(Context::current());
    UI.write(Ui::from_ctx(CTX.assume_init_ref()));


    (api.add_simple_shortcut)(convert_string(SHORTCUT_ID), addon_shortcut);
}

pub fn convert_string(s: &str) -> *const c_char {
    let a = PCSTR::from_raw((s.to_owned() + "\0").as_ptr());
    a.as_ptr() as *const c_char
}

pub unsafe fn run_backup() {
    let dir = CStr::from_ptr((API.assume_init().get_addon_directory)(convert_string("")))
        .to_str()
        .unwrap()
        .to_string();
    let wd = WalkDir::new(dir.clone());
    let wd_it = wd.into_iter().filter_entry(|e| {
        if e.file_type().is_dir() && e.file_name().to_str().unwrap().contains("common") {
            return false;
        }
        return true;
    });

    let docs_dir = dirs_next::document_dir().unwrap();
    let backup_dir = docs_dir.join("nexus-configs");
    if !backup_dir.exists() {
        fs::create_dir_all(&backup_dir).unwrap();
    }
    let local_time = chrono::Local::now();
    let backup_file = match File::create(backup_dir.join(format!("backup-{}.zip", local_time.format("%Y-%m-%d-%H-%M")))) {
        Ok(b) => b,
        Err(err) => {
            log(ELogLevel::CRITICAL, format!("Failed to create file {err}").to_string());
            return;
        }
    };

    let mut zip = zip::ZipWriter::new(backup_file);
    let mut buffer = Vec::new();

    let options = FileOptions::default();

    for e in wd_it {
        let entry = e.unwrap();
        let path = entry.path();
        let name = path.strip_prefix(dir.clone()).unwrap();

        if path.is_file() {
            if name.to_str().unwrap().to_string().contains(".dll") {
                // log(ELogLevel::DEBUG, format!("Skipping {path:?}...").to_string());
                continue;
            }
            #[allow(deprecated)]
            zip.start_file_from_path(name, options).unwrap();
            let mut f = File::open(path).unwrap();

            f.read_to_end(&mut buffer).unwrap();
            zip.write_all(&buffer).unwrap();
            buffer.clear();

            // log(ELogLevel::DEBUG, format!("Adding {path:?} as {name:?}...").to_string());
        } else if !name.as_os_str().is_empty() {
            // log(ELogLevel::DEBUG, format!("Adding dir {path:?} as {name:?}...").to_string());
            #[allow(deprecated)]
            zip.add_directory_from_path(name, options).unwrap();
        }
    }
    zip.finish().unwrap();
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

pub unsafe extern "C" fn addon_shortcut() {
    let ui = UI.assume_init_ref();

    ui.separator();
    ui.text_disabled("Config Backup");
    let clicked = ui.button("Run Backup");
    if clicked {
        run_backup();
    }
}

unsafe extern "C" fn unload() {
    (API.assume_init().remove_simple_shortcut)(convert_string(SHORTCUT_ID));
}

#[no_mangle]
pub extern "C" fn GetAddonDef() -> *mut AddonDefinition {
    static AD: AddonDefinition = AddonDefinition {
        signature: -50602,
        apiversion: nexus_rs::raw_structs::NEXUS_API_VERSION,
        name: b"Addon Config Backup Tool\0".as_ptr() as *const c_char,
        version: AddonVersion {
            major: 0,
            minor: 1,
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