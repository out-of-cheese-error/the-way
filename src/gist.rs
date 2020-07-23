//! Simple Gist API wrapper
use std::collections::HashMap;

use chrono::{DateTime, Utc};
use color_eyre::Help;

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
    client: ureq::Agent,
}

impl GistClient {
    /// Create a new Gist client
    pub fn new(access_token: &str, user_agent: &str) -> color_eyre::Result<Self> {
        let mut client = ureq::agent();
        client
            .set("Authorization", &format!("token {}", access_token))
            .set("user-agent", user_agent)
            .set("content-type", ACCEPT);
        Ok(Self { client })
    }

    fn get_response(response: ureq::Response) -> color_eyre::Result<Gist> {
        if response.ok() {
            Ok(response
                .into_json_deserialize::<Gist>()
                .map_err(|e| LostTheWay::SyncError {
                    message: format!("{}", e),
                })?)
        } else {
            Err(LostTheWay::SyncError {
                message: format!("{} {}", response.status(), response.into_string()?),
            })
            .suggestion(
                "Make sure your GitHub access token is valid.\n\
        Get one from https://github.com/settings/tokens/new (add the \"gist\" scope).\n\
        Set it to the environment variable $THE_WAY_GITHUB_TOKEN",
            )
        }
    }

    /// Create a new Gist with the given payload
    pub fn create_gist(&self, payload: &CreateGistPayload<'_>) -> color_eyre::Result<Gist> {
        let url = format!("{}{}/gists", GITHUB_API_URL, GITHUB_BASE_PATH);
        let response = self
            .client
            .post(&url)
            .send_json(ureq::serde_to_value(payload)?);
        Self::get_response(response)
    }

    /// Update an existing Gist
    pub fn update_gist(
        &self,
        gist_id: &str,
        payload: &UpdateGistPayload<'_>,
    ) -> color_eyre::Result<Gist> {
        let url = format!("{}{}/gists", GITHUB_API_URL, GITHUB_BASE_PATH);
        let response = self
            .client
            .patch(&format!("{}/{}", url, gist_id))
            .send_json(ureq::serde_to_value(payload)?);
        Self::get_response(response)
    }

    /// Retrieve a Gist by ID
    pub fn get_gist(&self, gist_id: &str) -> color_eyre::Result<Gist> {
        let url = format!("{}{}/gists", GITHUB_API_URL, GITHUB_BASE_PATH);
        let response = self.client.get(&format!("{}/{}", url, gist_id)).call();
        Self::get_response(response)
    }

    /// Delete Gist by ID
    pub fn delete_gist(&self, gist_id: &str) -> color_eyre::Result<()> {
        let url = format!("{}{}/gists", GITHUB_API_URL, GITHUB_BASE_PATH);
        let status = self.client.delete(&format!("{}/{}", url, gist_id)).call();
        assert!(status.ok());
        Ok(())
    }
}
