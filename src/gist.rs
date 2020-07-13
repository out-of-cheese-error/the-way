use std::collections::HashMap;

use chrono::{DateTime, Utc};
use color_eyre::Help;
use reqwest::header;

use crate::errors::LostTheWay;

const GITHUB_API_URL: &str = "https://api.github.com";
const GITHUB_BASE_PATH: &str = "";
const ACCEPT: &str = "application/vnd.github.v3+json";

#[derive(Serialize, Debug)]
pub struct GistContent<'a> {
    pub content: &'a str,
}

#[derive(Serialize, Debug)]
pub struct CreateGistPayload<'a> {
    pub description: &'a str,
    pub public: bool,
    pub files: HashMap<String, GistContent<'a>>,
}

#[derive(Serialize, Debug)]
pub struct UpdateGistPayload<'a> {
    pub description: &'a str,
    pub files: HashMap<String, Option<GistContent<'a>>>,
}

#[derive(Deserialize, Debug)]
pub struct Gist {
    pub html_url: String,
    pub id: String,
    pub updated_at: DateTime<Utc>,
    pub description: String,
    pub files: HashMap<String, GistFile>,
}

#[derive(Deserialize, Debug)]
pub struct GistFile {
    filename: String,
    pub content: String,
}

pub struct GistClient {
    client: reqwest::blocking::Client,
}

impl GistClient {
    pub fn new(access_token: &str, user_agent: &str) -> color_eyre::Result<Self> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_str(&format!("token {}", access_token))?,
        );
        headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_str(user_agent)?,
        );
        headers.insert(header::ACCEPT, header::HeaderValue::from_str(ACCEPT)?);
        let client = reqwest::blocking::Client::builder()
            .default_headers(headers)
            .build()?;
        Ok(GistClient { client })
    }

    /// Create a new Gist with the given payload
    pub fn create_gist(&self, payload: &CreateGistPayload<'_>) -> color_eyre::Result<Gist> {
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
    pub fn update_gist(
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
    pub fn get_gist(&self, gist_id: &str) -> color_eyre::Result<Gist> {
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

    pub fn delete_gist(&self, gist_id: &str) -> color_eyre::Result<()> {
        let url = format!("{}{}/gists", GITHUB_API_URL, GITHUB_BASE_PATH);
        let status = self
            .client
            .delete(&format!("{}/{}", url, gist_id))
            .send()?
            .status();
        assert!(status.is_success());
        Ok(())
    }
}
