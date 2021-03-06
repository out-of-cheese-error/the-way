//! Snippet information and methods
use std::borrow::Cow;
use std::collections::HashMap;
use std::io;

use chrono::{DateTime, Utc};
use regex::Regex;

use crate::gist::Gist;
use crate::language::{CodeHighlight, Language};
use crate::utils;

/// Stores information about a quote
#[derive(Serialize, Deserialize, Debug)]
pub struct Snippet {
    /// Snippet index, used to retrieve, copy, or modify a snippet
    #[serde(default)]
    pub index: usize,
    /// Snippet description, what does it do?
    pub description: String,
    /// Language the snippet is written in
    pub language: String,
    /// Snippet code
    pub code: String,
    /// extension
    #[serde(default)]
    pub extension: String,
    /// Tags attached to the snippet
    #[serde(default)]
    pub tags: Vec<String>,
    /// Time of recording the snippet
    #[serde(default = "Utc::now")]
    pub date: DateTime<Utc>,
    /// Time of last update
    #[serde(default = "Utc::now")]
    pub updated: DateTime<Utc>,
}

impl Snippet {
    /// New snippet
    fn new(
        index: usize,
        description: String,
        language: String,
        extension: String,
        tags: &str,
        date: DateTime<Utc>,
        updated: DateTime<Utc>,
        code: String,
    ) -> Self {
        Self {
            index,
            description,
            language,
            extension,
            tags: utils::split_tags(tags),
            date,
            updated,
            code,
        }
    }

    pub(crate) fn set_extension(
        &mut self,
        language_name: &str,
        languages: &HashMap<String, Language>,
    ) {
        self.extension = Language::get_extension(language_name, languages);
    }

    /// Queries user for new snippet info
    pub(crate) fn from_user(
        index: usize,
        languages: &HashMap<String, Language>,
        old_snippet: Option<&Self>,
    ) -> color_eyre::Result<Self> {
        let (old_description, old_language, old_tags, old_date, old_code) = match old_snippet {
            Some(s) => (
                Some(s.description.as_str()),
                Some(s.language.as_str()),
                Some(s.tags.join(" ")),
                Some(s.date.date().format("%Y-%m-%d").to_string()),
                Some(s.code.as_str()),
            ),
            None => (None, None, None, None, None),
        };

        let description = utils::user_input("Description", old_description, true, false)?;
        let language =
            utils::user_input("Language", old_language, true, false)?.to_ascii_lowercase();
        let extension = Language::get_extension(&language, languages);
        let tags = utils::user_input("Tags (space separated)", old_tags.as_deref(), true, true)?;
        let date = match old_date {
            Some(_) => utils::parse_date(&utils::user_input(
                "Date",
                old_date.as_deref(),
                true,
                false,
            )?)?
            .and_hms(0, 0, 0),
            None => Utc::now(),
        };
        let show_default = old_code
            .map(|c| c.split('\n').nth(1).is_none())
            .unwrap_or(false);
        let mut code = utils::user_input(
            "Code snippet (<RET> to edit in external editor)",
            if show_default { old_code } else { None },
            show_default,
            true,
        )?;
        if code.is_empty() {
            code = utils::external_editor_input(old_code.as_deref(), &extension)?;
        }
        Ok(Self::new(
            index,
            description,
            language,
            extension,
            &tags,
            date,
            Utc::now(),
            code,
        ))
    }

    /// Queries user for new shell snippet info
    pub(crate) fn cmd_from_user(index: usize, code: Option<&str>) -> color_eyre::Result<Self> {
        let code = utils::user_input("Command", code, true, false)?;
        let description = utils::user_input("Description", None, true, false)?;
        let tags = utils::user_input("Tags (space separated)", None, true, true)?;
        Ok(Self::new(
            index,
            description,
            "sh".into(),
            ".sh".into(),
            &tags,
            Utc::now(),
            Utc::now(),
            code,
        ))
    }

    /// write snippet to database
    pub(crate) fn to_bytes(&self) -> color_eyre::Result<Vec<u8>> {
        Ok(bincode::serialize(&self)?)
    }

    /// read snippet from database
    pub(crate) fn from_bytes(bytes: &[u8]) -> color_eyre::Result<Self> {
        Ok(bincode::deserialize(bytes)?)
    }

    /// Read snippets from a JSON stream and return consumable iterator
    pub(crate) fn read(
        json_reader: &mut dyn io::Read,
    ) -> impl Iterator<Item = serde_json::Result<Self>> + '_ {
        serde_json::Deserializer::from_reader(json_reader).into_iter::<Self>()
    }

    /// Appends a snippet to a JSON object/file
    pub(crate) fn to_json(&self, json_writer: &mut dyn io::Write) -> color_eyre::Result<()> {
        serde_json::to_writer(json_writer, self)?;
        Ok(())
    }

    /// Read potentially multiple snippets from a regular Gist (not a sync/backup Gist)
    pub(crate) fn from_gist(
        start_index: usize,
        languages: &HashMap<String, Language>,
        gist: &Gist,
    ) -> Vec<Self> {
        let mut index = start_index;
        let mut snippets = Vec::new();
        for (file_name, gist_file) in &gist.files {
            let code = &gist_file.content;
            let description = format!("{} - {} - {}", gist.description, gist.id, file_name);
            let language = &gist_file.language;
            let tags = "gist";
            let extension = Language::get_extension(&language, languages);
            let snippet = Self::new(
                index,
                description,
                language.to_string(),
                extension.to_string(),
                &tags,
                Utc::now(),
                Utc::now(),
                code.to_string(),
            );
            snippets.push(snippet);
            index += 1;
        }
        snippets
    }

    /// Filters snippets in date range
    pub(crate) fn filter_in_date_range(
        snippets: Vec<Self>,
        from_date: DateTime<Utc>,
        to_date: DateTime<Utc>,
    ) -> Vec<Self> {
        snippets
            .into_iter()
            .filter(|snippet| snippet.in_date_range(from_date, to_date))
            .collect()
    }

    /// Checks if a snippet was recorded within a date range
    pub(crate) fn in_date_range(&self, from_date: DateTime<Utc>, to_date: DateTime<Utc>) -> bool {
        from_date <= self.date && self.date < to_date
    }

    /// Check if a snippet has a particular tag associated with it
    pub(crate) fn has_tag(&self, tag: &str) -> bool {
        self.tags.contains(&tag.into())
    }

    /// Highlights the title: "■ #index. description | language :tag1:tag2:\n"
    /// the block is colored according to the language
    /// language uses `accent_style`
    /// tags use `dim_style`
    /// everything else is in `main_style`
    pub(crate) fn pretty_print_header(
        &self,
        highlighter: &CodeHighlight,
        language: &Language,
    ) -> Vec<String> {
        let mut colorized = Vec::new();
        let block = CodeHighlight::highlight_block(language.color);
        colorized.push(block);
        let text = format!("#{}. {} ", self.index, self.description);
        colorized.push(utils::highlight_string(&text, highlighter.main_style));

        let text = format!("| {} ", self.language);
        colorized.push(utils::highlight_string(&text, highlighter.accent_style));

        let text = format!(":{}:\n", self.tags.join(":"));
        colorized.push(utils::highlight_string(&text, highlighter.tag_style));
        colorized
    }

    pub(crate) fn pretty_print(
        &self,
        highlighter: &CodeHighlight,
        language: &Language,
    ) -> Vec<String> {
        let mut colorized = vec![String::from("\n")];
        colorized.extend_from_slice(&self.pretty_print_header(highlighter, language));
        colorized.push(String::from("\n"));
        colorized.extend_from_slice(&highlighter.highlight_code(&self.code, &self.extension));
        colorized.push(String::from("\n"));
        colorized.push(String::from("\n"));
        colorized
    }

    fn is_shell_snippet(&self) -> bool {
        // sh, bash, csh, tcsh
        matches!(self.language.as_str(), "sh" | "bash" | "csh" | "tcsh")
    }

    /// If snippet is a shell snippet, interactively fill parameters
    pub(crate) fn fill_snippet(&self) -> color_eyre::Result<Cow<str>> {
        // other languages, return as is
        if !self.is_shell_snippet() {
            return Ok(Cow::Borrowed(self.code.as_str()));
        }
        // Matches the param or param=value **inside** the angular brackets
        let re1 = Regex::new("<(?P<parameter>[^<>]+)>")?;
        // Matches <param> or <param=value>
        let re2 = Regex::new("(?P<match><[^<>]+>)")?;

        // Ask user to fill in (unique) parameters
        let mut filled_parameters = HashMap::new();
        eprintln!("{}", self.code);
        for capture in re1.captures_iter(&self.code) {
            let mut parts = capture["parameter"].split('=');
            let parameter_name = parts.next().unwrap().to_owned();
            let default = parts.next();
            if !filled_parameters.contains_key(&parameter_name) {
                let filled = utils::user_input(&parameter_name, default, true, false)?;
                filled_parameters.insert(parameter_name, filled);
            }
        }

        // Replace parameters in code
        Ok(re2.replace_all(&self.code, |caps: &regex::Captures| {
            let parameter = caps["match"][1..caps["match"].len() - 1]
                .split('=')
                .next()
                .unwrap();
            &filled_parameters[parameter]
        }))
    }
}
