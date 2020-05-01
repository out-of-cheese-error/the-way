use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Error;
use assert_cmd::Command;
use rexpect::spawn_bash;
use tempdir::TempDir;

fn create_temp_dir(name: &str) -> Result<TempDir, Error> {
    Ok(TempDir::new(name)?)
}

fn make_config_file(tempdir: &TempDir) -> Result<PathBuf, Error> {
    let db_dir = tempdir.path().join("db");
    let themes_dir = tempdir.path().join("themes");
    let config_contents = format!(
        "theme = 'base16-ocean.dark'\n\
db_dir = \"{}\"\n\
themes_dir = \"{}\"",
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
    let temp_dir = create_temp_dir("change_config_file")?;
    let config_file = make_config_file(&temp_dir)?;
    let mut cmd = Command::cargo_bin("the-way")?;
    let result = cmd
        .env("THE_WAY_CONFIG", &config_file)
        .arg("config")
        .arg("get")
        .assert();
    let output_config_file = String::from_utf8_lossy(&result.get_output().stdout);
    let output_config_file = Path::new(output_config_file.trim());
    assert!(output_config_file.exists(), "{:?}", output_config_file);
    assert_eq!(output_config_file, config_file);
    temp_dir.close()?;
    Ok(())
}

#[test]
fn change_theme() -> Result<(), Error> {
    let temp_dir = create_temp_dir("change_theme")?;
    let config_file = make_config_file(&temp_dir)?;
    let theme = "base16-ocean.dark";
    let mut cmd = Command::cargo_bin("the-way")?;
    cmd.env("THE_WAY_CONFIG", &config_file)
        .arg("themes")
        .arg("set")
        .arg(theme)
        .assert()
        .success();
    let mut cmd = Command::cargo_bin("the-way")?;
    assert_eq!(
        String::from_utf8_lossy(
            &cmd.env("THE_WAY_CONFIG", &config_file)
                .arg("themes")
                .arg("current")
                .assert()
                .get_output()
                .stdout
        )
        .trim(),
        theme
    );
    Ok(())
}

fn add_snippet_rexpect(config_file: PathBuf) -> rexpect::errors::Result<()> {
    let mut p = spawn_bash(Some(30_000))?;
    println!("Change config file");
    p.send_line(&format!(
        "export THE_WAY_CONFIG={}",
        config_file.to_string_lossy()
    ))?;
    p.wait_for_prompt()?;
    println!("Make bin");
    p.send_line("cargo build --release")?;
    p.wait_for_prompt()?;
    println!("Assert that the change worked");
    p.send_line("target/release/the-way config get")?; // TODO: yuck
    p.exp_regex(config_file.to_string_lossy().as_ref())?;
    p.wait_for_prompt()?;
    println!("Add a snippet");
    p.execute("target/release/the-way", "Description:")?; // TODO: yuck
    p.send_line("test description")?;
    p.exp_string("Language:")?;
    p.send_line("rust")?;
    p.exp_regex("Tags \\(.*\\):")?;
    p.send_line("tag1 tag2")?;
    p.exp_regex("Code snippet \\(.*\\):")?;
    p.send_line("code")?;
    p.exp_regex("Added snippet #1")?;
    p.wait_for_prompt()?;
    Ok(())
}

#[test]
fn add_snippet() -> Result<(), Error> {
    let temp_dir = create_temp_dir("add_snippet")?;
    let config_file = make_config_file(&temp_dir)?;
    assert!(add_snippet_rexpect(config_file).is_ok());
    temp_dir.close()?;
    Ok(())
}

#[test]
fn import_single_show() -> Result<(), Error> {
    let contents = r#"{"description":"test description","language":"rust","tags":["tag1","tag2"],"code":"some\ntest\ncode\n"}"#;
    let temp_dir = create_temp_dir("import")?;
    let config_file = make_config_file(&temp_dir)?;
    let mut cmd = Command::cargo_bin("the-way")?;
    cmd.env("THE_WAY_CONFIG", &config_file)
        .arg("import")
        .write_stdin(contents)
        .assert()
        .success();
    let mut cmd = Command::cargo_bin("the-way")?;
    assert!(String::from_utf8_lossy(
        &cmd.env("THE_WAY_CONFIG", &config_file)
            .arg("show 1")
            .assert()
            .get_output()
            .stdout,
    )
    .contains("test description"));
    Ok(())
}

#[test]
fn import_multiple_no_tags() -> Result<(), Error> {
    let contents_1 = r#"{"description":"test description","language":"rust","tags":["tag1","tag2"],"code":"some\ntest\ncode\n"}"#;
    let contents_2 =
        r#"{"description":"test description 2","language":"python","code":"some\ntest\ncode\n"}"#;
    let contents = format!("{}{}", contents_1, contents_2);
    let temp_dir = create_temp_dir("import_multiple_no_tags")?;
    let config_file = make_config_file(&temp_dir)?;
    let mut cmd = Command::cargo_bin("the-way")?;
    let output = cmd
        .env("THE_WAY_CONFIG", &config_file)
        .arg("import")
        .write_stdin(contents)
        .assert();
    assert_eq!(
        String::from_utf8_lossy(&output.get_output().stdout).trim(),
        "Imported 2 snippets"
    );
    let mut cmd = Command::cargo_bin("the-way")?;
    let list_output = cmd.env("THE_WAY_CONFIG", &config_file).arg("list").assert();
    let list_output = String::from_utf8_lossy(&list_output.get_output().stdout);
    assert!(list_output.contains("test description"));
    assert!(list_output.contains("test description 2"));
    Ok(())
}
