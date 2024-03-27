use std::{fs, thread};
use std::fs::File;
use std::io::{Read, Write};
use nexus::{AddonFlags, log, paths, render, UpdateProvider};
use nexus::gui::RawGuiRender;
use nexus::quick_access::add_simple_shortcut;
use walkdir::WalkDir;
use zip::write::FileOptions;

static SHORTCUT_ID: &str = "QAS_CONFIG_BACKUP";

nexus::export! {
    name: "Addon Config Backup",
    signature: -50602,
    load,
    unload,
    flags: AddonFlags::None,
    provider: UpdateProvider::GitHub,
    update_link: "https://github.com/mythwright/nexus-config-backup",
}

fn load() {
    add_simple_shortcut(SHORTCUT_ID, addon_shortcut()).revert_on_unload();
}

fn unload() {}

pub fn run_backup() -> bool {
    let _ = thread::spawn(|| {
        let dir = paths::get_addon_dir("").unwrap();
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
                log::log(
                    log::LogLevel::Critical,
                    "addon-config-backup",
                    format!("Failed to create file {err}"),
                );
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
    });
    true
}

fn addon_shortcut() -> RawGuiRender {
    render!(|ui| {
        ui.separator();
        let clicked = ui.button("Run Backup");
        if clicked {
            if run_backup() {
                log::log(
                    log::LogLevel::Info,
                    "Addon Config Backup",
                    "Finished saving backup to nexus-configs folder",
                );
            }
        }
    })
}