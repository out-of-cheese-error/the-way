use std::collections::HashMap;
use std::env;

use chrono::{DateTime, Utc};
use color_eyre::Help;
use reqwest::header;

use crate::errors::LostTheWay;
use crate::the_way::TheWay;

const GITHUB_API_URL: &str = "https://api.github.com";
const GITHUB_BASE_PATH: &str = "";
const USER_AGENT: &str = "the-way";
const DESCRIPTION: &str = "The Way Code Snippets";
const ACCEPT: &str = "application/vnd.github.v3+json";
const INDEX: &str = "# Is it not written...\n";

#[derive(Serialize, Debug)]
struct GistContent<'a> {
    content: &'a str,
}

#[derive(Serialize, Debug)]
struct CreateGistPayload<'a> {
    description: &'a str,
    public: bool,
    files: HashMap<String, GistContent<'a>>,
}

#[derive(Serialize, Debug)]
struct UpdateGistPayload<'a> {
    description: &'a str,
    files: HashMap<String, Option<GistContent<'a>>>,
}

#[derive(Deserialize, Debug)]
struct Gist {
    html_url: String,
    id: String,
    updated_at: DateTime<Utc>,
    files: HashMap<String, GistFile>,
}

#[derive(Deserialize, Debug)]
struct GistFile {
    filename: String,
    content: String,
}

struct GistClient {
    client: reqwest::blocking::Client,
}

impl GistClient {
    fn new(access_token: &str) -> color_eyre::Result<Self> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_str(&format!("token {}", access_token))?,
        );
        headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_str(USER_AGENT)?,
        );
        headers.insert(header::ACCEPT, header::HeaderValue::from_str(ACCEPT)?);
        let client = reqwest::blocking::Client::builder()
            .default_headers(headers)
            .build()?;
        Ok(GistClient { client })
    }

    /// Create a new Gist with the given payload
    fn create_gist(&self, payload: &CreateGistPayload<'_>) -> color_eyre::Result<Gist> {
        let url = format!("{}{}/gists", GITHUB_API_URL, GITHUB_BASE_PATH);
        let text = self.client.post(&url).json(payload).send()?.text()?;

        let result = serde_json::from_str::<Gist>(&text)
            .map_err(|_| LostTheWay::SyncError { message: text })
            .suggestion(
                "Make sure your GitHub access token is valid.\n\
        Get one from https://github.com/settings/tokens/new (add the \"gist\" scope).\n\
        Set it to the environment variable $THE_WAY_GITHUB_TOKEN",
            )?;
        Ok(result)
    }

    /// Update an existing Gist
    fn update_gist(
        &self,
        gist_id: &str,
        payload: &UpdateGistPayload<'_>,
    ) -> color_eyre::Result<Gist> {
        let url = format!("{}{}/gists", GITHUB_API_URL, GITHUB_BASE_PATH);
        let text = self
            .client
            .patch(&format!("{}/{}", url, gist_id))
            .json(&payload)
            .send()?
            .text()?;
        let result = serde_json::from_str::<Gist>(&text)
            .map_err(|_| LostTheWay::SyncError { message: text })
            .suggestion(
                "Make sure your GitHub access token is valid.\n\
        Get one from https://github.com/settings/tokens/new (add the \"gist\" scope).\n\
        Set it to the environment variable $THE_WAY_GITHUB_TOKEN",
            )?;
        Ok(result)
    }

    /// Retrieve a Gist by ID
    fn get_gist(&self, gist_id: &str) -> color_eyre::Result<Gist> {
        let url = format!("{}{}/gists", GITHUB_API_URL, GITHUB_BASE_PATH);
        let text = self
            .client
            .get(&format!("{}/{}", url, gist_id))
            .send()?
            .text()?;
        let result = serde_json::from_str::<Gist>(&text)
            .map_err(|_| LostTheWay::SyncError { message: text })
            .suggestion(
                "Make sure your GitHub access token is valid.\n\
        Get one from https://github.com/settings/tokens/new (add the \"gist\" scope).\n\
        Set it to the environment variable $THE_WAY_GITHUB_TOKEN",
            )?;
        Ok(result)
    }
}

fn get_spinner(message: &str) -> indicatif::ProgressBar {
    let spinner = indicatif::ProgressBar::new_spinner();
    spinner.enable_steady_tick(200);
    spinner.set_style(
        indicatif::ProgressStyle::default_spinner()
            .tick_chars("/|\\- ")
            .template("{spinner:.dim.bold.blue} {wide_msg}"),
    );
    spinner.set_message(message);
    spinner
}

impl TheWay {
    /// Creates a Gist with each code snippet as a separate file (named snippet_<index>.<ext>)
    /// and an index file (index.md) listing each snippet's description
    fn make_gist(&self, access_token: &str) -> color_eyre::Result<String> {
        // Make client
        let client = GistClient::new(access_token)?;
        // Start creating
        let spinner = get_spinner("Creating Gist...");

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
            index.push_str(&format!(
                "* [{}]({}#file-{})\n",
                snippet.description,
                result.html_url,
                format!("snippet_{}{}", snippet.index, snippet.extension).replace(".", "-")
            ));
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
        spinner.finish_with_message(&format!(
            "Created gist at {} with {} snippets",
            result.html_url,
            result.files.len()
        ));

        // Return created Gist ID
        Ok(result.id)
    }

    fn sync_gist(&mut self) -> color_eyre::Result<()> {
        // Make client
        let client = GistClient::new(self.config.github_access_token.as_ref().unwrap())?;

        // Start sync
        let spinner = get_spinner("Syncing...");

        let mut updated = 0;
        let mut added = 0;
        let mut downloaded = 0;
        let mut deleted = 0;
        let mut index = String::from(INDEX);

        // Retrieve gist

        let gist = client.get_gist(self.config.gist_id.as_ref().unwrap());
        if gist.is_err() {
            spinner.finish_with_message("Gist not found.");
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
                            files.insert(
                                format!("snippet_{}{}", snippet.index, snippet.extension),
                                Some(GistContent {
                                    content: snippet.code.as_str(),
                                }),
                            );
                            updated += 1;
                        }
                        _ => {}
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
            index.push_str(&format!(
                "* [{}]({}#file-{})\n",
                snippet.description,
                gist.html_url,
                format!("snippet_{}{}", snippet.index, snippet.extension).replace(".", "-")
            ));
        }
        for file in gist.files.keys() {
            if file.contains("snippet_") {
                let suggestion =
                    "Make sure snippet files in the Gist are of the form \'snippet_<index>.<ext>\'";
                let snippet_id = file
                    .split('.')
                    .next()
                    .ok_or(LostTheWay::SyncError {
                        message: "Invalid filename".into(),
                    })
                    .suggestion(suggestion)?
                    .split('_')
                    .last()
                    .ok_or(LostTheWay::SyncError {
                        message: "Invalid filename".into(),
                    })
                    .suggestion(suggestion)?
                    .parse::<usize>()
                    .map_err(|e| LostTheWay::SyncError {
                        message: format!("Invalid filename: {}", e),
                    })
                    .suggestion(suggestion)?;
                // Snippet deleted locally => delete from Gist
                if self.get_snippet(snippet_id).is_err() {
                    files.insert(file.to_owned(), None);
                    deleted += 1;
                }
            }
        }
        files.insert(
            "index.md".to_owned(),
            Some(GistContent {
                content: index.as_str(),
            }),
        );
        // Update Gist
        client.update_gist(
            &gist.id,
            &UpdateGistPayload {
                description: DESCRIPTION,
                files,
            },
        )?;
        spinner.finish_with_message("Done!");
        if added > 0 {
            println!("Added {} snippet(s)\n", added);
        }
        if updated > 0 {
            println!("Updated {} snippet(s)\n", updated);
        }
        if deleted > 0 {
            println!("Deleted {} snippet(s)\n", deleted);
        }
        if downloaded > 0 {
            println!("Downloaded {} snippet(s)\n", downloaded);
        }
        if added + updated + downloaded + deleted == 0 {
            println!("Everything up to date\n");
        }
        println!("Gist: {}", gist.html_url);
        Ok(())
    }

    pub(crate) fn sync(&mut self) -> color_eyre::Result<()> {
        // Check if environment variable has changed
        self.config.github_access_token = env::var("THE_WAY_GITHUB_TOKEN")
            .ok()
            .or_else(|| self.config.github_access_token.clone());
        // Get token from user if not set
        if self.config.github_access_token.is_none() {
            println!("Get a GitHub access token from https://github.com/settings/tokens/new (add the \"gist\" scope)\n");
            self.config.github_access_token = Some(
                dialoguer::Password::with_theme(&dialoguer::theme::ColorfulTheme::default())
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
}
