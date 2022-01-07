//! Code related to dealing with Gists
use std::collections::HashMap;

use color_eyre::Help;

use crate::errors::LostTheWay;
use crate::gist::{CreateGistPayload, GistClient, GistContent, UpdateGistPayload};
use crate::the_way::{snippet::Snippet, TheWay};
use crate::utils;

/// Gist description
const DESCRIPTION: &str = "The Way Code Snippets";
/// Heading for the index.md file
const INDEX: &str = "# Is it not written...\n";

fn parse_index_line(index_line: &str) -> color_eyre::Result<(String, usize, String, Vec<String>)> {
    // "[{snippet.description}]({result.html_url}#file-{snippet_{snippet.index}{snippet.extension}}) :tag1:tag2:\n"
    let re = regex::Regex::new(r"\* \[(.*)\]\(.*#file-snippet_([0-9]*)(.*)\)( :(.*)+:)?")?;
    let caps = re
        .captures(index_line)
        .ok_or(LostTheWay::GistFormattingError {
            message: format!("Index line isn't formatted correctly:\n{}", index_line),
        })?;
    let description = caps[1].to_owned();
    let index = caps[2].parse::<usize>()?;
    let extension = caps[3].replace('-', ".");
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
        Ok((description, index, extension, tags))
    } else {
        Ok((description, index, extension, vec![]))
    }
}

fn make_index_line(index: &mut String, html_url: &str, snippet: &Snippet) {
    index.push_str(&format!(
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

impl TheWay {
    /// Import Snippets from a regular Gist
    pub(crate) fn import_gist(&mut self, gist_url: &str) -> color_eyre::Result<Vec<Snippet>> {
        // it's assumed that access token is not required when reading Gists
        let client = GistClient::new(None)?;

        let spinner = utils::get_spinner("Fetching gist...");
        let gist = client.get_gist_by_url(gist_url);
        if let Err(err) = gist {
            spinner.finish_with_message("Error fetching gist.");
            return Err(err);
        }
        let gist = gist.unwrap();
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
        let client = GistClient::new(None)?;
        let spinner = utils::get_spinner("Fetching gist...");
        let gist = client.get_gist_by_url(gist_url);
        if let Err(err) = gist {
            spinner.finish_with_message("Error fetching gist.");
            return Err(err);
        }
        let gist = gist.unwrap();
        let snippets = Snippet::from_gist(None, &self.languages, &gist)?;
        let index_snippet =
            snippets
                .iter()
                .find(|s| s.index == 0)
                .ok_or(LostTheWay::GistFormattingError {
                    message: String::from("Index file not found"),
                })?;
        let mut index_mapping = HashMap::new();
        for line in index_snippet.code.trim().split('\n').skip(1) {
            let (description, index, extension, tags) = parse_index_line(line)?;
            index_mapping.insert(index, (description, extension, tags));
        }
        let mut snippets = snippets
            .into_iter()
            .filter(|s| s.index != 0)
            .map(|mut s| {
                if let Some((description, extension, tags)) = index_mapping.get(&s.index) {
                    s.description = description.to_owned();
                    if &s.extension != extension {
                        return Err(LostTheWay::GistFormattingError {
                            message: format!(
                                "Extension mismatch, expected {} but got {}",
                                extension, s.extension
                            ),
                        });
                    }
                    s.tags = tags.to_owned();
                    Ok(s)
                } else {
                    Err(LostTheWay::GistFormattingError {
                        message: format!("Snippet index {} not found in index file", s.index),
                    })
                }
            })
            .collect::<Result<Vec<_>, _>>()?;

        let mut start_index = self.get_current_snippet_index()? + 1;
        for snippet in &mut snippets {
            snippet.index = start_index;
            self.add_snippet(snippet)?;
            self.increment_snippet_index()?;
            start_index += 1;
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
        let mut index = String::from(INDEX);
        for snippet in &snippets {
            make_index_line(&mut index, &result.html_url, snippet);
        }
        let mut update_files = HashMap::new();
        update_files.insert(
            String::from("index.md"),
            Some(GistContent {
                content: index.as_str(),
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
        if self.list_snippets()?.is_empty() {
            println!("{}", self.highlight_string("No snippets to sync."));
            return Ok(());
        }
        // Make client
        let client = GistClient::new(self.config.github_access_token.as_deref())?;

        // Start sync
        let spinner = utils::get_spinner("Syncing...");

        let mut updated = 0;
        let mut added = 0;
        let mut downloaded = 0;
        let mut deleted = 0;
        let mut index = String::from(INDEX);

        // Retrieve gist

        let gist = client.get_gist(self.config.gist_id.as_ref().unwrap());
        if gist.is_err() {
            spinner.finish_with_message(self.highlight_string("Gist not found."));
            self.config.gist_id =
                Some(self.make_gist(self.config.github_access_token.as_ref().unwrap())?);
            return Ok(());
        }
        let gist = gist.unwrap();
        // Retrieve local snippets
        let mut snippets = self.list_snippets()?;

        let mut files = HashMap::new();
        for snippet in &mut snippets {
            // Check if snippet exists in Gist
            match gist
                .files
                .get(&format!("snippet_{}{}", snippet.index, snippet.extension))
            {
                Some(gist_file) => {
                    match snippet.updated.cmp(&gist.updated_at) {
                        std::cmp::Ordering::Less => {
                            // Snippet updated in Gist => download to local
                            if gist_file.content != snippet.code {
                                let index_key = snippet.index.to_string();
                                let index_key = index_key.as_bytes();
                                snippet.code = gist_file.content.clone();
                                self.add_to_snippet(index_key, &snippet.to_bytes()?)?;
                                downloaded += 1;
                            }
                        }
                        std::cmp::Ordering::Greater => {
                            // Snippet updated locally => update Gist
                            if gist_file.content != snippet.code {
                                files.insert(
                                    format!("snippet_{}{}", snippet.index, snippet.extension),
                                    Some(GistContent {
                                        content: snippet.code.as_str(),
                                    }),
                                );
                                updated += 1;
                            }
                        }
                        std::cmp::Ordering::Equal => {}
                    }
                }
                // Not in Gist => add
                None => {
                    files.insert(
                        format!("snippet_{}{}", snippet.index, snippet.extension),
                        Some(GistContent {
                            content: snippet.code.as_str(),
                        }),
                    );
                    added += 1;
                }
            }
            // Add to index
            make_index_line(&mut index, &gist.html_url, snippet);
        }
        for file in gist.files.keys() {
            if file.contains("snippet_") {
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
                    files.insert(file.to_owned(), None);
                    deleted += 1;
                }
            }
        }
        // Update Gist
        if let Some(index_file) = gist.files.get("index.md") {
            if index_file.content != index {
                files.insert(
                    "index.md".to_owned(),
                    Some(GistContent {
                        content: index.as_str(),
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
        if added > 0 {
            println!(
                "{}",
                self.highlight_string(&format!("Added {} snippet(s)", added))
            );
        }
        if updated > 0 {
            println!(
                "{}",
                self.highlight_string(&format!("Updated {} snippet(s)", updated))
            );
        }
        if deleted > 0 {
            println!(
                "{}",
                self.highlight_string(&format!("Deleted {} snippet(s)", deleted))
            );
        }
        if downloaded > 0 {
            println!(
                "{}",
                self.highlight_string(&format!("Downloaded {} snippet(s)", downloaded))
            );
        }
        if added + updated + downloaded + deleted == 0 {
            println!("{}", self.highlight_string("Everything up to date"));
        }
        println!(
            "{}",
            self.highlight_string(&format!("\nGist: {}", gist.html_url))
        );
        Ok(())
    }
}
