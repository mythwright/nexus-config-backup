use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct Settings {
    pub target_folder: String,
    pub backup_on_launch: bool,
    pub delete_old_on_launch: bool,
    pub backups_to_keep: i32,
}

impl Settings {
    pub fn default() -> Settings {
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