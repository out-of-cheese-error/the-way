//! CLI code
use std::collections::HashMap;
use std::path::Path;
use std::{fs, io};

use color_eyre::Help;
use dialoguer::Confirm;
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
    /// StructOpt struct
    cli: TheWayCLI,
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
        if let TheWayCLI::Config { cmd } = &cli {
            if let ConfigCommand::Default { file } = cmd {
                TheWayConfig::default_config(file.as_deref())?;
                return Ok(());
            }
        }
        let config = TheWayConfig::load()?;
        let mut the_way = Self {
            db: Self::get_db(&config.db_dir)?,
            cli,
            languages,
            highlighter: CodeHighlight::new(&config.theme, config.themes_dir.clone())?,
            config,
        };
        the_way.set_merge()?;
        the_way.run()?;
        Ok(())
    }

    fn run(&mut self) -> color_eyre::Result<()> {
        match &self.cli {
            TheWayCLI::New => self.the_way(),
            TheWayCLI::Search { filters } => self.search(filters),
            TheWayCLI::Cp { index } => self.copy(*index),
            TheWayCLI::Edit { index } => {
                let index = *index;
                self.edit(index)
            }
            TheWayCLI::Del { index, force } => {
                let (index, force) = (*index, *force);
                self.delete(index, force)
            }
            TheWayCLI::View { index } => self.view(*index),
            TheWayCLI::List { filters } => self.list(filters),
            TheWayCLI::Import { file } => {
                let mut num = 0;
                for mut snippet in self.import(file.as_deref())? {
                    snippet.index = self.get_current_snippet_index()? + 1;
                    self.add_snippet(&snippet)?;
                    self.increment_snippet_index()?;
                    num += 1;
                }
                println!("Imported {} snippets", num);
                Ok(())
            }
            TheWayCLI::Export { filters, file } => self.export(filters, file.as_deref()),
            TheWayCLI::Complete { shell } => self.complete(*shell),
            TheWayCLI::Themes { cmd } => match cmd {
                ThemeCommand::List => self.list_themes(),
                ThemeCommand::Set { theme } => {
                    self.highlighter.set_theme(theme.to_owned())?;
                    self.config.theme = theme.to_owned();
                    self.config.store()?;
                    Ok(())
                }
                ThemeCommand::Add { file } => {
                    self.highlighter.add_theme(file)?;
                    Ok(())
                }
                ThemeCommand::Get => self.get_theme(),
            },
            TheWayCLI::Clear { force } => self.clear(*force),
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
        println!("Added snippet #{}", self.add_snippet(&snippet)?);
        self.increment_snippet_index()?;
        Ok(())
    }

    /// Delete a snippet (and all associated data) from the trees and metadata
    fn delete(&mut self, index: usize, force: bool) -> color_eyre::Result<()> {
        if force
            || Confirm::new()
                .with_prompt(&format!("Delete snippet #{}?", index))
                .default(false)
                .interact()?
        {
            self.delete_snippet(index)?;
            println!("Snippet #{} deleted", index);
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
        println!("Snippet #{} changed", index);
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
        )? {
            print!("{}", line)
        }
        Ok(())
    }

    /// Copy a snippet to clipboard
    fn copy(&self, index: usize) -> color_eyre::Result<()> {
        let snippet = self.get_snippet(index)?;
        utils::copy_to_clipboard(snippet.code)?;
        println!("Snippet #{} copied to clipboard", index);
        Ok(())
    }

    /// List syntax highlighting themes
    fn list_themes(&self) -> color_eyre::Result<()> {
        for theme in self.highlighter.get_themes() {
            println!("{}", theme);
        }
        Ok(())
    }

    /// Print current syntax highlighting theme
    fn get_theme(&self) -> color_eyre::Result<()> {
        println!("{}", self.highlighter.get_theme_name());
        Ok(())
    }

    /// Imports snippets from a JSON file (ignores indices and appends to existing snippets)
    /// TODO: It may be nice to check for duplicates somehow, too expensive?
    fn import(&self, file: Option<&Path>) -> color_eyre::Result<Vec<Snippet>> {
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
        self.filter_snippets(filters)?
            .into_iter()
            .map(|snippet| snippet.to_json(&mut buffered))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(())
    }

    /// Lists snippets (optionally filtered)
    fn list(&self, filters: &Filters) -> color_eyre::Result<()> {
        let snippets = self.filter_snippets(filters)?;

        let mut colorized = Vec::new();
        let default_language = Language::default();
        for snippet in &snippets {
            colorized.extend_from_slice(
                &snippet.pretty_print(
                    &self.highlighter,
                    self.languages
                        .get(&snippet.language)
                        .unwrap_or(&default_language),
                )?,
            );
        }
        for line in colorized {
            print!("{}", line);
        }
        Ok(())
    }

    /// Displays all snippet descriptions in a skim fuzzy search window
    /// A preview window on the right shows the indices of snippets matching the query
    fn search(&self, filters: &Filters) -> color_eyre::Result<()> {
        let snippets = self.filter_snippets(&filters)?;
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
        )?;
        Ok(())
    }

    /// Generates shell completions
    fn complete(&self, shell: Shell) -> color_eyre::Result<()> {
        TheWayCLI::clap().gen_completions_to(utils::NAME, shell, &mut io::stdout());
        Ok(())
    }

    /// Removes all `sled` trees
    fn clear(&self, force: bool) -> color_eyre::Result<()> {
        if force
            || Confirm::new()
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
            Ok(())
        } else {
            let error: color_eyre::Result<()> = Err(LostTheWay::DoingNothing.into());
            error.suggestion("Press Y next time!")
        }
    }
}
