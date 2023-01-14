//! Code related to dealing with Gists
use chrono::Utc;
use std::collections::HashMap;

use color_eyre::Help;

use crate::errors::LostTheWay;
use crate::gist::{CreateGistPayload, Gist, GistClient, GistContent, UpdateGistPayload};
use crate::language::Language;
use crate::the_way::{cli::SyncCommand, snippet::Snippet, TheWay};
use crate::utils;
use std::string::ToString;
use strum_macros::Display;

/// Gist description
const DESCRIPTION: &str = "The Way Code Snippets";
/// Heading for the index.md file
const INDEX_HEADING: &str = "# Is it not written...\n";

/// Parse line in Gist index.md file to get the snippet index, description and tags
pub(crate) fn parse_index_line(
    index_line: &str,
) -> color_eyre::Result<(usize, String, Vec<String>)> {
    // "[{snippet.description}]({result.html_url}#file-{snippet_{snippet.index}{snippet.extension}}) :tag1:tag2:\n"
    let re = regex::Regex::new(r"\* \[(.*)\]\(.*#file-snippet_([0-9]*)(.*)\)( :(.*)+:)?")?;
    let caps = re
        .captures(index_line)
        .ok_or(LostTheWay::GistFormattingError {
            message: format!("Index line isn't formatted correctly:\n{}", index_line),
        })?;
    let description = caps[1].to_owned();
    let index = caps[2].parse::<usize>()?;
    if let Some(tags) = caps.get(4) {
        let tags = tags
            .as_str()
            .trim()
            .split(':')
            .filter_map(|t| {
                let t = t.to_owned();
                if t.is_empty() {
                    None
                } else {
                    Some(t)
                }
            })
            .collect::<Vec<_>>();
        Ok((index, description, tags))
    } else {
        Ok((index, description, vec![]))
    }
}

/// Make a list item for the Gist index.md file
fn make_index_line(index_file_content: &mut String, html_url: &str, snippet: &Snippet) {
    index_file_content.push_str(&format!(
        "* [{}]({}#file-{}){}\n",
        snippet.description,
        html_url,
        format!("snippet_{}{}", snippet.index, snippet.extension).replace('.', "-"),
        if snippet.tags.is_empty() {
            String::new()
        } else {
            format!(" :{}:", snippet.tags.join(":"))
        }
    ));
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Display)]
enum SyncAction {
    #[strum(serialize = "downloaded from Gist")]
    Downloaded,
    #[strum(serialize = "uploaded to Gist")]
    Uploaded,
    #[strum(serialize = "added locally")]
    AddedLocal,
    #[strum(serialize = "deleted locally")]
    DeletedLocal,
    #[strum(serialize = "added to Gist")]
    AddedGist,
    #[strum(serialize = "deleted from Gist")]
    DeletedGist,
    #[strum(serialize = "up to date")]
    UpToDate,
}

impl Snippet {
    /// Read potentially multiple snippets from a Gist
    /// if `start_index` is None, indices are read from the Gist filenames (index.md is set to index 0)
    pub(crate) fn from_gist(
        start_index: Option<usize>,
        languages: &HashMap<String, Language>,
        gist: &Gist,
    ) -> color_eyre::Result<Vec<Self>> {
        let mut current_index = start_index;
        let mut snippets = Vec::new();
        for (file_name, gist_file) in &gist.files {
            let code = &gist_file.content;
            let description = format!("{} - {} - {}", gist.description, gist.id, file_name);
            let language = &gist_file.language.to_ascii_lowercase();
            let tags = "gist";
            let extension = Language::get_extension(language, languages);
            let index = if let Some(i) = current_index {
                i
            } else if file_name == "index.md" {
                0
            } else {
                file_name
                    .split('.')
                    .next()
                    .ok_or(LostTheWay::GistFormattingError {
                        message: format!("Filename {} missing extension", file_name),
                    })?
                    .split('_')
                    .nth(1)
                    .ok_or(LostTheWay::GistFormattingError {
                        message: format!("Filename {} missing index", file_name),
                    })?
                    .parse()?
            };
            let snippet = Self::new(
                index,
                description,
                language.to_string(),
                extension.to_owned(),
                tags,
                Utc::now(),
                Utc::now(),
                code.to_string(),
            );
            snippets.push(snippet);
            current_index = current_index.map(|i| i + 1);
        }
        Ok(snippets)
    }

    /// Read snippets from a gist created by `the-way sync`
    pub(crate) fn from_the_way_gist(
        languages: &HashMap<String, Language>,
        gist: &Gist,
    ) -> color_eyre::Result<Vec<Self>> {
        let snippets = Self::from_gist(None, languages, gist)?;
        let index_snippet =
            snippets
                .iter()
                .find(|s| s.index == 0)
                .ok_or(LostTheWay::GistFormattingError {
                    message: String::from("Index file not found"),
                })?;
        let mut index_mapping = HashMap::new();
        for line in index_snippet.code.trim().split('\n').skip(1) {
            let (index, description, tags) = parse_index_line(line)?;
            index_mapping.insert(index, (description, tags));
        }
        Ok(snippets
            .into_iter()
            .filter(|s| s.index != 0)
            .map(|mut snippet| {
                if let Some((description, tags)) = index_mapping.get(&snippet.index) {
                    snippet.description = description.clone();
                    snippet.tags = tags.clone();
                    Ok(snippet)
                } else {
                    Err(LostTheWay::GistFormattingError {
                        message: format!("Snippet index {} not found in index file", snippet.index),
                    })
                }
            })
            .collect::<Result<Vec<_>, _>>()?)
    }
}

impl TheWay {
    /// Fetch gist
    fn get_gist(gist_url: &str) -> color_eyre::Result<Gist> {
        let client = GistClient::new(None)?;
        let spinner = utils::get_spinner("Fetching gist...");
        let gist = client.get_gist_by_url(gist_url);
        if let Err(err) = gist {
            spinner.finish_with_message("Error fetching gist.");
            return Err(err);
        }
        gist
    }

    /// Import Snippets from a regular Gist
    pub(crate) fn import_gist(&mut self, gist_url: &str) -> color_eyre::Result<Vec<Snippet>> {
        let gist = Self::get_gist(gist_url)?;
        let start_index = self.get_current_snippet_index()? + 1;
        let snippets = Snippet::from_gist(Some(start_index), &self.languages, &gist)?;
        for snippet in &snippets {
            self.add_snippet(snippet)?;
            self.increment_snippet_index()?;
        }
        Ok(snippets)
    }

    /// Import snippets from a Gist created by `the_way sync`
    pub(crate) fn import_the_way_gist(
        &mut self,
        gist_url: &str,
    ) -> color_eyre::Result<Vec<Snippet>> {
        let gist = Self::get_gist(gist_url)?;
        let mut snippets = Snippet::from_the_way_gist(&self.languages, &gist)?;
        let mut current_index = self.get_current_snippet_index()? + 1;
        for snippet in &mut snippets {
            snippet.index = current_index;
            self.add_snippet(snippet)?;
            self.increment_snippet_index()?;
            current_index += 1;
        }
        Ok(snippets)
    }

    /// Creates a Gist with each code snippet as a separate file (named snippet_<index>.<ext>)
    /// and an index file (index.md) listing each snippet's description
    pub(crate) fn make_gist(&self, access_token: &str) -> color_eyre::Result<String> {
        // Make client
        let client = GistClient::new(Some(access_token))?;
        // Start creating
        let spinner = utils::get_spinner("Creating Gist...");

        // Make snippet files
        let mut files = HashMap::new();
        let snippets = self.list_snippets()?;
        for snippet in &snippets {
            let filename = format!("snippet_{}{}", snippet.index, snippet.extension);
            files.insert(
                filename,
                GistContent {
                    content: snippet.code.as_str(),
                },
            );
        }
        let payload = CreateGistPayload {
            description: DESCRIPTION,
            public: false,
            files,
        };
        // Upload snippet files to Gist
        let result = client.create_gist(&payload)?;

        // Make index file
        let mut index_file_content = String::from(INDEX_HEADING);
        for snippet in &snippets {
            make_index_line(&mut index_file_content, &result.html_url, snippet);
        }
        let mut update_files = HashMap::new();
        update_files.insert(
            String::from("index.md"),
            Some(GistContent {
                content: index_file_content.as_str(),
            }),
        );
        let update_payload = UpdateGistPayload {
            description: DESCRIPTION,
            files: update_files,
        };
        // Upload index file to Gist
        let result = client.update_gist(&result.id, &update_payload)?;
        spinner.finish_with_message(utils::highlight_string(
            &format!(
                "Created gist at {} with {} snippets",
                result.html_url,
                result.files.len()
            ),
            self.highlighter.main_style,
        ));

        // Return created Gist ID
        Ok(result.id)
    }

    /// Syncs local and Gist snippets according to user-selected source
    pub(crate) fn sync_gist(
        &mut self,
        github_access_token: Option<&str>,
        source: SyncCommand,
        force: bool,
    ) -> color_eyre::Result<()> {
        // Retrieve local snippets
        let mut snippets = self.list_snippets()?;
        if snippets.is_empty() && source == SyncCommand::Local {
            self.color_print("No snippets to sync.\n")?;
            return Ok(());
        }
        // Make client
        let client = GistClient::new(github_access_token)?;

        // Start sync
        let spinner = utils::get_spinner("Syncing...");

        // Count each type of sync action
        let mut action_counts = HashMap::new();
        // Keep track of added and updated Gist files
        let mut files = HashMap::new();
        // Keep track of local snippets to add and delete
        let mut add_snippets = Vec::new();
        let mut delete_snippets = Vec::new();
        // Index file
        let mut index_file_content = String::from(INDEX_HEADING);

        // Retrieve gist and gist snippets
        let gist = client.get_gist(self.config.gist_id.as_ref().unwrap());
        if gist.is_err() {
            spinner.finish_with_message(utils::highlight_string(
                "Gist not found.",
                self.highlighter.main_style,
            ));
            self.config.gist_id = Some(self.make_gist(github_access_token.as_ref().unwrap())?);
            return Ok(());
        }
        let gist = gist?;
        let gist_snippets = Snippet::from_the_way_gist(&self.languages, &gist)?
            .into_iter()
            .map(|snippet| (snippet.index, snippet))
            .collect::<HashMap<_, _>>();

        // Compare local snippets to gist snippets
        for snippet in &mut snippets {
            // if snippet is already present in gist:
            //     if source is Gist download changes to local snippet
            //     if source is Date and gist date is newer, download changes to local snippet
            //     if source is Local, upload changes to gist
            //     if source is Date and gist date is older, upload changes to gist
            // else
            //     if source is Local or Date, add snippet to gist
            //     if source is Gist, delete snippet from local
            let sync_action = if let Some(gist_snippet) = gist_snippets.get(&snippet.index) {
                // Snippets with same index exist in local and gist
                if snippet == gist_snippet {
                    // No change
                    SyncAction::UpToDate
                } else if source == SyncCommand::Local
                    || (source == SyncCommand::Date && snippet.updated > gist.updated_at)
                {
                    // Snippet updated locally or source is local => update Gist
                    files.insert(
                        format!("snippet_{}{}", snippet.index, snippet.extension),
                        Some(GistContent {
                            content: snippet.code.as_str(),
                        }),
                    );
                    SyncAction::Uploaded
                } else if source == SyncCommand::Gist
                    || (source == SyncCommand::Date && snippet.updated < gist.updated_at)
                {
                    // Snippet updated in Gist or source is Gist => update local snippet
                    let index_key = gist_snippet.index.to_string();
                    let index_key = index_key.as_bytes();
                    self.add_to_snippet(index_key, &gist_snippet.to_bytes()?)?;
                    SyncAction::Downloaded
                } else {
                    // Update dates match
                    SyncAction::UpToDate
                }
            } else {
                // Snippet with this index not in Gist
                match source {
                    SyncCommand::Local | SyncCommand::Date => {
                        // Snippet in local and not in gist => add to gist
                        files.insert(
                            format!("snippet_{}{}", snippet.index, snippet.extension),
                            Some(GistContent {
                                content: snippet.code.as_str(),
                            }),
                        );
                        SyncAction::AddedGist
                    }
                    SyncCommand::Gist => {
                        // Snippet deleted in gist => delete from local
                        delete_snippets.push(snippet.index);
                        SyncAction::DeletedLocal
                    }
                }
            };
            if sync_action != SyncAction::DeletedLocal {
                // add snippet to index file
                make_index_line(&mut index_file_content, &gist.html_url, snippet);
            }
            *action_counts.entry(sync_action).or_insert(0) += 1;
        }
        // Compare gist snippets to local snippets
        for file in gist.files.keys() {
            if file != "index.md" {
                let snippet_index = get_gist_snippet_index(file)?;
                // if snippet is not present locally:
                //     if source is Local or Date, delete snippet from gist
                //     if source is Gist add snippet to local snippets
                if self.get_snippet(snippet_index).is_err() {
                    let sync_action = match source {
                        SyncCommand::Local | SyncCommand::Date => {
                            // delete from Gist
                            files.insert(file.clone(), None);
                            SyncAction::DeletedGist
                        }
                        SyncCommand::Gist => {
                            // add to local
                            let gist_snippet = gist_snippets.get(&snippet_index).ok_or(
                                LostTheWay::GistFormattingError {
                                    message: format!("Invalid snippet index {}", snippet_index),
                                },
                            )?;
                            add_snippets.push(gist_snippet);
                            // add snippet to index file
                            make_index_line(&mut index_file_content, &gist.html_url, gist_snippet);
                            SyncAction::AddedLocal
                        }
                    };
                    *action_counts.entry(sync_action).or_insert(0) += 1;
                }
            }
        }
        // Update Gist
        if let Some(index_file) = gist.files.get("index.md") {
            if index_file.content != index_file_content {
                files.insert(
                    "index.md".to_owned(),
                    Some(GistContent {
                        content: index_file_content.as_str(),
                    }),
                );
            }
        }
        if !files.is_empty() {
            client.update_gist(
                &gist.id,
                &UpdateGistPayload {
                    description: DESCRIPTION,
                    files,
                },
            )?;
        }
        spinner.finish_with_message("Done!");
        let mut max_index = 0;
        for snippet in add_snippets {
            let index = self.add_snippet(snippet)?;
            if index > max_index {
                max_index = index;
            }
        }
        self.modify_snippet_index(max_index + 1)?;
        let delete = if delete_snippets.is_empty() || force {
            true
        } else {
            utils::confirm(
                &format!("Delete {} snippets locally?", delete_snippets.len()),
                false,
            )?
        };
        if delete {
            for index in delete_snippets {
                self.delete_snippet(index)?;
            }
        }

        // Print results
        for (action, count) in action_counts {
            if action == SyncAction::DeletedLocal && !delete {
                continue;
            }
            self.color_print(&format!("{} snippet(s) {}\n", count, action))?;
        }
        self.color_print(&format!("\nGist: {}\n", gist.html_url))?;
        Ok(())
    }
}

fn get_gist_snippet_index(file: &str) -> color_eyre::Result<usize> {
    let suggestion =
        "Make sure snippet files in the Gist are of the form \'snippet_<index>.<ext>\'";
    let snippet_index = file
        .split('.')
        .next()
        .ok_or(LostTheWay::GistFormattingError {
            message: format!("Invalid filename {}: No .", file),
        })
        .suggestion(suggestion)?
        .split('_')
        .last()
        .ok_or(LostTheWay::GistFormattingError {
            message: format!("Invalid filename {}: No _", file),
        })
        .suggestion(suggestion)?
        .parse::<usize>()
        .map_err(|e| LostTheWay::GistFormattingError {
            message: format!("Invalid filename {}: {}", file, e),
        })
        .suggestion(suggestion)?;
    Ok(snippet_index)
}
