use std::collections::HashMap;

use anyhow::Error;
use serde_yaml::Value;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct LanguageYML {
    #[serde(default)]
    extensions: Vec<String>,
    #[serde(default)]
    aliases: Vec<String>,
    #[serde(default)]
    color: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Language {
    pub(crate) name: String,
    pub(crate) extension: String,
    pub(crate) color: Option<String>, // TODO: use a color type from whatever you're rendering with
}

impl Default for Language {
    fn default() -> Self {
        Language::new(
            String::from("text"),
            String::from(".txt"),
            Some(String::from("#ffffff")),
        )
    }
}

impl Language {
    fn new(name: String, extension: String, color: Option<String>) -> Self {
        Language {
            name,
            extension,
            color,
        }
    }
}

fn read_languages_from_yml(yml_string: &str) -> Result<HashMap<String, LanguageYML>, Error> {
    let language_strings: HashMap<String, Value> = serde_yaml::from_str(yml_string)?;
    let mut languages = HashMap::with_capacity(language_strings.len());
    for (key, value) in language_strings {
        languages.insert(key, serde_yaml::from_value(value)?);
    }
    Ok(languages)
}

pub(crate) fn get_languages(yml_string: &str) -> Result<HashMap<String, Language>, Error> {
    let languages = read_languages_from_yml(yml_string)?;
    let mut name_to_language = HashMap::new();
    for (name, language_yml) in languages {
        if let Some(extension) = language_yml.extensions.first() {
            let mut language =
                Language::new(name.clone(), extension.to_owned(), language_yml.color);
            name_to_language.insert(name.to_ascii_lowercase(), language.clone());
            name_to_language.insert(name, language.clone());
            for alias in language_yml.aliases {
                language.name = alias.clone();
                name_to_language.insert(alias.to_owned(), language.clone());
            }
        }
    }
    Ok(name_to_language)
}
