//! Code related to dealing with Gists
use chrono::Utc;
use std::collections::HashMap;

use color_eyre::Help;

use crate::errors::LostTheWay;
use crate::gist::{CreateGistPayload, Gist, GistClient, GistContent, UpdateGistPayload};
use crate::language::Language;
use crate::the_way::{snippet::Snippet, TheWay};
use crate::utils;
use strum::EnumIter;
use strum::IntoEnumIterator;

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

#[derive(Debug, EnumIter, PartialEq, Eq, Hash, Clone, Copy)]
enum SyncAction {
    Downloaded,
    Uploaded,
    Added,
    UpToDate,
    Deleted,
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
        spinner.finish_with_message(self.highlight_string(&format!(
            "Created gist at {} with {} snippets",
            result.html_url,
            result.files.len()
        )));

        // Return created Gist ID
        Ok(result.id)
    }

    /// Syncs local and Gist snippets
    pub(crate) fn sync_gist(&mut self) -> color_eyre::Result<()> {
        // Retrieve local snippets
        let mut snippets = self.list_snippets()?;
        if snippets.is_empty() {
            println!("{}", self.highlight_string("No snippets to sync."));
            return Ok(());
        }
        // Make client
        let client = GistClient::new(self.config.github_access_token.as_deref())?;

        // Start sync
        let spinner = utils::get_spinner("Syncing...");

        // Count each type of sync action
        let mut counts = SyncAction::iter()
            .map(|action| (action, 0))
            .collect::<HashMap<_, _>>();
        // Keep track of added and updated files
        let mut files = HashMap::new();
        // Index file
        let mut index_file_content = String::from(INDEX_HEADING);

        // Retrieve gist and gist snippets
        let gist = client.get_gist(self.config.gist_id.as_ref().unwrap());
        if gist.is_err() {
            spinner.finish_with_message(self.highlight_string("Gist not found."));
            self.config.gist_id =
                Some(self.make_gist(self.config.github_access_token.as_ref().unwrap())?);
            return Ok(());
        }
        let gist = gist?;
        let gist_snippets = Snippet::from_the_way_gist(&self.languages, &gist)?
            .into_iter()
            .map(|snippet| (snippet.index, snippet))
            .collect::<HashMap<_, _>>();

        // Compare local snippets to gist
        for snippet in &mut snippets {
            let sync_action = self.sync_snippet(
                snippet,
                &gist_snippets,
                &gist,
                &mut files,
                &mut index_file_content,
            )?;
            *counts.entry(sync_action).or_insert(0) += 1;
        }
        // Compare gist to local snippets
        for file in gist.files.keys() {
            if file != "index.md" {
                let suggestion =
                    "Make sure snippet files in the Gist are of the form \'snippet_<index>.<ext>\'";
                let snippet_id = file
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
                // Snippet deleted locally => delete from Gist
                if self.get_snippet(snippet_id).is_err() {
                    files.insert(file.clone(), None);
                    *counts.entry(SyncAction::Deleted).or_insert(0) += 1;
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

        // Print results
        for (action, count) in counts {
            if count > 0 {
                if action == SyncAction::UpToDate {
                    println!(
                        "{}",
                        self.highlight_string(&format!("{} snippet(s) are up to date", count))
                    );
                } else {
                    println!(
                        "{}",
                        self.highlight_string(&format!("{:?} {} snippet(s)", action, count))
                    );
                }
            }
        }
        println!(
            "{}",
            self.highlight_string(&format!("\nGist: {}", gist.html_url))
        );
        Ok(())
    }

    /// Synchronize a snippet with the gist:
    /// if gist date is newer and snippet is different, download to local snippet
    /// if gist date is older and snippet is different, upload to gist
    /// if snippet not in gist, add
    fn sync_snippet<'a>(
        &self,
        snippet: &'a mut Snippet,
        gist_snippets: &HashMap<usize, Snippet>,
        gist: &Gist,
        files: &mut HashMap<String, Option<GistContent<'a>>>,
        index_file_content: &mut String,
    ) -> color_eyre::Result<SyncAction> {
        let action = if let Some(gist_snippet) = gist_snippets.get(&snippet.index) {
            match snippet.updated.cmp(&gist.updated_at) {
                std::cmp::Ordering::Less => {
                    // Snippet updated in Gist => download to local
                    if snippet == gist_snippet {
                        SyncAction::UpToDate
                    } else {
                        let index_key = gist_snippet.index.to_string();
                        let index_key = index_key.as_bytes();
                        self.add_to_snippet(index_key, &gist_snippet.to_bytes()?)?;
                        SyncAction::Downloaded
                    }
                }
                std::cmp::Ordering::Greater => {
                    // Snippet updated locally => update Gist
                    if snippet == gist_snippet {
                        SyncAction::UpToDate
                    } else {
                        files.insert(
                            format!("snippet_{}{}", snippet.index, snippet.extension),
                            Some(GistContent {
                                content: snippet.code.as_str(),
                            }),
                        );
                        SyncAction::Uploaded
                    }
                }
                std::cmp::Ordering::Equal => {
                    // Snippet up to date
                    SyncAction::UpToDate
                }
            }
        } else {
            files.insert(
                format!("snippet_{}{}", snippet.index, snippet.extension),
                Some(GistContent {
                    content: snippet.code.as_str(),
                }),
            );
            SyncAction::Added
        };
        // Add to index
        make_index_line(index_file_content, &gist.html_url, snippet);
        Ok(action)
    }
}
