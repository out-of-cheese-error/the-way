use std::fs;
use std::path::PathBuf;

use anyhow::Error;
use assert_cmd::Command;
#[cfg(target_os = "macos")]
use clipboard::{ClipboardContext, ClipboardProvider};
use predicates::prelude::*;
use rexpect::session::PtyBashSession;
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
    // Test nonexistent file
    let config_file = "no-such-file";
    let mut cmd = Command::cargo_bin("the-way")?;
    cmd.env("THE_WAY_CONFIG", config_file)
        .arg("config")
        .arg("get")
        .assert()
        .failure();

    // Test changing file
    let temp_dir = create_temp_dir("change_config_file")?;
    let config_file = make_config_file(&temp_dir)?;
    let mut cmd = Command::cargo_bin("the-way")?;
    cmd.env("THE_WAY_CONFIG", &config_file)
        .arg("config")
        .arg("get")
        .assert()
        .stdout(predicate::str::starts_with(config_file.to_string_lossy()));
    temp_dir.close()?;
    Ok(())
}

#[test]
fn change_theme() -> Result<(), Error> {
    let temp_dir = create_temp_dir("change_theme")?;
    let config_file = make_config_file(&temp_dir)?;
    // Test nonexistent theme
    let theme = "no-such-theme";
    let mut cmd = Command::cargo_bin("the-way")?;
    cmd.env("THE_WAY_CONFIG", &config_file)
        .arg("themes")
        .arg("set")
        .arg(theme)
        .assert()
        .failure();

    // Test changing theme
    let theme = "base16-mocha.dark";
    let mut cmd = Command::cargo_bin("the-way")?;
    cmd.env("THE_WAY_CONFIG", &config_file)
        .arg("themes")
        .arg("set")
        .arg(theme)
        .assert()
        .success();
    let mut cmd = Command::cargo_bin("the-way")?;
    cmd.env("THE_WAY_CONFIG", &config_file)
        .arg("themes")
        .arg("get")
        .assert()
        .stdout(predicate::str::contains(theme));
    Ok(())
}

fn add_snippet_rexpect(config_file: PathBuf) -> rexpect::errors::Result<PtyBashSession> {
    // shouldn't take more than 30 mins for a release build right?
    let mut p = spawn_bash(Some(300_000))?;
    p.send_line(&format!(
        "export THE_WAY_CONFIG={}",
        config_file.to_string_lossy()
    ))?;
    p.wait_for_prompt()?;
    p.send_line("cargo build --release")?;
    p.wait_for_prompt()?;
    p.send_line("target/release/the-way config get")?;
    p.exp_regex(config_file.to_string_lossy().as_ref())?;
    p.wait_for_prompt()?;
    p.execute("target/release/the-way new", "Description:")?;
    p.send_line("test description 1")?;
    p.exp_string("Language:")?;
    p.send_line("rust")?;
    p.exp_regex("Tags \\(.*\\):")?;
    p.send_line("tag1 tag2")?;
    p.exp_regex("Code snippet \\(.*\\):")?;
    p.send_line("code")?;
    p.exp_regex("Added snippet #1")?;
    p.wait_for_prompt()?;
    Ok(p)
}

fn change_snippet_rexpect(config_file: PathBuf) -> rexpect::errors::Result<()> {
    let mut p = add_snippet_rexpect(config_file)?;
    p.execute("target/release/the-way edit 1", "Description:")?;
    p.send_line("test description 2")?;
    p.exp_string("Language:")?;
    p.send_line("")?;
    p.exp_regex("Tags \\(.*\\):")?;
    p.send_line("")?;
    p.exp_regex("Date \\[.*\\]:")?;
    p.send_line("")?;
    p.exp_regex("Code snippet \\(.*\\):")?;
    p.send_line("code 2")?;
    p.exp_regex("Snippet #1 changed")?;
    p.wait_for_prompt()?;
    p.send_line("target/release/the-way view 1")?;
    assert!(p.wait_for_prompt()?.contains("test description 2"));
    Ok(())
}

#[ignore] // expensive, and change_snippet tests both
#[test]
fn add_snippet() -> Result<(), Error> {
    let temp_dir = create_temp_dir("add_snippet")?;
    let config_file = make_config_file(&temp_dir)?;
    assert!(add_snippet_rexpect(config_file).is_ok());
    temp_dir.close()?;
    Ok(())
}

#[test]
fn change_snippet() -> Result<(), Error> {
    let temp_dir = create_temp_dir("change_snippet")?;
    let config_file = make_config_file(&temp_dir)?;
    assert!(change_snippet_rexpect(config_file).is_ok());
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
    cmd.env("THE_WAY_CONFIG", &config_file)
        .arg("view")
        .arg("1")
        .assert()
        .stdout(predicate::str::contains("test description"));
    Ok(())
}

#[test]
fn import_multiple_no_tags() -> Result<(), Error> {
    let contents_1 = r#"{"description":"test description 1","language":"rust","tags":["tag1","tag2"],"code":"some\ntest\ncode\n"}"#;
    let contents_2 =
        r#"{"description":"test description 2","language":"python","code":"some\ntest\ncode\n"}"#;
    let contents = format!("{}{}", contents_1, contents_2);
    let temp_dir = create_temp_dir("import_multiple_no_tags")?;
    let config_file = make_config_file(&temp_dir)?;
    let mut cmd = Command::cargo_bin("the-way")?;
    cmd.env("THE_WAY_CONFIG", &config_file)
        .arg("import")
        .write_stdin(contents)
        .assert()
        .stdout(predicate::str::starts_with("Imported 2 snippets"));
    let mut cmd = Command::cargo_bin("the-way")?;
    cmd.env("THE_WAY_CONFIG", &config_file)
        .arg("list")
        .assert()
        .stdout(
            predicate::str::contains("test description 1")
                .and(predicate::str::contains("test description 2")),
        );
    Ok(())
}

#[test]
fn delete() -> Result<(), Error> {
    let contents_1 = r#"{"description":"test description 1","language":"rust","tags":["tag1","tag2"],"code":"some\ntest\ncode\n"}"#;
    let contents_2 =
        r#"{"description":"test description 2","language":"python","code":"some\ntest\ncode\n"}"#;
    let contents = format!("{}{}", contents_1, contents_2);
    let temp_dir = create_temp_dir("delete")?;
    let config_file = make_config_file(&temp_dir)?;
    let mut cmd = Command::cargo_bin("the-way")?;
    cmd.env("THE_WAY_CONFIG", &config_file)
        .arg("import")
        .write_stdin(contents)
        .assert()
        .stdout(predicate::str::starts_with("Imported 2 snippets"));
    let mut cmd = Command::cargo_bin("the-way")?;
    cmd.env("THE_WAY_CONFIG", &config_file)
        .arg("list")
        .assert()
        .stdout(
            predicate::str::contains("test description 1")
                .and(predicate::str::contains("test description 2")),
        );

    // Test bad index
    let mut cmd = Command::cargo_bin("the-way")?;
    cmd.env("THE_WAY_CONFIG", &config_file)
        .arg("del")
        .arg("-f")
        .arg("3")
        .assert()
        .failure();

    // Test good index
    let mut cmd = Command::cargo_bin("the-way")?;
    cmd.env("THE_WAY_CONFIG", &config_file)
        .arg("del")
        .arg("-f")
        .arg("2")
        .assert()
        .stdout(predicate::str::starts_with("Snippet #2 deleted"));
    let mut cmd = Command::cargo_bin("the-way")?;
    cmd.env("THE_WAY_CONFIG", &config_file)
        .arg("list")
        .assert()
        .stdout(
            predicate::str::contains("test description 1")
                .and(predicate::str::contains("test description 2").not()),
        );

    // Test already deleted index
    let mut cmd = Command::cargo_bin("the-way")?;
    cmd.env("THE_WAY_CONFIG", &config_file)
        .arg("del")
        .arg("-f")
        .arg("2")
        .assert()
        .failure();
    Ok(())
}

#[cfg(target_os = "macos")]
#[test]
fn copy() -> Result<(), Error> {
    let contents = r#"{"description":"test description","language":"rust","tags":["tag1","tag2"],"code":"some\ntest\ncode\n"}"#;
    let temp_dir = create_temp_dir("copy")?;
    let config_file = make_config_file(&temp_dir)?;
    let mut cmd = Command::cargo_bin("the-way")?;
    cmd.env("THE_WAY_CONFIG", &config_file)
        .arg("import")
        .write_stdin(contents)
        .assert()
        .success();

    // Test bad index
    let mut cmd = Command::cargo_bin("the-way")?;
    cmd.env("THE_WAY_CONFIG", &config_file)
        .arg("cp")
        .arg("2")
        .assert()
        .failure();

    // Test good index
    let mut cmd = Command::cargo_bin("the-way")?;
    cmd.env("THE_WAY_CONFIG", &config_file)
        .arg("cp")
        .arg("1")
        .assert()
        .stdout(predicate::str::starts_with(
            "Snippet #1 copied to clipboard",
        ));
    let ctx: Result<ClipboardContext, _> = ClipboardProvider::new();
    assert!(ctx.is_ok());
    let mut ctx = ctx.unwrap();
    let contents = ctx.get_contents();
    assert!(contents.is_ok());
    let contents = contents.unwrap();
    assert!(contents.contains("some\ntest\ncode"));
    Ok(())
}
