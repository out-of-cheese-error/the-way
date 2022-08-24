//! Simple Gist API wrapper
use std::collections::HashMap;

use chrono::{DateTime, Utc};
use color_eyre::Help;
use regex::Regex;

use crate::errors::LostTheWay;

const GITHUB_API_URL: &str = "https://api.github.com";
const GITHUB_BASE_PATH: &str = "";
const ACCEPT: &str = "application/vnd.github.v3+json";
const USER_AGENT: &str = "the-way";

/// Expects URL like `https://gist.github.com/user/<gist_id>`
/// or `https://gist.github.com/<gist_id>`
fn gist_id_from_url(gist_url: &str) -> color_eyre::Result<Option<&str>> {
    let re = Regex::new(r"https://gist\.github\.com/(.+/)?(?P<gist_id>[0-9a-f]+)$")?;
    Ok(re
        .captures(gist_url)
        .and_then(|cap| cap.name("gist_id").map(|gist_id| gist_id.as_str())))
}

/// Gist code content
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
    pub content: String,
    pub language: String,
}

pub struct GistClient<'a> {
    client: ureq::Agent,
    access_token: Option<&'a str>,
}

impl<'a> GistClient<'a> {
    /// Create a new Gist client
    pub fn new(access_token: Option<&'a str>) -> color_eyre::Result<Self> {
        Ok(Self {
            client: ureq::agent(),
            access_token,
        })
    }

    fn add_headers(&self, request: ureq::Request) -> ureq::Request {
        let mut request = request
            .set("user-agent", USER_AGENT)
            .set("content-type", ACCEPT);
        if let Some(access_token) = &self.access_token {
            request = request.set("Authorization", &format!("token {}", access_token));
        }
        request
    }

    fn get_response(response: Result<ureq::Response, ureq::Error>) -> color_eyre::Result<Gist> {
        match response {
            Ok(response) => {
                Ok(response
                    .into_json::<Gist>()
                    .map_err(|e| LostTheWay::SyncError {
                        message: format!("{}", e),
                    })?)
            }
            Err(ureq::Error::Status(code, response)) => Err(LostTheWay::SyncError {
                message: format!("{} {}", code, response.into_string()?),
            })
            .suggestion(
                "Make sure your GitHub access token is valid.\n\
        Get one from https://github.com/settings/tokens/new (add the \"gist\" scope).\n\
        Set it to the environment variable $THE_WAY_GITHUB_TOKEN",
            ),
            Err(_) => Err(LostTheWay::SyncError {
                message: "io/transport error".into(),
            })
            .suggestion(
                "Make sure your GitHub access token is valid.\n\
        Get one from https://github.com/settings/tokens/new (add the \"gist\" scope).\n\
        Set it to the environment variable $THE_WAY_GITHUB_TOKEN",
            ),
        }
    }

    /// Create a new Gist with the given payload
    pub fn create_gist(&self, payload: &CreateGistPayload<'_>) -> color_eyre::Result<Gist> {
        let url = format!("{}{}/gists", GITHUB_API_URL, GITHUB_BASE_PATH);
        let response = self
            .add_headers(self.client.post(&url))
            .send_json(ureq::serde_json::to_value(payload)?);
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
            .add_headers(
                self.client
                    .request("PATCH", &format!("{}/{}", url, gist_id)),
            )
            .send_json(ureq::serde_json::to_value(payload)?);
        Self::get_response(response)
    }

    /// Retrieve a Gist by ID
    pub fn get_gist(&self, gist_id: &str) -> color_eyre::Result<Gist> {
        let url = format!("{}{}/gists", GITHUB_API_URL, GITHUB_BASE_PATH);
        let response = self.add_headers(self.client.get(&format!("{}/{}", url, gist_id)));
        Self::get_response(response.call())
    }

    /// Retrieve a Gist by URL
    pub fn get_gist_by_url(&self, gist_url: &str) -> color_eyre::Result<Gist> {
        let gist_id = gist_id_from_url(gist_url)?;
        match gist_id {
            Some(gist_id) => self.get_gist(gist_id),
            None => Err(LostTheWay::GistUrlError {
                message: format!("Problem extracting gist ID from {}", gist_url),
            })
            .suggestion("The URL should look like https://gist.github.com/<user>/<gist_id>."),
        }
    }

    /// Delete Gist by ID
    pub fn delete_gist(&self, gist_id: &str) -> color_eyre::Result<()> {
        let url = format!("{}{}/gists", GITHUB_API_URL, GITHUB_BASE_PATH);
        let status = self.add_headers(self.client.delete(&format!("{}/{}", url, gist_id)));
        if status.call().is_err() {
            Err(LostTheWay::GistUrlError {
                message: format!("Couldn't delete gist with ID {}", gist_id),
            }
            .into())
        } else {
            Ok(())
        }
    }
}
