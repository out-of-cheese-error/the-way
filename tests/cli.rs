use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use expectrl::repl::{spawn_bash, ReplSession};
use predicates::prelude::*;
use tempfile::{tempdir, TempDir};
use the_way::configuration::TheWayConfig;
use the_way::gist::{Gist, GistClient, GistContent, UpdateGistPayload};

fn setup_the_way() -> color_eyre::Result<(TempDir, PathBuf)> {
    let temp_dir = tempdir()?;
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
    Ok((temp_dir, config_file))
}

#[test]
fn it_works() -> color_eyre::Result<()> {
    let (temp_dir, config_file) = setup_the_way()?;
    let mut cmd = Command::cargo_bin("the-way")?;
    // Pretty much the only command that works without assuming any input or modifying anything
    cmd.env("THE_WAY_CONFIG", &config_file)
        .arg("list")
        .assert()
        .success();
    drop(config_file);
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
    let (temp_dir, config_file) = setup_the_way()?;
    let mut cmd = Command::cargo_bin("the-way")?;
    cmd.env("THE_WAY_CONFIG", &config_file)
        .arg("config")
        .arg("get")
        .assert()
        .stdout(predicate::str::contains(config_file.to_string_lossy()));
    drop(config_file);
    temp_dir.close()?;
    Ok(())
}

#[test]
fn change_theme() -> color_eyre::Result<()> {
    let (temp_dir, config_file) = setup_the_way()?;
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
    drop(config_file);
    temp_dir.close()?;
    Ok(())
}

fn add_snippet_interactive(config_file: &Path) -> color_eyre::Result<ReplSession> {
    let mut p = spawn_bash()?;
    p.send_line(&format!(
        "export THE_WAY_CONFIG={}",
        config_file.to_string_lossy()
    ))?;

    let executable = env!("CARGO_BIN_EXE_the-way");
    p.expect_prompt()?;
    p.send_line(&format!("{executable} config get"))?;
    p.expect(config_file.to_string_lossy().as_ref())?;
    p.expect_prompt()?;
    p.send_line(&format!("{executable} new"))?;
    p.expect("Description")?;
    p.send_line("test description 1")?;
    p.expect("Language")?;
    p.send_line("rust")?;
    p.expect("Tags")?;
    p.send_line("tag1 tag2")?;
    p.expect("Code snippet")?;
    p.send_line("code")?;
    p.expect("Snippet #1 added")?;
    p.expect_prompt()?;
    Ok(p)
}

fn add_two_snippets_interactive(config_file: &Path) -> color_eyre::Result<()> {
    let mut p = spawn_bash()?;
    p.send_line(&format!(
        "export THE_WAY_CONFIG={}",
        config_file.to_string_lossy()
    ))?;

    let executable = env!("CARGO_BIN_EXE_the-way");
    p.expect_prompt()?;
    p.send_line(&format!("{executable} config get"))?;
    p.expect(config_file.to_string_lossy().as_ref())?;
    p.send_line(&format!("{executable} new"))?;
    p.expect("Description")?;
    p.send_line("test description 1")?;
    p.expect("Language")?;
    p.send_line("rust")?;
    p.expect("Tags")?;
    p.send_line("tag1 tag2")?;
    p.expect("Code snippet")?;
    p.send_line("code")?;
    p.expect("Snippet #1 added")?;
    p.expect_prompt()?;
    p.send_line(&format!("{executable} new"))?;
    p.expect("Description")?;
    p.send_line("test description 2")?;
    p.expect("Language")?;
    p.send_line("python")?;
    p.expect("Tags")?;
    p.send_line("tag1 tag2")?;
    p.expect("Code snippet")?;
    p.send_line("code")?;
    p.expect("Snippet #2 added")?;
    Ok(())
}

fn change_snippet_interactive(config_file: &Path) -> color_eyre::Result<()> {
    let mut p = add_snippet_interactive(config_file)?;
    let executable = env!("CARGO_BIN_EXE_the-way");
    p.send_line(&format!("{executable} edit 1"))?;
    p.expect("Description")?;
    p.send_line("test description 2")?;
    p.expect("Language")?;
    p.send_line("")?;
    p.expect("Tags")?;
    p.send_line("")?;
    p.expect("Date")?;
    p.send_line("")?;
    p.expect("Edit snippet")?;
    p.send_line("")?;
    p.expect("Snippet #1 changed")?;
    p.expect_prompt()?;
    p.send_line(&format!("{executable} view 1"))?;
    p.expect("test description 2")?;
    Ok(())
}

fn add_two_cmd_snippets_interactive(config_file: &Path) -> color_eyre::Result<()> {
    let mut p = spawn_bash()?;
    p.send_line(&format!(
        "export THE_WAY_CONFIG={}",
        config_file.to_string_lossy()
    ))?;

    let executable = env!("CARGO_BIN_EXE_the-way");
    p.expect_prompt()?;
    p.send_line(&format!("{executable} config get"))?;
    p.expect(config_file.to_string_lossy().as_ref())?;
    // as argument
    p.send_line(&format!("{executable} cmd \"shell snippet 1\""))?;
    p.expect("Command")?;
    p.send_line("\n")?;
    p.expect("Description")?;
    p.send_line("test description 1")?;
    p.expect("Tags")?;
    p.send_line("tag1 tag2")?;
    p.expect("Snippet #1 added")?;
    p.expect_prompt()?;
    // interactively
    p.send_line(&format!("{executable} cmd"))?;
    p.expect("Command")?;
    p.send_line("shell snippet 2")?;
    p.expect("Description")?;
    p.send_line("test description 2")?;
    p.expect("Tags")?;
    p.send_line("tag1 tag2")?;
    p.expect("Snippet #2 added")?;
    Ok(())
}

#[ignore] // expensive, and change_snippet tests both
#[test]
fn add_snippet() -> color_eyre::Result<()> {
    let (temp_dir, config_file) = setup_the_way()?;
    assert!(add_snippet_interactive(&config_file).is_ok());
    drop(config_file);
    temp_dir.close()?;
    Ok(())
}

#[test]
fn add_two_snippets() -> color_eyre::Result<()> {
    let (temp_dir, config_file) = setup_the_way()?;
    add_two_snippets_interactive(&config_file).unwrap();
    drop(config_file);
    temp_dir.close()?;
    Ok(())
}

#[test]
fn add_two_cmd_snippets() -> color_eyre::Result<()> {
    let (temp_dir, config_file) = setup_the_way()?;
    add_two_cmd_snippets_interactive(&config_file).unwrap();
    drop(config_file);
    temp_dir.close()?;
    Ok(())
}

#[test]
fn change_snippet() -> color_eyre::Result<()> {
    let (temp_dir, config_file) = setup_the_way()?;
    assert!(change_snippet_interactive(&config_file).is_ok());
    drop(config_file);
    temp_dir.close()?;
    Ok(())
}

#[test]
fn import_single_show() -> color_eyre::Result<()> {
    let contents = r#"{"description":"test description","language":"rust","tags":["tag1","tag2"],"code":"some\ntest\ncode\n"}"#;

    let (temp_dir, config_file) = setup_the_way()?;
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
    drop(config_file);
    temp_dir.close()?;
    Ok(())
}

#[test]
fn import_multiple_no_tags() -> color_eyre::Result<()> {
    let contents_1 = r#"{"description":"test description 1","language":"rust","tags":["tag1","tag2"],"code":"some\ntest\ncode\n"}"#;
    let contents_2 =
        r#"{"description":"test description 2","language":"python","code":"some\ntest\ncode\n"}"#;
    let contents = format!("{contents_1}\n{contents_2}");

    let (temp_dir, config_file) = setup_the_way()?;
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
    drop(config_file);
    temp_dir.close()?;
    Ok(())
}

// This test is ignored because it tries to fetch a real Gist and runs into
// Github rate limits when ran by CI.
#[ignore]
#[test]
fn import_gist() -> color_eyre::Result<()> {
    let (temp_dir, config_file) = setup_the_way()?;
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
    drop(config_file);
    temp_dir.close()?;
    Ok(())
}

// This test is ignored because it tries to fetch a real Gist and runs into
// Github rate limits when ran by CI.
#[ignore]
#[test]
fn import_the_way_gist() -> color_eyre::Result<()> {
    use the_way::the_way::snippet::Snippet;
    let (temp_dir, config_file) = setup_the_way()?;
    let mut cmd = Command::cargo_bin("the-way")?;
    cmd.env("THE_WAY_CONFIG", &config_file)
        .arg("import")
        .arg("-w https://gist.github.com/Ninjani/c46ca310bfd7617f4ac4192130f33295")
        .assert()
        .success();

    // export
    let mut cmd = Command::cargo_bin("the-way")?;
    let file = temp_dir.path().join("snippets.json");
    cmd.env("THE_WAY_CONFIG", &config_file)
        .arg("export")
        .arg(file.to_str().unwrap())
        .assert()
        .success();

    // load exported snippets
    let snippets = serde_json::Deserializer::from_reader(fs::File::open(&file)?)
        .into_iter::<Snippet>()
        .collect::<Result<HashSet<_>, _>>()?;

    // load the-way gist snippets
    let test_snippets =
        serde_json::Deserializer::from_reader(fs::File::open("./tests/data/snippets.json")?)
            .into_iter::<Snippet>()
            .collect::<Result<HashSet<_>, _>>()?;

    assert_eq!(snippets, test_snippets);
    drop(config_file);
    temp_dir.close()?;
    Ok(())
}

#[test]
fn export() -> color_eyre::Result<()> {
    use the_way::the_way::snippet::Snippet;

    let contents_1 = r#"{"description":"test description 1","language":"rust","tags":["tag1","tag2"],"code":"some\ntest\ncode\n"}"#;
    let contents_2 =
        r#"{"description":"test description 2","language":"python","code":"some\ntest\ncode\n"}"#;
    let contents = format!("{contents_1}\n{contents_2}");
    let (temp_dir, config_file) = setup_the_way()?;

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
    drop(config_file);
    temp_dir.close()?;
    Ok(())
}

#[test]
fn delete() -> color_eyre::Result<()> {
    let contents_1 = r#"{"description":"test description 1","language":"rust","tags":["tag1","tag2"],"code":"some\ntest\ncode\n"}"#;
    let contents_2 =
        r#"{"description":"test description 2","language":"python","code":"some\ntest\ncode\n"}"#;
    let contents = format!("{contents_1}\n{contents_2}");
    let (temp_dir, config_file) = setup_the_way()?;
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
    drop(config_file);
    temp_dir.close()?;
    Ok(())
}

#[cfg(target_os = "macos")]
#[test]
fn copy() -> color_eyre::Result<()> {
    use clipboard::{ClipboardContext, ClipboardProvider};

    let contents = r#"{"description":"test description","language":"rust","tags":["tag1","tag2"],"code":"some\ntest\ncode\n"}"#;
    let (temp_dir, config_file) = setup_the_way()?;
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
    drop(config_file);
    temp_dir.close()?;
    Ok(())
}

#[cfg(target_os = "macos")]
#[test]
fn copy_shell_script() -> color_eyre::Result<()> {
    use clipboard::{ClipboardContext, ClipboardProvider};
    let (temp_dir, config_file) = setup_the_way()?;
    assert!(copy_shell_script_interactive(&config_file).is_ok());
    let ctx: color_eyre::Result<ClipboardContext, _> = ClipboardProvider::new();
    assert!(ctx.is_ok());
    let mut ctx = ctx.unwrap();
    let contents = ctx.get_contents();
    assert!(contents.is_ok());
    let contents = contents.unwrap();
    assert!(contents.contains("shell snippet value1 code value2 value1"));
    drop(config_file);
    temp_dir.close()?;
    Ok(())
}

fn copy_shell_script_interactive(config_file: &Path) -> color_eyre::Result<()> {
    let mut p = spawn_bash()?;
    p.send_line(&format!(
        "export THE_WAY_CONFIG={}",
        config_file.to_string_lossy()
    ))?;

    let executable = env!("CARGO_BIN_EXE_the-way");
    p.expect_prompt()?;
    p.send_line(&format!("{executable} config get"))?;
    p.expect(config_file.to_string_lossy().as_ref())?;
    // add a shell snippet
    p.send_line(&format!(
        "{} cmd \"shell snippet <param1=value1> code <param2> <param1>\"",
        executable
    ))?;
    p.expect("Command")?;
    p.send_line("\n")?;
    p.expect("Description")?;
    p.send_line("test description 1")?;
    p.expect("Tags")?;
    p.send_line("tag1 tag2")?;
    p.expect("Snippet #1 added")?;
    p.expect_prompt()?;
    // Test interactive copy
    p.send_line(&format!("{executable} cp 1"))?;
    p.expect("param1")?;
    p.send_line("\n")?;
    p.expect("param2")?;
    p.send_line("value2")?;
    p.expect("Snippet #1 copied to clipboard")?;
    Ok(())
}

fn make_gist(config_file: &Path, client: &GistClient) -> color_eyre::Result<Gist> {
    let contents_1 = r#"{"description":"test description 1","language":"rust","tags":["tag1","tag2"],"code":"code\nthe\nfirst\n"}"#;
    let contents_2 =
        r#"{"description":"test description 2","language":"python","code":"code\nthe\nsecond\n"}"#;
    let contents_3 =
        r#"{"description":"test description 3","language":"python","code":"code\nthe\nthird\n"}"#;
    let contents = format!("{contents_1}\n{contents_2}\n{contents_3}");

    // import
    let mut cmd = Command::cargo_bin("the-way")?;
    cmd.env("THE_WAY_CONFIG", config_file)
        .arg("import")
        .write_stdin(contents)
        .assert()
        .stdout(predicate::str::contains("Imported 3 snippets"));

    // sync (doesn't matter which one, this just makes a new Gist)
    let mut cmd = Command::cargo_bin("the-way")?;
    cmd.env("THE_WAY_CONFIG", config_file)
        .arg("sync")
        .arg("date")
        .assert()
        .success();

    // get gist_id from config
    std::env::set_var("THE_WAY_CONFIG", config_file);
    let config = TheWayConfig::load();
    assert!(config.is_ok());
    let config = config?;
    assert!(config.gist_id.is_some());

    // get Gist
    let gist = client.get_gist(&config.gist_id.unwrap());
    assert!(gist.is_ok());
    let gist = gist?;

    // check Gist contents
    assert_eq!(gist.files.len(), 4);
    for (filename, gistfile) in &gist.files {
        assert!(
            (filename.starts_with("snippet_1") && (gistfile.content == "code\nthe\nfirst\n"))
                || (filename.starts_with("snippet_2")
                    && (gistfile.content == "code\nthe\nsecond\n"))
                || (filename.starts_with("snippet_3")
                    && (gistfile.content == "code\nthe\nthird\n"))
                || filename.starts_with("index")
        );
    }
    Ok(gist)
}

fn sync_edit(config_file: &Path, gist: &Gist, client: &GistClient) -> color_eyre::Result<()> {
    // edit snippet_1 in Gist
    let update_payload = UpdateGistPayload {
        description: &gist.description,
        files: vec![(
            "snippet_1.rs".to_owned(),
            Some(GistContent {
                content: "code\nthe\nfirstx\n",
            }),
        )]
        .into_iter()
        .collect(),
    };
    assert!(client.update_gist(&gist.id, &update_payload).is_ok());

    // delete snippet_2 locally
    let mut cmd = Command::cargo_bin("the-way")?;
    cmd.env("THE_WAY_CONFIG", config_file)
        .arg("del")
        .arg("-f")
        .arg("2")
        .assert()
        .stdout(predicate::str::contains("Snippet #2 deleted"));

    // delete snippet_3 from Gist
    let update_payload = UpdateGistPayload {
        description: &gist.description,
        files: vec![("snippet_3.py".to_owned(), None)]
            .into_iter()
            .collect(),
    };
    assert!(client.update_gist(&gist.id, &update_payload).is_ok());

    // add snippet_4 to Gist
    let mut index_file_content = gist.files.get("index.md").unwrap().content.clone();
    index_file_content.push_str(&format!(
        "* [{}]({}#file-{}){}\n",
        "test description 4",
        gist.html_url,
        "snippet_4.txt",
        String::new()
    ));

    let add_payload = UpdateGistPayload {
        description: &gist.description,
        files: vec![
            (
                "snippet_4.txt".to_owned(),
                Some(GistContent {
                    content: "code\nthe\nfourth\n",
                }),
            ),
            (
                "index.md".to_owned(),
                Some(GistContent {
                    content: &index_file_content,
                }),
            ),
        ]
        .into_iter()
        .collect(),
    };
    assert!(client.update_gist(&gist.id, &add_payload).is_ok());

    // get Gist
    let gist = client.get_gist(&gist.id);
    assert!(gist.is_ok());
    let gist = gist?;

    // check Gist contents
    assert_eq!(gist.files.len(), 4);
    for (filename, gistfile) in &gist.files {
        assert!(
            (filename.starts_with("snippet_1") && (gistfile.content == "code\nthe\nfirstx\n"))
                || (filename.starts_with("snippet_2")
                    && (gistfile.content == "code\nthe\nsecond\n"))
                || (filename.starts_with("snippet_4")
                    && (gistfile.content == "code\nthe\nfourth\n"))
                || filename.starts_with("index")
        );
    }

    Ok(())
}

#[ignore]
#[test]
/// Tests `the-way sync date` functionality. Needs to have the environment variable $THE_WAY_GITHUB_TOKEN set!
/// Ignored by default since Travis doesn't allow secret/encrypted environment variables in PRs
fn sync_date() -> color_eyre::Result<()> {
    let (temp_dir, config_file) = setup_the_way()?;
    let token = &std::env::var("THE_WAY_GITHUB_TOKEN")?;
    let client = GistClient::new(Some(token))?;

    // make Gist with 3 snippets
    let gist = make_gist(&config_file, &client)?;
    // make edits: edit snippet_1 in Gist, delete snippet_2 locally, delete snippet_3 in Gist, add snippet_4 to Gist
    sync_edit(&config_file, &gist, &client)?;

    // sync - downloads snippet_1 locally + deletes snippet_2 in Gist + adds snippet_3 to Gist + deletes snippet_4 from Gist
    let mut cmd = Command::cargo_bin("the-way")?;
    cmd.env("THE_WAY_CONFIG", &config_file)
        .env("THE_WAY_GITHUB_TOKEN", token)
        .arg("sync")
        .arg("date")
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
        .env("THE_WAY_GITHUB_TOKEN", token)
        .arg("sync")
        .arg("date")
        .assert()
        .success();
    let gist = client.get_gist(&gist.id);
    assert!(gist.is_ok());
    let gist = gist?;
    assert!((*updated - gist.updated_at) < chrono::Duration::seconds(1));

    // check Gist contents
    assert_eq!(gist.files.len(), 3);
    for (filename, gistfile) in &gist.files {
        assert!(
            (filename.starts_with("snippet_1") && (gistfile.content == "code\nthe\nfirstx\n"))
                // adds snippet 3 to Gist
                || (filename.starts_with("snippet_3")
                    && (gistfile.content == "code\nthe\nthird\n"))
                || filename.starts_with("index") // deleted snippet_2 and snippet_4 from Gist
        );
    }

    // check snippet_1 downloaded
    let mut cmd = Command::cargo_bin("the-way")?;
    cmd.env("THE_WAY_CONFIG", &config_file)
        .arg("view")
        .arg("1")
        .assert()
        .stdout(predicate::str::contains("firstx"));

    // check snippet_4 not downloaded
    let mut cmd = Command::cargo_bin("the-way")?;
    cmd.env("THE_WAY_CONFIG", &config_file)
        .arg("view")
        .arg("4")
        .assert()
        .failure();

    // delete Gist
    assert!(client.delete_gist(&gist.id).is_ok());
    assert!(client.get_gist(&gist.id).is_err());
    drop(config_file);
    temp_dir.close()?;
    Ok(())
}

#[ignore]
#[test]
/// Tests `the-way sync local` functionality. Needs to have the environment variable $THE_WAY_GITHUB_TOKEN set!
/// Ignored by default since Travis doesn't allow secret/encrypted environment variables in PRs
fn sync_local() -> color_eyre::Result<()> {
    let (temp_dir, config_file) = setup_the_way()?;
    let token = &std::env::var("THE_WAY_GITHUB_TOKEN")?;
    let client = GistClient::new(Some(token))?;

    // make Gist with 3 snippets
    let gist = make_gist(&config_file, &client)?;
    // make edits: edit snippet_1 in Gist, delete snippet_2 locally, delete snippet_3 in Gist, adds snippet_4 to Gist
    sync_edit(&config_file, &gist, &client)?;

    // sync - uploads local snippet_1 to Gist + deletes snippet_2 from Gist + adds snippet_3 to Gist + deletes snippet_4 from Gist
    let mut cmd = Command::cargo_bin("the-way")?;
    cmd.env("THE_WAY_CONFIG", &config_file)
        .env("THE_WAY_GITHUB_TOKEN", token)
        .arg("sync")
        .arg("local")
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
        .env("THE_WAY_GITHUB_TOKEN", token)
        .arg("sync")
        .arg("local")
        .assert()
        .success();
    let gist = client.get_gist(&gist.id);
    assert!(gist.is_ok());
    let gist = gist?;
    assert!((*updated - gist.updated_at) < chrono::Duration::seconds(1));

    // check Gist contents
    assert_eq!(gist.files.len(), 3);
    for (filename, gistfile) in &gist.files {
        assert!(
            // uploaded local snippet_1 to Gist
            (filename.starts_with("snippet_1") && (gistfile.content == "code\nthe\nfirst\n"))
                // added snippet_3 to Gist
                || (filename.starts_with("snippet_3") && (gistfile.content == "code\nthe\nthird\n"))
                || filename.starts_with("index") // deleted snippet_2 and snippet_4 from Gist
        );
    }

    // check snippet_1 not changed
    let mut cmd = Command::cargo_bin("the-way")?;
    cmd.env("THE_WAY_CONFIG", &config_file)
        .env("THE_WAY_GITHUB_TOKEN", token)
        .arg("view")
        .arg("1")
        .assert()
        .stdout(predicate::str::contains("firstx").not());

    // check snippet_4 not downloaded
    let mut cmd = Command::cargo_bin("the-way")?;
    cmd.env("THE_WAY_CONFIG", &config_file)
        .env("THE_WAY_GITHUB_TOKEN", token)
        .arg("view")
        .arg("4")
        .assert()
        .failure();

    // delete Gist
    assert!(client.delete_gist(&gist.id).is_ok());
    assert!(client.get_gist(&gist.id).is_err());
    drop(config_file);
    temp_dir.close()?;
    Ok(())
}

#[ignore]
#[test]
/// Tests `the-way sync gist` functionality. Needs to have the environment variable $THE_WAY_GITHUB_TOKEN set!
/// Ignored by default since Travis doesn't allow secret/encrypted environment variables in PRs
fn sync_gist() -> color_eyre::Result<()> {
    let (temp_dir, config_file) = setup_the_way()?;

    let token = &std::env::var("THE_WAY_GITHUB_TOKEN")?;
    let client = GistClient::new(Some(token))?;

    // make Gist with 3 snippets
    let gist = make_gist(&config_file, &client)?;
    // make edits: edit snippet_1 in Gist, delete snippet_2 locally, delete snippet_3 in Gist, adds snippet_4 to Gist
    sync_edit(&config_file, &gist, &client)?;

    // sync - downloads snippet 1 locally + adds snippet 2 locally + deletes snippet 3 locally + downloads snippet 4 locally
    let mut cmd = Command::cargo_bin("the-way")?;
    cmd.env("THE_WAY_CONFIG", &config_file)
        .env("THE_WAY_GITHUB_TOKEN", token)
        .arg("sync")
        .arg("-f")
        .arg("gist")
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
        .env("THE_WAY_GITHUB_TOKEN", token)
        .arg("sync")
        .arg("gist")
        .assert()
        .success();
    let gist = client.get_gist(&gist.id);
    assert!(gist.is_ok());
    let gist = gist?;
    assert!((*updated - gist.updated_at) < chrono::Duration::seconds(1));

    // check Gist contents
    assert_eq!(gist.files.len(), 4);
    for (filename, gistfile) in &gist.files {
        assert!(
            (filename.starts_with("snippet_1") && (gistfile.content == "code\nthe\nfirstx\n"))
                || (filename.starts_with("snippet_2")
                    && (gistfile.content == "code\nthe\nsecond\n"))
                || (filename.starts_with("snippet_4")
                    && (gistfile.content == "code\nthe\nfourth\n"))
                || filename.starts_with("index")
        );
    }

    // downloaded snippet_1 locally
    let mut cmd = Command::cargo_bin("the-way")?;
    cmd.env("THE_WAY_CONFIG", &config_file)
        .env("THE_WAY_GITHUB_TOKEN", token)
        .arg("view")
        .arg("1")
        .assert()
        .stdout(predicate::str::contains("firstx"));

    // added snippet_2 locally
    let mut cmd = Command::cargo_bin("the-way")?;
    cmd.env("THE_WAY_CONFIG", &config_file)
        .arg("view")
        .arg("2")
        .assert()
        .stdout(predicate::str::contains("second"));

    // deleted snippet_3 locally
    let mut cmd = Command::cargo_bin("the-way")?;
    cmd.env("THE_WAY_CONFIG", &config_file)
        .arg("view")
        .arg("3")
        .assert()
        .failure();

    // added snippet_4 locally
    let mut cmd = Command::cargo_bin("the-way")?;
    cmd.env("THE_WAY_CONFIG", &config_file)
        .arg("view")
        .arg("4")
        .assert()
        .stdout(predicate::str::contains("fourth"));

    // delete Gist
    assert!(client.delete_gist(&gist.id).is_ok());
    assert!(client.get_gist(&gist.id).is_err());
    drop(config_file);
    temp_dir.close()?;
    Ok(())
}
