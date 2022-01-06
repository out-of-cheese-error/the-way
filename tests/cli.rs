use std::fs;
use std::path::PathBuf;

use assert_cmd::Command;
use predicates::prelude::*;
use rexpect::session::PtyReplSession;
use rexpect::spawn_bash;
use tempfile::{tempdir, TempDir};

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
        .stdout(predicate::str::contains(config_file.to_string_lossy()));
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

fn add_snippet_rexpect(config_file: PathBuf) -> rexpect::errors::Result<PtyReplSession> {
    let mut p = spawn_bash(Some(3000))?;
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
    p.exp_regex("Snippet #1 added")?;
    p.wait_for_prompt()?;
    Ok(p)
}

fn add_two_snippets_rexpect(config_file: PathBuf) -> rexpect::errors::Result<()> {
    let mut p = spawn_bash(Some(3000))?;
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
    p.exp_regex("Snippet #1 added")?;
    p.wait_for_prompt()?;
    p.execute(&format!("{} new", executable), "Description")?;
    p.send_line("test description 2")?;
    p.exp_string("Language")?;
    p.send_line("python")?;
    p.exp_regex("Tags")?;
    p.send_line("tag1 tag2")?;
    p.exp_regex("Code snippet")?;
    p.send_line("code")?;
    p.exp_regex("Snippet #2 added")?;
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
    p.exp_regex("Edit snippet")?;
    p.send_line("")?;
    p.exp_regex("Snippet #1 changed")?;
    p.wait_for_prompt()?;
    p.send_line(&format!("{} view 1", executable))?;
    assert!(p.wait_for_prompt()?.contains("test description 2"));
    Ok(())
}

fn add_two_cmd_snippets_rexpect(config_file: PathBuf) -> rexpect::errors::Result<()> {
    let mut p = spawn_bash(Some(3000))?;
    p.send_line(&format!(
        "export THE_WAY_CONFIG={}",
        config_file.to_string_lossy()
    ))?;

    let executable = env!("CARGO_BIN_EXE_the-way");
    p.wait_for_prompt()?;
    p.send_line(&format!("{} config get", executable))?;
    p.exp_regex(config_file.to_string_lossy().as_ref())?;
    // as argument
    p.execute(
        &format!("{} cmd \"shell snippet 1\"", executable),
        "Command",
    )?;
    p.send_line("\n")?;
    p.exp_string("Description")?;
    p.send_line("test description 1")?;
    p.exp_regex("Tags")?;
    p.send_line("tag1 tag2")?;
    p.exp_regex("Snippet #1 added")?;
    p.wait_for_prompt()?;
    // interactively
    p.execute(&format!("{} cmd", executable), "Command")?;
    p.send_line("shell snippet 2")?;
    p.exp_string("Description")?;
    p.send_line("test description 2")?;
    p.exp_regex("Tags")?;
    p.send_line("tag1 tag2")?;
    p.exp_regex("Snippet #2 added")?;
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
fn add_two_cmd_snippets() -> color_eyre::Result<()> {
    let temp_dir = tempdir()?;
    let config_file = make_config_file(&temp_dir)?;
    assert!(add_two_cmd_snippets_rexpect(config_file).is_ok());
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
    let contents = format!("{}\n{}", contents_1, contents_2);
    let temp_dir = tempdir()?;
    let config_file = make_config_file(&temp_dir)?;
    let mut cmd = Command::cargo_bin("the-way")?;
    cmd.env("THE_WAY_CONFIG", &config_file)
        .arg("import")
        .write_stdin(contents)
        .assert()
        .stdout(predicate::str::contains("Imported 2 snippets"));
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

// This test is ignored because it tries to fetch a real Gist and runs into
// Github rate limits when ran by CI (not sure why this happens though).
#[ignore]
#[test]
fn import_gist() -> color_eyre::Result<()> {
    let temp_dir = tempdir()?;
    let config_file = make_config_file(&temp_dir)?;
    let mut cmd = Command::cargo_bin("the-way")?;
    cmd.env("THE_WAY_CONFIG", &config_file)
        .arg("import")
        .arg("-g https://gist.github.com/xiaochuanyu/e5deab8d78ce838f22f160c9b14daf17")
        .assert()
        .success();
    let mut cmd = Command::cargo_bin("the-way")?;
    cmd.env("THE_WAY_CONFIG", &config_file)
        .arg("view")
        .arg("1")
        .assert()
        .stdout(predicate::str::contains(
            "the-way Test - e5deab8d78ce838f22f160c9b14daf17 - TestTheWay.java",
        ));
    temp_dir.close()?;
    Ok(())
}

#[test]
fn export() -> color_eyre::Result<()> {
    use the_way::the_way::snippet::Snippet;

    let contents_1 = r#"{"description":"test description 1","language":"rust","tags":["tag1","tag2"],"code":"some\ntest\ncode\n"}"#;
    let contents_2 =
        r#"{"description":"test description 2","language":"python","code":"some\ntest\ncode\n"}"#;
    let contents = format!("{}\n{}", contents_1, contents_2);
    let temp_dir = tempdir()?;
    let config_file = make_config_file(&temp_dir)?;

    // import
    let mut cmd = Command::cargo_bin("the-way")?;
    cmd.env("THE_WAY_CONFIG", &config_file)
        .arg("import")
        .write_stdin(contents)
        .assert()
        .stdout(predicate::str::contains("Imported 2 snippets"));

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
    let contents = format!("{}\n{}", contents_1, contents_2);
    let temp_dir = tempdir()?;
    let config_file = make_config_file(&temp_dir)?;
    let mut cmd = Command::cargo_bin("the-way")?;
    cmd.env("THE_WAY_CONFIG", &config_file)
        .arg("import")
        .write_stdin(contents)
        .assert()
        .stdout(predicate::str::contains("Imported 2 snippets"));
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
        .stdout(predicate::str::contains("Snippet #2 deleted"));
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
    use clipboard::{ClipboardContext, ClipboardProvider};

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
        .stderr(predicate::str::contains("Snippet #1 copied to clipboard"));
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

#[cfg(target_os = "macos")]
#[test]
fn copy_shell_script() -> color_eyre::Result<()> {
    use clipboard::{ClipboardContext, ClipboardProvider};

    let temp_dir = tempdir()?;
    let config_file = make_config_file(&temp_dir)?;
    assert!(copy_shell_script_rexpect(config_file).is_ok());
    let ctx: color_eyre::Result<ClipboardContext, _> = ClipboardProvider::new();
    assert!(ctx.is_ok());
    let mut ctx = ctx.unwrap();
    let contents = ctx.get_contents();
    assert!(contents.is_ok());
    let contents = contents.unwrap();
    assert!(contents.contains("shell snippet value1 code value2 value1"));
    temp_dir.close()?;
    Ok(())
}

fn copy_shell_script_rexpect(config_file: PathBuf) -> rexpect::errors::Result<()> {
    let mut p = spawn_bash(Some(3000))?;
    p.send_line(&format!(
        "export THE_WAY_CONFIG={}",
        config_file.to_string_lossy()
    ))?;

    let executable = env!("CARGO_BIN_EXE_the-way");
    p.wait_for_prompt()?;
    p.send_line(&format!("{} config get", executable))?;
    p.exp_regex(config_file.to_string_lossy().as_ref())?;
    // add a shell snippet
    p.execute(
        &format!(
            "{} cmd \"shell snippet <param1=value1> code <param2> <param1>\"",
            executable
        ),
        "Command",
    )?;
    p.send_line("\n")?;
    p.exp_string("Description")?;
    p.send_line("test description 1")?;
    p.exp_regex("Tags")?;
    p.send_line("tag1 tag2")?;
    p.exp_regex("Snippet #1 added")?;
    p.wait_for_prompt()?;
    // Test interactive copy
    p.execute(&format!("{} cp 1", executable), "param1")?;
    p.send_line("\n")?;
    p.exp_string("param2")?;
    p.send_line("value2")?;
    p.exp_regex("Snippet #1 copied to clipboard")?;
    Ok(())
}

#[ignore]
#[test]
/// Tests Gist sync functionality. Needs to have the environment variable $THE_WAY_GITHUB_TOKEN set!
/// Ignored by default since Travis doesn't allow secret/encrypted environment variables in PRs
fn sync_gist() -> color_eyre::Result<()> {
    use the_way::configuration::TheWayConfig;
    use the_way::gist::{GistClient, GistContent, UpdateGistPayload};

    let temp_dir = tempdir()?;
    let config_file = make_config_file(&temp_dir)?;

    let contents_1 = r#"{"description":"test description 1","language":"rust","tags":["tag1","tag2"],"code":"some\ntest\ncode\n"}"#;
    let contents_2 =
        r#"{"description":"test description 2","language":"python","code":"some\ntest\ncode\n"}"#;
    let contents = format!("{}\n{}", contents_1, contents_2);

    // import
    let mut cmd = Command::cargo_bin("the-way")?;
    cmd.env("THE_WAY_CONFIG", &config_file)
        .arg("import")
        .write_stdin(contents)
        .assert()
        .stdout(predicate::str::contains("Imported 2 snippets"));

    // sync
    let mut cmd = Command::cargo_bin("the-way")?;
    cmd.env("THE_WAY_CONFIG", &config_file)
        .arg("sync")
        .assert()
        .success();

    // get gist_id from config
    std::env::set_var("THE_WAY_CONFIG", &config_file);
    let config = TheWayConfig::load();
    assert!(config.is_ok());
    let config = config?;
    assert!(config.gist_id.is_some());

    // get Gist
    let token = &std::env::var("THE_WAY_GITHUB_TOKEN")?;
    let client = GistClient::new(Some(token))?;
    let gist = client.get_gist(&config.gist_id.unwrap());
    assert!(gist.is_ok());
    let gist = gist?;

    // check Gist contents
    assert_eq!(gist.files.len(), 3);
    for (filename, gistfile) in &gist.files {
        if filename.starts_with("snippet_") {
            assert_eq!(gistfile.content, "some\ntest\ncode\n");
        }
    }

    // edit Gist
    let update_payload = UpdateGistPayload {
        description: &gist.description,
        files: vec![(
            "snippet_1.rs".to_owned(),
            Some(GistContent {
                content: "some\nmore\ntest\ncode\n",
            }),
        )]
        .into_iter()
        .collect(),
    };
    assert!(client.update_gist(&gist.id, &update_payload).is_ok());

    // delete locally (easier than editing)
    let mut cmd = Command::cargo_bin("the-way")?;
    cmd.env("THE_WAY_CONFIG", &config_file)
        .arg("del")
        .arg("-f")
        .arg("2")
        .assert()
        .stdout(predicate::str::contains("Snippet #2 deleted"));

    // sync - download + deleted
    let mut cmd = Command::cargo_bin("the-way")?;
    cmd.env("THE_WAY_CONFIG", &config_file)
        .arg("sync")
        .assert()
        .success();

    // get Gist
    let gist = client.get_gist(&gist.id);
    assert!(gist.is_ok());
    let gist = gist?;
    let updated = &gist.updated_at;

    // sync again - nothing changed
    let mut cmd = Command::cargo_bin("the-way")?;
    cmd.env("THE_WAY_CONFIG", &config_file)
        .arg("sync")
        .assert()
        .success();
    let gist = client.get_gist(&gist.id);
    assert!(gist.is_ok());
    let gist = gist?;
    assert_eq!(updated, &gist.updated_at);

    // check Gist contents
    assert_eq!(gist.files.len(), 2);
    for (filename, gistfile) in &gist.files {
        if filename.starts_with("snippet_") {
            assert_eq!(gistfile.content, "some\nmore\ntest\ncode\n");
        }
    }

    // check downloaded
    let mut cmd = Command::cargo_bin("the-way")?;
    cmd.env("THE_WAY_CONFIG", &config_file)
        .arg("view")
        .arg("1")
        .assert()
        .stdout(predicates::str::contains("more"));

    // delete Gist
    assert!(client.delete_gist(&gist.id).is_ok());
    assert!(client.get_gist(&gist.id).is_err());
    temp_dir.close()?;
    Ok(())
}
