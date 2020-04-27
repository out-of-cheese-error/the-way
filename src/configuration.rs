use anyhow::Error;
use directories::ProjectDirs;
use path_abs::{PathDir, PathOps};

use crate::errors::LostTheWay;

const NAME: &str = "the-way";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TheWayConfig {
    pub(crate) theme: String,
    pub(crate) db_dir: PathDir,
    pub(crate) themes_dir: PathDir,
}

fn get_project_dir() -> Result<ProjectDirs, Error> {
    Ok(ProjectDirs::from("", "", NAME).ok_or(LostTheWay::Homeless)?)
}

impl Default for TheWayConfig {
    fn default() -> Self {
        let dir = get_project_dir().unwrap();
        let data_dir = PathDir::create_all(dir.data_dir()).unwrap();
        Self {
            theme: String::from("base16-ocean.dark"),
            db_dir: PathDir::create(data_dir.join("the_way_db")).unwrap(),
            themes_dir: PathDir::create(data_dir.join("themes")).unwrap(),
        }
    }
}

impl TheWayConfig {
    pub(crate) fn get() -> Result<Self, confy::ConfyError> {
        Ok(confy::load(NAME)?)
    }

    pub(crate) fn store(&self) -> Result<(), Error> {
        confy::store(NAME, &(*self).clone())?;
        Ok(())
    }
}
