use std::fs;
use std::path::PathBuf;

use assert_cmd::Command;
#[cfg(target_os = "macos")]
use clipboard::{ClipboardContext, ClipboardProvider};
use predicates::prelude::*;
use rexpect::session::PtyBashSession;
use rexpect::spawn_bash;
use tempfile::{tempdir, TempDir};

use the_way::the_way::snippet::Snippet;

fn make_config_file(temp_dir: &TempDir) -> color_eyre::Result<PathBuf> {
    let db_dir = temp_dir.path().join("db");
    let themes_dir = temp_dir.path().join("themes");
    let config_contents = format!(
        "theme = 'base16-ocean.dark'\n\
db_dir = \"{}\"\n\
themes_dir = \"{}\"",
        db_dir.to_str().unwrap(),
        themes_dir.to_str().unwrap()
    );
    let config_file = temp_dir.path().join("the-way.toml");
    fs::write(&config_file, config_contents)?;
    Ok(config_file.to_path_buf())
}

#[test]
fn it_works() -> color_eyre::Result<()> {
    let temp_dir = tempdir()?;
    let config_file = make_config_file(&temp_dir)?;
    let mut cmd = Command::cargo_bin("the-way")?;
    // Pretty much the only command that works without assuming any input or modifying anything
    cmd.env("THE_WAY_CONFIG", config_file)
        .arg("list")
        .assert()
        .success();
    temp_dir.close()?;
    Ok(())
}

#[test]
fn change_config_file() -> color_eyre::Result<()> {
    // Test nonexistent file
    let config_file = "no-such-file";
    let mut cmd = Command::cargo_bin("the-way")?;
    cmd.env("THE_WAY_CONFIG", config_file)
        .arg("config")
        .arg("get")
        .assert()
        .failure();

    // Test changing file
    let temp_dir = tempdir()?;
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
fn change_theme() -> color_eyre::Result<()> {
    let temp_dir = tempdir()?;
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
    temp_dir.close()?;
    Ok(())
}

fn add_snippet_rexpect(config_file: PathBuf) -> rexpect::errors::Result<PtyBashSession> {
    let mut p = spawn_bash(Some(300_000))?;
    p.send_line(&format!(
        "export THE_WAY_CONFIG={}",
        config_file.to_string_lossy()
    ))?;

    let executable = env!("CARGO_BIN_EXE_the-way");
    p.wait_for_prompt()?;
    p.send_line(&format!("{} config get", executable))?;
    p.exp_regex(config_file.to_string_lossy().as_ref())?;
    p.wait_for_prompt()?;
    p.execute(&format!("{} new", executable), "Description")?;
    p.send_line("test description 1")?;
    p.exp_string("Language")?;
    p.send_line("rust")?;
    p.exp_regex("Tags")?;
    p.send_line("tag1 tag2")?;
    p.exp_regex("Code snippet")?;
    p.send_line("code")?;
    p.exp_regex("Added snippet #1")?;
    p.wait_for_prompt()?;
    Ok(p)
}

fn add_two_snippets_rexpect(config_file: PathBuf) -> rexpect::errors::Result<()> {
    let mut p = spawn_bash(Some(300_000))?;
    p.send_line(&format!(
        "export THE_WAY_CONFIG={}",
        config_file.to_string_lossy()
    ))?;

    let executable = env!("CARGO_BIN_EXE_the-way");
    p.wait_for_prompt()?;
    p.send_line(&format!("{} config get", executable))?;
    p.exp_regex(config_file.to_string_lossy().as_ref())?;
    p.execute(&format!("{} new", executable), "Description")?;
    p.send_line("test description 1")?;
    p.exp_string("Language")?;
    p.send_line("rust")?;
    p.exp_regex("Tags")?;
    p.send_line("tag1 tag2")?;
    p.exp_regex("Code snippet")?;
    p.send_line("code")?;
    p.exp_regex("Added snippet #1")?;
    p.wait_for_prompt()?;
    p.execute(&format!("{} new", executable), "Description")?;
    p.send_line("test description 2")?;
    p.exp_string("Language")?;
    p.send_line("python")?;
    p.exp_regex("Tags")?;
    p.send_line("tag1 tag2")?;
    p.exp_regex("Code snippet")?;
    p.send_line("code")?;
    p.exp_regex("Added snippet #2")?;
    Ok(())
}

fn change_snippet_rexpect(config_file: PathBuf) -> rexpect::errors::Result<()> {
    let mut p = add_snippet_rexpect(config_file)?;
    let executable = env!("CARGO_BIN_EXE_the-way");
    p.execute(&format!("{} edit 1", executable), "Description")?;
    p.send_line("test description 2")?;
    p.exp_string("Language")?;
    p.send_line("")?;
    p.exp_regex("Tags")?;
    p.send_line("")?;
    p.exp_regex("Date")?;
    p.send_line("")?;
    p.exp_regex("Code snippet")?;
    p.send_line("code 2")?;
    p.exp_regex("Snippet #1 changed")?;
    p.wait_for_prompt()?;
    p.send_line(&format!("{} view 1", executable))?;
    assert!(p.wait_for_prompt()?.contains("test description 2"));
    Ok(())
}

#[ignore] // expensive, and change_snippet tests both
#[test]
fn add_snippet() -> color_eyre::Result<()> {
    let temp_dir = tempdir()?;
    let config_file = make_config_file(&temp_dir)?;
    assert!(add_snippet_rexpect(config_file).is_ok());
    temp_dir.close()?;
    Ok(())
}

#[test]
fn add_two_snippets() -> color_eyre::Result<()> {
    let temp_dir = tempdir()?;
    let config_file = make_config_file(&temp_dir)?;
    assert!(add_two_snippets_rexpect(config_file).is_ok());
    temp_dir.close()?;
    Ok(())
}

#[test]
fn change_snippet() -> color_eyre::Result<()> {
    let temp_dir = tempdir()?;
    let config_file = make_config_file(&temp_dir)?;
    assert!(change_snippet_rexpect(config_file).is_ok());
    temp_dir.close()?;
    Ok(())
}

#[test]
fn import_single_show() -> color_eyre::Result<()> {
    let contents = r#"{"description":"test description","language":"rust","tags":["tag1","tag2"],"code":"some\ntest\ncode\n"}"#;
    let temp_dir = tempdir()?;
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
    temp_dir.close()?;
    Ok(())
}

#[test]
fn import_multiple_no_tags() -> color_eyre::Result<()> {
    let contents_1 = r#"{"description":"test description 1","language":"rust","tags":["tag1","tag2"],"code":"some\ntest\ncode\n"}"#;
    let contents_2 =
        r#"{"description":"test description 2","language":"python","code":"some\ntest\ncode\n"}"#;
    let contents = format!("{}{}", contents_1, contents_2);
    let temp_dir = tempdir()?;
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
    temp_dir.close()?;
    Ok(())
}

#[test]
fn export() -> color_eyre::Result<()> {
    let contents_1 = r#"{"description":"test description 1","language":"rust","tags":["tag1","tag2"],"code":"some\ntest\ncode\n"}"#;
    let contents_2 =
        r#"{"description":"test description 2","language":"python","code":"some\ntest\ncode\n"}"#;
    let contents = format!("{}{}", contents_1, contents_2);
    let temp_dir = tempdir()?;
    let config_file = make_config_file(&temp_dir)?;

    // import
    let mut cmd = Command::cargo_bin("the-way")?;
    cmd.env("THE_WAY_CONFIG", &config_file)
        .arg("import")
        .write_stdin(contents)
        .assert()
        .stdout(predicate::str::starts_with("Imported 2 snippets"));

    // export
    let mut cmd = Command::cargo_bin("the-way")?;
    let file = temp_dir.path().join("snippets.json");
    cmd.env("THE_WAY_CONFIG", &config_file)
        .arg("export")
        .arg(file.to_str().unwrap())
        .assert()
        .success();

    let snippets = serde_json::Deserializer::from_reader(fs::File::open(&file)?)
        .into_iter::<Snippet>()
        .collect::<Result<Vec<_>, _>>()?;

    assert_eq!(snippets.len(), 2);

    for snippet in snippets {
        assert_eq!(snippet.code, "some\ntest\ncode\n");
    }
    temp_dir.close()?;
    Ok(())
}

#[test]
fn delete() -> color_eyre::Result<()> {
    let contents_1 = r#"{"description":"test description 1","language":"rust","tags":["tag1","tag2"],"code":"some\ntest\ncode\n"}"#;
    let contents_2 =
        r#"{"description":"test description 2","language":"python","code":"some\ntest\ncode\n"}"#;
    let contents = format!("{}{}", contents_1, contents_2);
    let temp_dir = tempdir()?;
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
    temp_dir.close()?;
    Ok(())
}

#[cfg(target_os = "macos")]
#[test]
fn copy() -> color_eyre::Result<()> {
    let contents = r#"{"description":"test description","language":"rust","tags":["tag1","tag2"],"code":"some\ntest\ncode\n"}"#;
    let temp_dir = tempdir()?;
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
    let ctx: color_eyre::Result<ClipboardContext, _> = ClipboardProvider::new();
    assert!(ctx.is_ok());
    let mut ctx = ctx.unwrap();
    let contents = ctx.get_contents();
    assert!(contents.is_ok());
    let contents = contents.unwrap();
    assert!(contents.contains("some\ntest\ncode"));
    temp_dir.close()?;
    Ok(())
}
