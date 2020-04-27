use std::collections::HashMap;

use anyhow::Error;
use path_abs::{PathDir, PathFile, PathInfo, PathOps};
use serde_yaml::Value;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};
use syntect::LoadingError;

use crate::errors::LostTheWay;

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
pub(crate) struct Language {
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

pub(crate) struct CodeHighlight {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
    theme_name: String,
    theme_dir: PathDir,
}

impl CodeHighlight {
    pub(crate) fn new(theme: &str, theme_dir: PathDir) -> Result<Self, Error> {
        let mut theme_set = ThemeSet::load_defaults();
        theme_set.add_from_folder(&theme_dir).unwrap();
        Ok(Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_name: theme.into(),
            theme_set,
            theme_dir,
        })
    }

    pub(crate) fn set_theme(&mut self, theme_name: String) -> Result<(), Error> {
        if self.theme_set.themes.contains_key(&theme_name) {
            self.theme_name = theme_name;
            Ok(())
        } else {
            Err(LostTheWay::ThemeNotFound { theme_name }.into())
        }
    }

    pub(crate) fn get_themes(&self) -> Vec<String> {
        self.theme_set.themes.keys().cloned().collect()
    }

    pub(crate) fn add_theme(&mut self, theme_file: &PathFile) -> Result<(), Error> {
        let basename = theme_file
            .file_stem()
            .and_then(|x| x.to_str())
            .ok_or(LoadingError::BadPath)
            .unwrap();
        // Copy theme to theme file directory
        let theme_file = theme_file.copy(PathFile::new(
            self.theme_dir.join(format!("{}.tmTheme", basename)),
        )?)?;
        let theme = ThemeSet::get_theme(theme_file).unwrap();
        self.theme_set.themes.insert(basename.to_owned(), theme);
        Ok(())
    }

    pub(crate) fn highlight(&self, code: &str, extension: &str) -> Result<(), Error> {
        let extension = extension.split('.').nth(1).unwrap();
        let syntax = self.syntax_set.find_syntax_by_extension(extension).ok_or(
            LostTheWay::LanguageNotFound {
                language: extension.into(),
            },
        )?;
        let mut h = HighlightLines::new(syntax, &self.theme_set.themes[&self.theme_name]);
        for line in LinesWithEndings::from(code) {
            let ranges: Vec<(Style, &str)> = h.highlight(line, &self.syntax_set);
            let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
            print!("{}", escaped);
        }
        println!();
        Ok(())
    }
}
