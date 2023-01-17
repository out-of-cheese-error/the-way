//! CLI code
use std::collections::HashMap;
use std::io::{ErrorKind, Write};
use std::path::Path;
use std::{fs, io, process};

use clap::CommandFactory;
use clap_complete::Shell;
use color_eyre::Help;
use dialoguer::theme::ColorfulTheme;
use dialoguer::{Confirm, Select};

use crate::configuration::{ConfigCommand, TheWayConfig};
use crate::errors::LostTheWay;
use crate::language::{CodeHighlight, Language};
use crate::the_way::{
    cli::{SyncCommand, TheWayCLI, TheWaySubcommand, ThemeCommand},
    filter::Filters,
    snippet::Snippet,
};
use crate::utils;

pub mod cli;
mod database;
mod filter;
mod gist;
mod search;
pub mod snippet;

/// Stores
/// - project directory information from `directories`
/// - argument parsing information from `clap`
/// - the `sled` databases storing linkage information between languages, tags, and snippets
pub struct TheWay {
    /// stores the main project directory, the themes directory, and the currently set theme
    config: TheWayConfig,
    /// database storing snippets and links to languages and tags
    db: sled::Db,
    /// Maps a language name to its color and extension
    languages: HashMap<String, Language>,
    /// for `syntect` code highlighting
    highlighter: CodeHighlight,
    /// colorize output even if terminal is not in tty mode
    colorize: bool,
    /// don't colorize output even if terminal is in tty mode
    plain: bool,
}

// All command-line related functions
impl TheWay {
    /// Initialize program with command line input.
    /// Reads `sled` trees and metadata file from the locations specified in config.
    /// (makes new ones the first time).
    pub fn start(cli: TheWayCLI, languages: HashMap<String, Language>) -> color_eyre::Result<()> {
        if let TheWaySubcommand::Config {
            cmd: ConfigCommand::Default { file },
        } = &cli.cmd
        {
            TheWayConfig::default_config(file.as_deref())?;
            return Ok(());
        }

        let config = TheWayConfig::load()?;
        let mut the_way = Self {
            db: Self::get_db(&config.db_dir)?,
            languages,
            highlighter: CodeHighlight::new(&config.theme, config.themes_dir.clone())?,
            config,
            colorize: cli.colorize,
            plain: cli.plain,
        };
        the_way.set_merge()?;
        the_way.run(cli)?;
        Ok(())
    }

    fn run(&mut self, cli: TheWayCLI) -> color_eyre::Result<()> {
        self.colorize = cli.colorize;
        self.plain = cli.plain;
        match cli.cmd {
            TheWaySubcommand::New => self.the_way(),
            TheWaySubcommand::Cmd { code } => self.the_way_cmd(code),
            TheWaySubcommand::Search {
                filters,
                stdout,
                exact,
            } => self.search(&filters, stdout, exact),
            TheWaySubcommand::Cp { index, stdout } => self.copy(index, stdout),
            TheWaySubcommand::Edit { index } => self.edit(index),
            TheWaySubcommand::Del { index, force } => self.delete(index, force),
            TheWaySubcommand::View { index } => self.view(index),
            TheWaySubcommand::List { filters } => self.list(&filters),
            TheWaySubcommand::Import {
                file,
                gist_url,
                the_way_url,
            } => self.import(file.as_deref(), gist_url, the_way_url),
            TheWaySubcommand::Export { filters, file } => self.export(&filters, file.as_deref()),
            TheWaySubcommand::Complete { shell } => {
                Self::complete(shell);
                Ok(())
            }
            TheWaySubcommand::Themes { cmd } => self.themes(cmd),
            TheWaySubcommand::Clear { force } => self.clear(force),
            TheWaySubcommand::Config { cmd } => match cmd {
                ConfigCommand::Default { file } => TheWayConfig::default_config(file.as_deref()), //Already handled
                ConfigCommand::Get => TheWayConfig::print_config_location(),
            },
            TheWaySubcommand::Sync { cmd, force } => self.sync(cmd, force),
        }
    }

    /// Adds a new snippet
    fn the_way(&mut self) -> color_eyre::Result<()> {
        let snippet =
            Snippet::from_user(self.get_current_snippet_index()? + 1, &self.languages, None)?;
        let index = self.add_snippet(&snippet)?;
        self.color_print(&format!("Snippet #{index} added\n"))?;
        self.increment_snippet_index()?;
        Ok(())
    }

    /// Adds a new shell snippet
    fn the_way_cmd(&mut self, code: Option<String>) -> color_eyre::Result<()> {
        let snippet =
            Snippet::cmd_from_user(self.get_current_snippet_index()? + 1, code.as_deref())?;
        let index = self.add_snippet(&snippet)?;
        self.color_print(&format!("Snippet #{index} added\n"))?;
        self.increment_snippet_index()?;
        Ok(())
    }

    /// Delete a snippet (and all associated data) from the trees and metadata
    fn delete(&mut self, index: usize, force: bool) -> color_eyre::Result<()> {
        if force
            || Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt(&format!("Delete snippet #{index}?\n"))
                .default(false)
                .interact()?
        {
            self.delete_snippet(index)?;
            self.color_print(&format!("Snippet #{index} deleted\n"))?;
            Ok(())
        } else {
            let error: color_eyre::Result<()> = Err(LostTheWay::DoingNothing.into());
            error.suggestion("Press Y next time!")
        }
    }

    /// Modify a stored snippet's information
    fn edit(&mut self, index: usize) -> color_eyre::Result<()> {
        let old_snippet = self.get_snippet(index)?;
        let new_snippet = Snippet::from_user(index, &self.languages, Some(&old_snippet))?;
        self.delete_snippet(index)?;
        self.add_snippet(&new_snippet)?;
        self.color_print(&format!("Snippet #{index} changed\n"))?;
        Ok(())
    }

    /// Pretty prints a snippet to terminal
    fn view(&self, index: usize) -> color_eyre::Result<()> {
        let snippet = self.get_snippet(index)?;
        utils::smart_print(
            &snippet.pretty_print(
                &self.highlighter,
                self.languages
                    .get(&snippet.language)
                    .unwrap_or(&Language::default()),
            )?,
            false,
            self.colorize,
            self.plain,
        )?;
        Ok(())
    }

    /// Copy a snippet to clipboard
    fn copy(&self, index: usize, to_stdout: bool) -> color_eyre::Result<()> {
        let snippet = self.get_snippet(index)?;
        let code = snippet.fill_snippet(self.highlighter.selection_style)?;
        if to_stdout {
            // See https://github.com/rust-lang/rust/issues/46016
            if let Err(e) = writeln!(io::stdout(), "{code}") {
                if e.kind() != ErrorKind::BrokenPipe {
                    eprintln!("{e}");
                    process::exit(1);
                }
            }
        } else {
            utils::copy_to_clipboard(&self.config.copy_cmd, &code)?;
            eprintln!(
                "{}",
                utils::highlight_string(
                    &format!("Snippet #{index} copied to clipboard\n"),
                    self.highlighter.main_style
                )
            );
        }
        Ok(())
    }

    /// Import from file or gist
    fn import(
        &mut self,
        file: Option<&Path>,
        gist_url: Option<String>,
        the_way_url: Option<String>,
    ) -> color_eyre::Result<()> {
        let mut num = 0;
        match (gist_url, the_way_url) {
            (Some(gist_url), None) => {
                let snippets = self.import_gist(&gist_url)?;
                num = snippets.len();
            }
            (None, Some(the_way_url)) => {
                let snippets = self.import_the_way_gist(&the_way_url)?;
                num += snippets.len();
            }
            (None, None) => {
                for mut snippet in self.import_file(file)? {
                    snippet.index = self.get_current_snippet_index()? + 1;
                    self.add_snippet(&snippet)?;
                    self.increment_snippet_index()?;
                    num += 1;
                }
            }
            _ => {
                return Err(LostTheWay::OutOfCheeseError {
                    message: "the-way called with both gist_url and the_way_url".into(),
                }
                .into());
            }
        }
        self.color_print(&format!("Imported {num} snippets\n"))?;
        Ok(())
    }

    /// Imports snippets from a JSON file (ignores indices and appends to existing snippets)
    /// TODO: It may be nice to check for duplicates somehow, too expensive?
    fn import_file(&self, file: Option<&Path>) -> color_eyre::Result<Vec<Snippet>> {
        let reader: Box<dyn io::Read> = match file {
            Some(file) => Box::new(fs::File::open(file)?),
            None => Box::new(io::stdin()),
        };
        let mut buffered = io::BufReader::new(reader);
        let mut snippets = Snippet::read(&mut buffered).collect::<Result<Vec<_>, _>>()?;
        for snippet in &mut snippets {
            snippet.set_extension(&snippet.language.clone(), &self.languages);
        }
        Ok(snippets)
    }

    /// Saves (optionally filtered) snippets to a JSON file
    fn export(&self, filters: &Filters, file: Option<&Path>) -> color_eyre::Result<()> {
        let writer: Box<dyn io::Write> = match file {
            Some(file) => Box::new(fs::File::create(file)?),
            None => Box::new(io::stdout()),
        };
        let mut buffered = io::BufWriter::new(writer);
        for snippet in self.filter_snippets(filters)? {
            snippet.to_json(&mut buffered)?;
            buffered.write_all(b"\n")?;
        }
        Ok(())
    }

    /// Prints given snippets in full
    fn show_snippets(&self, snippets: &[Snippet]) -> color_eyre::Result<()> {
        let mut colorized = Vec::new();
        let default_language = Language::default();
        for snippet in snippets {
            colorized.extend_from_slice(
                &snippet.pretty_print(
                    &self.highlighter,
                    self.languages
                        .get(&snippet.language)
                        .unwrap_or(&default_language),
                )?,
            );
        }
        utils::smart_print(&colorized, false, self.colorize, self.plain)?;
        Ok(())
    }

    /// Lists snippets (optionally filtered)
    fn list(&self, filters: &Filters) -> color_eyre::Result<()> {
        let mut snippets = self.filter_snippets(filters)?;
        snippets.sort_by(|a, b| a.index.cmp(&b.index));
        self.show_snippets(&snippets)?;
        Ok(())
    }

    /// Displays all snippet descriptions in a skim fuzzy search window
    /// A preview window on the right shows the indices of snippets matching the query
    fn search(&mut self, filters: &Filters, stdout: bool, exact: bool) -> color_eyre::Result<()> {
        let mut snippets = self.filter_snippets(filters)?;
        snippets.sort_by(|a, b| a.index.cmp(&b.index));
        self.make_search(
            snippets,
            self.highlighter.skim_theme.clone(),
            self.highlighter.selection_style,
            stdout,
            exact,
        )?;
        Ok(())
    }

    /// Generates shell completions
    fn complete(shell: Shell) {
        let mut cmd = TheWayCLI::command();
        clap_complete::generate(shell, &mut cmd, utils::NAME, &mut io::stdout());
    }

    /// Removes all `sled` trees
    fn clear(&self, force: bool) -> color_eyre::Result<()> {
        if force
            || Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt("Clear all data?")
                .default(false)
                .interact()?
        {
            for path in fs::read_dir(&self.config.db_dir)? {
                let path = path?.path();
                if path.is_dir() {
                    fs::remove_dir_all(path)?;
                } else {
                    fs::remove_file(path)?;
                }
            }
            self.reset_index()?;
            self.color_print("Data cleared.\n")?;
            Ok(())
        } else {
            let error: color_eyre::Result<()> = Err(LostTheWay::DoingNothing.into());
            error.suggestion("Press Y next time!")
        }
    }

    /// Syncs snippets to Gist
    fn sync(&mut self, cmd: SyncCommand, force: bool) -> color_eyre::Result<()> {
        // Take token from environment variable or config file
        let mut github_access_token = std::env::var("THE_WAY_GITHUB_TOKEN")
            .ok()
            .or_else(|| self.config.github_access_token.clone());
        // Get token from user if not set
        if github_access_token.is_none() {
            self.color_print("Get a GitHub access token from https://github.com/settings/tokens/new (add the \"gist\" scope)\n\n")?;
            github_access_token = Some(
                dialoguer::Password::with_theme(&ColorfulTheme::default())
                    .with_prompt("GitHub access token")
                    .interact()?,
            );
            if utils::confirm("Save to config?", false)? {
                self.config.github_access_token = github_access_token.clone();
            }
        }
        if self.config.gist_id.is_some() {
            self.sync_gist(github_access_token.as_deref(), cmd, force)?;
        } else {
            self.config.gist_id = Some(self.make_gist(github_access_token.as_ref().unwrap())?);
        }
        self.config.store()?;
        Ok(())
    }

    fn themes(&mut self, cmd: ThemeCommand) -> color_eyre::Result<()> {
        match cmd {
            ThemeCommand::Set { theme } => {
                let theme = if let Some(theme) = theme {
                    theme
                } else {
                    let themes = self.highlighter.get_themes();
                    let theme_index =
                        Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
                            .with_prompt("Choose a syntax highlighting theme:")
                            .items(&themes[..])
                            .interact()?;
                    themes[theme_index].clone()
                };
                self.highlighter.set_theme(theme.clone())?;
                self.color_print(&format!("Theme changed to {theme}\n"))?;
                self.config.theme = theme;
                self.config.store()?;
                Ok(())
            }
            ThemeCommand::Add { file } => {
                let theme = self.highlighter.add_theme(&file)?;
                self.color_print(&format!("Added theme {theme}\n"))?;
                Ok(())
            }
            ThemeCommand::Language { file } => {
                let language = self.highlighter.add_syntax(&file)?;
                self.color_print(&format!("Added {language} syntax\n"))?;
                Ok(())
            }
            ThemeCommand::Get => {
                self.color_print(&format!(
                    "Current theme: {}\n",
                    self.highlighter.get_theme_name()
                ))?;
                Ok(())
            }
        }
    }

    /// Adds some color to logging output, uses selected theme
    pub(crate) fn color_print(&self, input: &str) -> color_eyre::Result<()> {
        utils::smart_print(
            &[(self.highlighter.main_style, input.to_string())],
            false,
            self.colorize,
            self.plain,
        )?;
        Ok(())
    }
}
