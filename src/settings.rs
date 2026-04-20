use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct Settings {
    pub target_folder: Option<String>,
    pub backup_on_launch: Option<bool>,
    pub delete_old_on_launch: Option<bool>,
    pub backups_to_keep: Option<i32>,
    pub package_addons: Option<bool>,
}

impl Settings {
    pub fn default() -> Settings {
        Settings {
            target_folder: Some(
                dirs_next::document_dir()
                    .unwrap()
                    .join("nexus-configs")
                    .to_str()
                    .unwrap()
                    .to_string(),
            ),
            backup_on_launch: Some(false),
            delete_old_on_launch: Some(false),
            backups_to_keep: Some(5),
            package_addons: Some(false),
        }
    }
    pub fn validate(&mut self) {
        let defaults = Self::default();
        if self.backup_on_launch.is_none() {
            self.backup_on_launch = defaults.backup_on_launch;
        }
        if self.package_addons.is_none() {
            self.package_addons = defaults.package_addons;
        }
        if self.delete_old_on_launch.is_none() {
            self.delete_old_on_launch = defaults.delete_old_on_launch;
        }
        if self.backups_to_keep.is_none() {
            self.backups_to_keep = defaults.backups_to_keep;
        }
        if self.target_folder.is_none() {
            self.target_folder = defaults.target_folder;
        }
    }
}
