use std::fs;
use std::path::PathBuf;

use anyhow::Error;
use directories::ProjectDirs;

use crate::errors::LostTheWay;
use crate::utils::NAME;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TheWayConfig {
    pub(crate) theme: String,
    pub(crate) db_dir: PathBuf,
    pub(crate) themes_dir: PathBuf,
}

/// Main project directory, cross-platform
fn get_project_dir() -> Result<ProjectDirs, Error> {
    Ok(ProjectDirs::from("", "", NAME).ok_or(LostTheWay::Homeless)?)
}

impl Default for TheWayConfig {
    fn default() -> Self {
        let dir = get_project_dir().expect("Couldn't get project dir");
        let data_dir = dir.data_dir();
        if !data_dir.exists() {
            fs::create_dir_all(data_dir).expect("Couldn't create data dir");
        }
        let db_dir = data_dir.join("the_way_db");
        if !db_dir.exists() {
            fs::create_dir(&db_dir).expect("Couldn't create db dir");
        }
        let themes_dir = data_dir.join("themes");
        if !themes_dir.exists() {
            fs::create_dir(&themes_dir).expect("Couldn't create themes dir");
        }
        Self {
            theme: String::from("base16-ocean.dark"),
            db_dir,
            themes_dir,
        }
    }
}

impl TheWayConfig {
    /// Read config
    pub(crate) fn get() -> Result<Self, confy::ConfyError> {
        Ok(confy::load(NAME)?)
    }

    /// Write possibly modified config
    pub(crate) fn store(&self) -> Result<(), Error> {
        confy::store(NAME, &(*self).clone())?;
        Ok(())
    }
}
