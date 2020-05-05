use std::io::Write;
use std::path::{Path, PathBuf};
use std::{env, fs, io};

use anyhow::Error;
use directories::ProjectDirs;
use structopt::StructOpt;

use crate::errors::LostTheWay;
use crate::utils::NAME;

#[derive(StructOpt, Debug)]
pub(crate) enum ConfigCommand {
    /// Prints / writes the default configuration options.
    /// Set the generated config file as default by setting the $THE_WAY_CONFIG environment variable
    Default {
        #[structopt(parse(from_os_str))]
        file: Option<PathBuf>,
    },
    /// Prints location of currently set configuration file
    Get,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TheWayConfig {
    pub(crate) theme: String,
    pub(crate) db_dir: PathBuf,
    pub(crate) themes_dir: PathBuf,
}

/// Main project directory, cross-platform
fn get_project_dir() -> Result<ProjectDirs, Error> {
    Ok(ProjectDirs::from("rs", "", NAME).ok_or(LostTheWay::Homeless)?)
}

impl Default for TheWayConfig {
    fn default() -> Self {
        let (db_dir, themes_dir, theme) = {
            let dir = get_project_dir().expect("Couldn't get project dir");
            let data_dir = dir.data_dir();
            if !data_dir.exists() {
                fs::create_dir_all(data_dir).expect("Couldn't create data dir");
            }
            (
                data_dir.join("the_way_db"),
                data_dir.join("themes"),
                String::from("base16-ocean.dark"),
            )
        };
        let config = Self {
            theme,
            db_dir,
            themes_dir,
        };
        config.make_dirs().unwrap();
        config
    }
}

impl TheWayConfig {
    pub(crate) fn default_config(file: Option<&Path>) -> Result<(), Error> {
        let writer: Box<dyn io::Write> = match file {
            Some(file) => Box::new(fs::File::open(file)?),
            None => Box::new(io::stdout()),
        };
        let mut buffered = io::BufWriter::new(writer);
        let contents =
            "theme = 'base16-ocean.dark'\ndb_dir = 'the_way_db'\nthemes_dir = 'the_way_themes'";
        write!(&mut buffered, "{}", contents)?;
        Ok(())
    }

    pub(crate) fn print_config_location() -> Result<(), Error> {
        println!("{}", TheWayConfig::get()?.to_string_lossy());
        Ok(())
    }

    fn make_dirs(&self) -> Result<(), Error> {
        if !self.db_dir.exists() {
            fs::create_dir(&self.db_dir).map_err(|e: io::Error| LostTheWay::ConfigError {
                message: format!("Couldn't create db dir {:?}, {}", self.db_dir, e),
            })?;
        }
        if !self.themes_dir.exists() {
            fs::create_dir(&self.themes_dir).map_err(|e: io::Error| LostTheWay::ConfigError {
                message: format!("Couldn't create themes dir {:?}, {}", self.themes_dir, e),
            })?;
        }
        Ok(())
    }

    fn get_default_config_file() -> Result<PathBuf, Error> {
        let dir = get_project_dir()?;
        let config_dir = dir.config_dir();
        Ok(config_dir.join(format!("{}.toml", NAME)))
    }

    /// Gets the current config file location
    fn get() -> Result<PathBuf, Error> {
        let config_file = env::var("THE_WAY_CONFIG").ok();
        match config_file {
            Some(file) => {
                let path = Path::new(&file).to_owned();
                if path.exists() {
                    Ok(path)
                } else {
                    Err(LostTheWay::ConfigError {
                        message: format!("No such file {}", file),
                    }
                    .into())
                }
            }
            None => Self::get_default_config_file(),
        }
    }

    /// Read config from default location
    pub(crate) fn load() -> Result<Self, Error> {
        // Reads THE_WAY_CONFIG environment variable to get config file location
        let config_file = env::var("THE_WAY_CONFIG").ok();
        match config_file {
            Some(file) => {
                let path = Path::new(&file).to_owned();
                if path.exists() {
                    let config: TheWayConfig = confy::load_path(Path::new(&file))?;
                    config.make_dirs()?;
                    Ok(config)
                } else {
                    Err(LostTheWay::ConfigError {
                        message: format!("No such file {}", file),
                    }
                    .into())
                }
            }
            None => Ok(confy::load(NAME)?),
        }
    }

    /// Write possibly modified config
    pub(crate) fn store(&self) -> Result<(), Error> {
        // Reads THE_WAY_CONFIG environment variable to get config file location
        let config_file = env::var("THE_WAY_CONFIG").ok();
        match config_file {
            Some(file) => confy::store_path(Path::new(&file), &(*self).clone())?,
            None => confy::store(NAME, &(*self).clone())?,
        };
        Ok(())
    }
}
