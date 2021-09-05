//! CLI code
use std::collections::HashMap;
use std::io::{ErrorKind, Write};
use std::path::Path;
use std::{fs, io, process};

use color_eyre::Help;
use dialoguer::theme::ColorfulTheme;
use dialoguer::{Confirm, Select};
use structopt::clap::Shell;
use structopt::StructOpt;

use crate::configuration::{ConfigCommand, TheWayConfig};
use crate::errors::LostTheWay;
use crate::language::{CodeHighlight, Language};
use crate::the_way::{
    cli::{TheWayCLI, ThemeCommand},
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
}

// All command-line related functions
impl TheWay {
    /// Initialize program with command line input.
    /// Reads `sled` trees and metadata file from the locations specified in config.
    /// (makes new ones the first time).
    pub fn start(cli: TheWayCLI, languages: HashMap<String, Language>) -> color_eyre::Result<()> {
        if let TheWayCLI::Config {
            cmd: ConfigCommand::Default { file },
        } = &cli
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
        };
        the_way.set_merge()?;
        the_way.run(cli)?;
        Ok(())
    }

    fn run(&mut self, cli: TheWayCLI) -> color_eyre::Result<()> {
        match cli {
            TheWayCLI::New => self.the_way(),
            TheWayCLI::Cmd { code } => self.the_way_cmd(code),
            TheWayCLI::Search { filters, stdout } => self.search(&filters, stdout),
            TheWayCLI::Cp { index, stdout } => self.copy(index, stdout),
            TheWayCLI::Edit { index } => self.edit(index),
            TheWayCLI::Del { index, force } => self.delete(index, force),
            TheWayCLI::View { index } => self.view(index),
            TheWayCLI::List { filters } => self.list(&filters),
            TheWayCLI::Import { file, gist_url } => self.import(file.as_deref(), gist_url),
            TheWayCLI::Export { filters, file } => self.export(&filters, file.as_deref()),
            TheWayCLI::Complete { shell } => {
                Self::complete(shell);
                Ok(())
            }
            TheWayCLI::Themes { cmd } => self.themes(cmd),
            TheWayCLI::Clear { force } => self.clear(force),
            TheWayCLI::Config { cmd } => match cmd {
                ConfigCommand::Default { file } => TheWayConfig::default_config(file.as_deref()), //Already handled
                ConfigCommand::Get => TheWayConfig::print_config_location(),
            },
            TheWayCLI::Sync => self.sync(),
        }
    }

    /// Adds a new snippet
    fn the_way(&mut self) -> color_eyre::Result<()> {
        let snippet =
            Snippet::from_user(self.get_current_snippet_index()? + 1, &self.languages, None)?;
        let index = self.add_snippet(&snippet)?;
        println!(
            "{}",
            self.highlight_string(&format!("Snippet #{} added", index))
        );
        self.increment_snippet_index()?;
        Ok(())
    }

    /// Adds a new shell snippet
    fn the_way_cmd(&mut self, code: Option<String>) -> color_eyre::Result<()> {
        let snippet =
            Snippet::cmd_from_user(self.get_current_snippet_index()? + 1, code.as_deref())?;
        let index = self.add_snippet(&snippet)?;
        println!(
            "{}",
            self.highlight_string(&format!("Snippet #{} added", index))
        );
        self.increment_snippet_index()?;
        Ok(())
    }

    /// Delete a snippet (and all associated data) from the trees and metadata
    fn delete(&mut self, index: usize, force: bool) -> color_eyre::Result<()> {
        if force
            || Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt(&format!("Delete snippet #{}?", index))
                .default(false)
                .interact()?
        {
            self.delete_snippet(index)?;
            println!(
                "{}",
                self.highlight_string(&format!("Snippet #{} deleted", index))
            );
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
        println!(
            "{}",
            self.highlight_string(&format!("Snippet #{} changed", index))
        );
        Ok(())
    }

    /// Pretty prints a snippet to terminal
    fn view(&self, index: usize) -> color_eyre::Result<()> {
        let snippet = self.get_snippet(index)?;
        for line in snippet.pretty_print(
            &self.highlighter,
            self.languages
                .get(&snippet.language)
                .unwrap_or(&Language::default()),
        ) {
            print!("{}", line)
        }
        Ok(())
    }

    /// Copy a snippet to clipboard
    fn copy(&self, index: usize, to_stdout: bool) -> color_eyre::Result<()> {
        let snippet = self.get_snippet(index)?;
        let code = snippet.fill_snippet(self.highlighter.highlight_style)?;
        if to_stdout {
            // See https://github.com/rust-lang/rust/issues/46016
            let mut stdout = std::io::stdout();
            if let Err(e) = writeln!(stdout, "{}", code) {
                if e.kind() != ErrorKind::BrokenPipe {
                    eprintln!("{}", e);
                    process::exit(1);
                }
            }
        } else {
            utils::copy_to_clipboard(&code)?;
            eprintln!(
                "{}",
                self.highlight_string(&format!("Snippet #{} copied to clipboard", index))
            );
        }
        Ok(())
    }

    /// Import from file or gist
    fn import(&mut self, file: Option<&Path>, gist_url: Option<String>) -> color_eyre::Result<()> {
        let mut num = 0;
        if let Some(gist_url) = gist_url {
            let snippets = self.import_gist(&gist_url)?;
            num = snippets.len();
        } else {
            for mut snippet in self.import_file(file)? {
                snippet.index = self.get_current_snippet_index()? + 1;
                self.add_snippet(&snippet)?;
                self.increment_snippet_index()?;
                num += 1;
            }
        }
        println!(
            "{}",
            self.highlight_string(&format!("Imported {} snippets", num))
        );
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
            snippet.set_extension(&snippet.language.to_owned(), &self.languages);
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
    fn show_snippets(&self, snippets: &[Snippet]) {
        let mut colorized = Vec::new();
        let default_language = Language::default();
        for snippet in snippets {
            colorized.extend_from_slice(
                &snippet.pretty_print(
                    &self.highlighter,
                    self.languages
                        .get(&snippet.language)
                        .unwrap_or(&default_language),
                ),
            );
        }
        for line in colorized {
            print!("{}", line);
        }
    }

    /// Lists snippets (optionally filtered)
    fn list(&self, filters: &Filters) -> color_eyre::Result<()> {
        let mut snippets = self.filter_snippets(filters)?;
        snippets.sort_by(|a, b| a.index.cmp(&b.index));
        self.show_snippets(&snippets);
        Ok(())
    }

    /// Displays all snippet descriptions in a skim fuzzy search window
    /// A preview window on the right shows the indices of snippets matching the query
    fn search(&mut self, filters: &Filters, stdout: bool) -> color_eyre::Result<()> {
        let mut snippets = self.filter_snippets(filters)?;
        snippets.sort_by(|a, b| a.index.cmp(&b.index));
        self.make_search(
            snippets,
            &format!(
                "#{}",
                hex::encode(vec![
                    self.highlighter.highlight_style.foreground.r,
                    self.highlighter.highlight_style.foreground.g,
                    self.highlighter.highlight_style.foreground.b,
                ])
            ),
            stdout,
        )?;
        Ok(())
    }

    /// Generates shell completions
    fn complete(shell: Shell) {
        TheWayCLI::clap().gen_completions_to(utils::NAME, shell, &mut io::stdout());
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
            println!("{}", self.highlight_string("Data cleared."));
            Ok(())
        } else {
            let error: color_eyre::Result<()> = Err(LostTheWay::DoingNothing.into());
            error.suggestion("Press Y next time!")
        }
    }

    /// Syncs snippets to Gist
    fn sync(&mut self) -> color_eyre::Result<()> {
        // Check if environment variable has changed
        self.config.github_access_token = std::env::var("THE_WAY_GITHUB_TOKEN")
            .ok()
            .or_else(|| self.config.github_access_token.clone());
        // Get token from user if not set
        if self.config.github_access_token.is_none() {
            println!(
                "{}",
                self.highlight_string("Get a GitHub access token from https://github.com/settings/tokens/new (add the \"gist\" scope)\n",
                )
            );
            self.config.github_access_token = Some(
                dialoguer::Password::with_theme(&ColorfulTheme::default())
                    .with_prompt("GitHub access token")
                    .interact()?,
            );
        }
        if self.config.gist_id.is_some() {
            self.sync_gist()?;
        } else {
            self.config.gist_id =
                Some(self.make_gist(self.config.github_access_token.as_ref().unwrap())?);
        }
        self.config.store()?;
        Ok(())
    }

    fn themes(&mut self, cmd: ThemeCommand) -> color_eyre::Result<()> {
        match cmd {
            ThemeCommand::Set { theme } => {
                let theme = match theme {
                    Some(theme) => theme,
                    None => {
                        let themes = self.highlighter.get_themes();
                        let theme_index =
                            Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
                                .with_prompt("Choose a syntax highlighting theme:")
                                .items(&themes[..])
                                .interact()?;
                        themes[theme_index].to_owned()
                    }
                };
                self.highlighter.set_theme(theme.to_owned())?;
                println!(
                    "{}",
                    self.highlight_string(&format!("Theme changed to {}", theme))
                );
                self.config.theme = theme;
                self.config.store()?;
                Ok(())
            }
            ThemeCommand::Add { file } => {
                let theme = self.highlighter.add_theme(&file)?;
                println!(
                    "{}",
                    self.highlight_string(&format!("Added theme {}", theme))
                );
                Ok(())
            }
            ThemeCommand::Language { file } => {
                let language = self.highlighter.add_syntax(&file)?;
                println!(
                    "{}",
                    self.highlight_string(&format!("Added {} syntax", language))
                );
                Ok(())
            }
            ThemeCommand::Get => {
                println!(
                    "{}",
                    self.highlight_string(&format!(
                        "Current theme: {}",
                        self.highlighter.get_theme_name()
                    ))
                );
                Ok(())
            }
        }
    }

    pub(crate) fn highlight_string(&self, input: &str) -> String {
        utils::highlight_string(input, self.highlighter.main_style)
    }
}
