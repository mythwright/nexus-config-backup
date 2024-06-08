use std::{
    fmt::Error,
    fs,
    io::{Read, Write},
    ptr::addr_of_mut,
    thread,
};
use std::fs::File;
use std::path::PathBuf;

use nexus::{AddonFlags, log, paths, render, UpdateProvider};
use nexus::alert::alert_notify;
use nexus::gui::{RawGuiRender, register_render, RenderType};
use nexus::imgui::InputInt;
use nexus::quick_access::add_simple_shortcut;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;

static SHORTCUT_ID: &str = "QAS_CONFIG_BACKUP";

struct ConfigBackup {
    pub backup_folder: Option<PathBuf>,
    settings: Option<Settings>,
}

impl ConfigBackup {
    fn new() -> ConfigBackup {
        ConfigBackup { backup_folder: None, settings: None }
    }

    fn init(&mut self) -> bool {
        self.backup_folder = dirs_next::document_dir();

        let config_path = paths::get_addon_dir("addon-config-backup").unwrap();
        let config_path = ConfigBackup::get_or_init_folder(&config_path).join("config.toml");
        if !config_path.exists() {
            {
                let res = File::create(config_path.clone());
                if res.is_err() {
                    log::log(
                        log::LogLevel::Critical,
                        "addon-config-backup",
                        format!("Failed to create file {:?}", res.err().unwrap()),
                    );
                    return false;
                }
                let s = Settings::default();
                res.ok().unwrap().write_all(toml::to_string_pretty(&s).unwrap().as_bytes()).unwrap();
            }
        }
        let mut content = String::new();
        File::open(config_path).unwrap().read_to_string(&mut content).unwrap();

        let res: Result<Settings, toml::de::Error> = toml::from_str(content.as_str());
        if res.is_err() {
            log::log(
                log::LogLevel::Critical,
                "addon-config-backup",
                format!("Failed to read file {:?}", res.err().unwrap()),
            );
            return false;
        }
        self.settings = Some(res.ok().unwrap());
        true
    }

    fn get_or_init_folder(target: &PathBuf) -> &PathBuf {
        if !target.exists() {
            fs::create_dir_all(&target).unwrap();
        }
        target
    }

    fn backup_location(&self) -> String {
        self.backup_folder.clone().unwrap().to_str().unwrap().to_string()
    }
    
    fn save(&mut self) {
        let config_path = paths::get_addon_dir("addon-config-backup").unwrap();
        let config_path = ConfigBackup::get_or_init_folder(&config_path).join("config.toml");
        let mut config = File::open(config_path).unwrap();
        config.write_all(toml::to_string_pretty(self.settings.as_mut().unwrap()).unwrap().as_bytes()).unwrap();
    }
}

#[derive(Deserialize, Serialize)]
struct Settings {
    pub target_folder: String,
    pub backup_on_launch: bool,
    pub delete_old_on_launch: bool,
    pub backups_to_keep: u8,
}

impl Settings {
    fn default() -> Settings {
        Settings {
            target_folder: dirs_next::document_dir()
                .unwrap()
                .join("nexus-configs")
                .to_str()
                .unwrap()
                .to_string(),
            backup_on_launch: false,
            delete_old_on_launch: false,
            backups_to_keep: 5,
        }
    }
}

static mut GLOBAL_CONFIG: Lazy<ConfigBackup> = Lazy::new(ConfigBackup::new);

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
    grab_global().init();

    add_simple_shortcut(SHORTCUT_ID, addon_shortcut()).revert_on_unload();
    register_render(RenderType::OptionsRender, render!(render_options)).revert_on_unload();
}

fn unload() {
    grab_global().save();
}

fn render_options(ui: &nexus::imgui::Ui) {
    let g = grab_global();

    ui.text("General Settings");
    ui.separator();
    ui.input_text("Destination Folder", &mut g.backup_location()).read_only(true).build();
    ui.checkbox("Backup Settings on Game Launch", &mut g.settings.as_mut().unwrap().backup_on_launch);

    ui.text("Background Tasks");
    ui.separator();
    ui.checkbox("Automatically delete old backups",&mut g.settings.as_mut().unwrap().delete_old_on_launch);
    InputInt::new(ui, "Backups to Keep", &mut i32::from(g.settings.as_mut().unwrap().backups_to_keep)).build();
}

fn grab_global() -> &'static mut Lazy<ConfigBackup> {
    unsafe { &mut *addr_of_mut!(GLOBAL_CONFIG) }
}


pub fn run_backup() -> Result<(), Error> {
    let _ = thread::spawn(|| {
        let dir = paths::get_addon_dir("").unwrap();
        let wd = WalkDir::new(dir.clone());
        let wd_it = wd.into_iter().filter_entry(|e| {
            if e.file_type().is_dir() && e.file_name().to_str().unwrap().contains("common") {
                return false;
            }
            return true;
        });

        let backup_dir = dirs_next::document_dir().unwrap().join("nexus-configs");
        let backup_dir = ConfigBackup::get_or_init_folder(&backup_dir);
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

        let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

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
    Ok(())
}

fn addon_shortcut() -> RawGuiRender {
    render!(|ui| {
        if ui.button("Run Backup") {
            if run_backup().ok().is_some() {
                log::log(
                    log::LogLevel::Info,
                    "Addon Config Backup",
                    "Finished saving backup to nexus-configs folder",
                );
                alert_notify("Finished saving backup to nexus-configs folder");
            }
        }
        ui.same_line_with_spacing(0.0, 10.0);
        if ui.button("Cleanup old backups") {

        }
    })
}