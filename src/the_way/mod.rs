use std::collections::HashMap;
use std::io;

use anyhow::Error;
use clap::{load_yaml, App};
use clap::{ArgMatches, Shell};
use path_abs::{PathDir, PathFile};

use crate::configuration::TheWayConfig;
use crate::errors::LostTheWay;
use crate::language::{CodeHighlight, Language};
use crate::the_way::filter::Filters;
use crate::the_way::snippet::Snippet;
use crate::utils;

mod database;
mod filter;
mod search;
mod snippet;

/// Stores
/// - project directory information from `directories`
/// - argument parsing information from `clap`
/// - the `sled` databases storing linkage information between languages, tags, and snippets
pub struct TheWay<'a> {
    config: TheWayConfig,
    matches: ArgMatches<'a>,
    db: sled::Db,
    languages: HashMap<String, Language>,
    highlighter: CodeHighlight,
}

// All command-line related functions
impl<'a> TheWay<'a> {
    /// Initialize program with command line input.
    /// Reads `sled` trees and metadata file from the locations specified in config.
    /// (makes new ones the first time).
    pub(crate) fn start(
        matches: ArgMatches<'a>,
        languages: HashMap<String, Language>,
    ) -> Result<(), Error> {
        let config = TheWayConfig::get()?;
        let mut the_way = Self {
            db: Self::get_db(&config.db_dir)?,
            matches,
            languages,
            highlighter: CodeHighlight::new(&config.theme, config.themes_dir.clone())?,
            config,
        };
        the_way.set_merge()?;
        the_way.run()?;
        // the_way.debug();
        Ok(())
    }

    /// Parses command-line arguments to decide which sub-command to run
    fn run(&mut self) -> Result<(), Error> {
        if self.matches.is_present("delete") {
            self.delete()
        } else if self.matches.is_present("show") {
            self.show()
        } else if self.matches.is_present("change") {
            self.change()
        } else if self.matches.is_present("copy") {
            self.copy()
        } else {
            match self.matches.subcommand() {
                ("import", Some(matches)) => {
                    for mut snippet in self.import(matches)? {
                        snippet.index = self.get_current_snippet_index()? + 1;
                        self.add_snippet(&snippet)?;
                    }
                    Ok(())
                }
                ("export", Some(matches)) => self.export(matches),
                ("list", Some(matches)) => self.list(matches),
                ("search", Some(matches)) => self.search(matches),
                ("themes", Some(matches)) => match matches.subcommand() {
                    ("set", Some(matches)) => {
                        let theme_name = utils::get_argument_value("theme", matches)?.ok_or(
                            LostTheWay::OutOfCheeseError {
                                message: "Argument THEME not used".into(),
                            },
                        )?;
                        self.highlighter.set_theme(theme_name.to_owned())?;
                        self.config.theme = theme_name.to_owned();
                        self.config.store()?;
                        Ok(())
                    }
                    ("add", Some(matches)) => {
                        let theme_file = utils::get_argument_value("file", matches)?.ok_or(
                            LostTheWay::OutOfCheeseError {
                                message: "Argument FILE not used".into(),
                            },
                        )?;
                        let theme_file = PathFile::new(theme_file)?;
                        self.highlighter.add_theme(&theme_file)?;
                        Ok(())
                    }
                    _ => self.list_themes()
                },
                ("clear", Some(_)) => self.clear(),
                ("complete", Some(matches)) => self.complete(matches),
                _ => self.the_way(),
            }
        }
    }

    fn the_way(&mut self) -> Result<(), Error> {
        let snippet =
            Snippet::from_user(self.get_current_snippet_index()? + 1, &self.languages, None)?;
        println!("Added snippet #{}", self.add_snippet(&snippet)?);
        Ok(())
    }

    /// Delete a snippet (and all associated data) from the trees and metadata
    fn delete(&mut self) -> Result<(), Error> {
        let index = utils::get_argument_value("delete", &self.matches)?.ok_or(
            LostTheWay::OutOfCheeseError {
                message: "Argument delete not used".into(),
            },
        )?;
        let mut sure_delete;
        loop {
            sure_delete =
                utils::user_input(&format!("Delete snippet #{} Y/N?", index), Some("N"), true)?
                    .to_ascii_uppercase();
            if sure_delete == "Y" || sure_delete == "N" {
                break;
            }
        }
        if sure_delete == "Y" {
            let index = index.parse::<usize>()?;
            let snippet = self.remove_snippet(index)?;
            self.delete_from_trees(&snippet, index)?;
            println!("Snippet #{} deleted", index);
            Ok(())
        } else {
            Err(LostTheWay::DoingNothing {
                message: "I'm a coward.".into(),
            }
            .into())
        }
    }

    /// Modify a stored snippet's information
    fn change(&mut self) -> Result<(), Error> {
        let index = utils::get_argument_value("change", &self.matches)?
            .ok_or(LostTheWay::OutOfCheeseError {
                message: "Argument change not used".into(),
            })?
            .parse::<usize>()?;
        let old_snippet = self.get_snippet(index)?;
        let new_snippet = Snippet::from_user(index, &self.languages, Some(&old_snippet))?;

        self.delete_from_trees(&old_snippet, index)?;
        let language_key = new_snippet.language.as_bytes();
        let index_key = index.to_string();
        let index_key = index_key.as_bytes();
        self.add_to_language(language_key, index_key)?;
        self.add_to_tags(&new_snippet.tags, index_key)?;
        self.add_to_snippet(index_key, &new_snippet.to_bytes()?)?;
        println!("Snippet #{} changed", index); // TODO: change to log?
        Ok(())
    }

    fn show(&self) -> Result<(), Error> {
        let index = utils::get_argument_value("show", &self.matches)?
            .ok_or(LostTheWay::OutOfCheeseError {
                message: "Argument show not used".into(),
            })?
            .parse::<usize>()?;
        let snippet = self.get_snippet(index)?;
        for line in snippet.pretty_print(&self.highlighter, &self.languages[&snippet.language])? {
            print!("{}", line)
        }
        Ok(())
    }

    // Copy a snippet to clipboard
    fn copy(&self) -> Result<(), Error> {
        let index = utils::get_argument_value("copy", &self.matches)?
            .ok_or(LostTheWay::OutOfCheeseError {
                message: "Argument copy not used".into(),
            })?
            .parse::<usize>()?;
        let snippet = self.get_snippet(index)?;
        utils::copy_to_clipboard(snippet.code);
        println!("Snippet #{} copied to clipboard", index);
        Ok(())
    }

    /// Syntax highlighting management
    fn list_themes(&self) -> Result<(), Error> {
        for theme in self.highlighter.get_themes() {
            println!("{}", theme);
        }
        Ok(())
    }

    fn import(&self, matches: &ArgMatches<'a>) -> Result<Vec<Snippet>, Error> {
        let json_file = PathFile::new(utils::get_argument_value("json", matches)?.ok_or(
            LostTheWay::OutOfCheeseError {
                message: "Argument json not used".into(),
            },
        )?)?;
        let snippets: Result<Vec<Snippet>, serde_json::Error> =
            Snippet::read_from_file(&json_file)?.collect();
        Ok(snippets?)
    }

    // Saves (optionally filtered) snippets to a JSON file
    fn export(&self, matches: &ArgMatches<'a>) -> Result<(), Error> {
        let json_file = PathFile::create(utils::get_argument_value("json", matches)?.ok_or(
            LostTheWay::OutOfCheeseError {
                message: "Argument json not used".into(),
            },
        )?)?;
        let filters = Filters::get_filters(matches)?;
        let mut writer = json_file.open_edit()?;
        self.filter_snippets(&filters)?
            .into_iter()
            .map(|snippet| snippet.to_json(&mut writer))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(())
    }

    /// Lists snippets (optionally filtered)
    fn list(&self, matches: &ArgMatches<'a>) -> Result<(), Error> {
        let filters = Filters::get_filters(matches)?;
        let snippets = self.filter_snippets(&filters)?;

        let mut colorized = Vec::new();
        for snippet in &snippets {
            colorized.extend_from_slice(
                &snippet.pretty_print(&self.highlighter, &self.languages[&snippet.language])?,
            );
        }
        for line in colorized {
            print!("{}", line);
        }
        Ok(())
    }

    /// Displays all snippet descriptions in a skim fuzzy search window
    /// A preview window on the right shows the indices of snippets matching the query
    fn search(&self, matches: &ArgMatches<'a>) -> Result<(), Error> {
        let filters = Filters::get_filters(matches)?;
        let snippets = self.filter_snippets(&filters)?;
        self.make_search(snippets)?;
        Ok(())
    }

    /// Generates shell completions
    fn complete(&self, matches: &ArgMatches<'a>) -> Result<(), Error> {
        let shell =
            utils::get_argument_value("shell", matches)?.ok_or(LostTheWay::OutOfCheeseError {
                message: "Argument shell not used".into(),
            })?;
        let yaml = load_yaml!("../the_way.yml");
        let mut app = App::from(yaml);
        app.gen_completions_to(
            utils::NAME,
            shell.parse::<Shell>().unwrap(),
            &mut io::stdout(),
        );
        Ok(())
    }

    /// Removes all `sled` trees
    fn clear(&self) -> Result<(), Error> {
        let mut sure_delete;
        loop {
            sure_delete =
                utils::user_input("Clear all data Y/N?", Some("N"), true)?.to_ascii_uppercase();
            if sure_delete == "Y" || sure_delete == "N" {
                break;
            }
        }
        if sure_delete == "Y" {
            for path in self.config.db_dir.list()? {
                let path = path?;
                if path.is_dir() {
                    PathDir::new(path)?.remove_all()?;
                } else {
                    PathFile::new(path)?.remove()?;
                }
            }
            self.reset_index()?;
            Ok(())
        } else {
            Err(LostTheWay::DoingNothing {
                message: "I'm a coward.".into(),
            }
            .into())
        }
    }
}
