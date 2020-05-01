use std::convert::TryInto;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Error;
use assert_cmd::Command;
use directories::ProjectDirs;
use predicates::prelude::*;
use tempdir::TempDir;

fn create_temp_dir(name: &str) -> Result<TempDir, Error> {
    Ok(TempDir::new(name)?)
}

fn make_config_file(name: &str) -> Result<PathBuf, Error> {
    let tempdir = create_temp_dir(name)?;
    let db_dir = tempdir.path().join("db");
    let themes_dir = tempdir.path().join("themes");
    let config_contents = format!(
        "theme = 'base16-ocean.dark'\n\
db_dir = {}\n\
themes_dir = {}",
        db_dir.to_str().unwrap(),
        themes_dir.to_str().unwrap()
    );
    let config_file = tempdir.path().join("the-way.toml");
    fs::write(&config_file, config_contents)?;
    Ok(config_file.to_path_buf())
}

#[test]
fn it_works() -> Result<(), Error> {
    let mut cmd = Command::cargo_bin("the-way")?;
    // Pretty much the only command that works without assuming any input or modifying anything
    cmd.arg("list").assert().success();
    Ok(())
}

#[test]
fn change_config_file() -> Result<(), Error> {
    let config_file = make_config_file("change_config_file")?;
    let mut cmd = Command::cargo_bin("the-way")?;
    let output = cmd
        .env("THE_WAY_CONFIG", &config_file)
        .arg("config")
        .arg("get")
        .assert()
        .get_output()
        .stdout
        .clone();
    let output_config_file = String::from_utf8(output)?.trim().to_owned();
    let output_config_file = Path::new(&output_config_file);
    assert!(output_config_file.exists(), "{:?}", output_config_file);
    assert_eq!(output_config_file, config_file);
    Ok(())
}

#[test]
fn set_theme() -> Result<(), Error> {
    let theme = "base16-ocean.dark";
    let mut cmd = Command::cargo_bin("the-way")?;
    cmd.arg("themes").arg("set").arg(theme).assert().success();
    let mut cmd = Command::cargo_bin("the-way")?;
    let output = cmd
        .arg("themes")
        .arg("current")
        .assert()
        .get_output()
        .stdout
        .clone();
    let theme_output = String::from_utf8(output)?;
    assert_eq!(theme_output.trim(), theme);
    Ok(())
}
