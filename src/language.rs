use std::collections::HashMap;

use anyhow::Error;
use hex::FromHex;
use path_abs::{PathDir, PathFile, PathInfo, PathOps};
use serde_yaml::Value;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Color, FontStyle, Style, StyleModifier, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};
use syntect::LoadingError;

use crate::errors::LostTheWay;
use crate::utils;
use crate::utils::END_ANSI;

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
    pub(crate) syntax_set: SyntaxSet,
    pub(crate) theme_set: ThemeSet,
    pub(crate) theme_name: String,
    pub(crate) theme_dir: PathDir,
    pub(crate) main_style: Style,
    pub(crate) accent_style: Style,
    pub(crate) dim_style: Style,
}

impl CodeHighlight {
    pub(crate) fn new(theme: &str, theme_dir: PathDir) -> Result<Self, Error> {
        let mut theme_set = ThemeSet::load_defaults();
        theme_set.add_from_folder(&theme_dir).unwrap();
        let mut highlighter = Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_name: theme.into(),
            theme_set,
            theme_dir,
            main_style: Style::default(),
            accent_style: Style::default(),
            dim_style: Style::default(),
        };
        highlighter.get_styles();
        Ok(highlighter)
    }

    fn get_styles(&mut self) {
        self.get_main_style();
        self.get_accent_style();
        self.get_dim_style();
    }

    fn get_main_style(&mut self) {
        let main_color = self.theme_set.themes[&self.theme_name]
            .settings
            .foreground
            .unwrap_or(Color::WHITE);
        self.main_style.apply(StyleModifier {
            foreground: Some(main_color),
            background: None,
            font_style: Some(FontStyle::BOLD),
        });
    }

    fn get_dim_style(&self) {
        let dim_color = self.theme_set.themes[&self.theme_name]
            .settings
            .selection
            .unwrap_or(Color::WHITE);
        self.dim_style.apply(StyleModifier {
            foreground: Some(dim_color),
            background: None,
            font_style: Some(FontStyle::ITALIC),
        });
    }

    fn get_accent_style(&self) {
        let accent_color = self.theme_set.themes[&self.theme_name]
            .settings
            .caret
            .unwrap_or(Color::WHITE);
        self.accent_style.apply(StyleModifier {
            foreground: Some(accent_color),
            background: None,
            font_style: None,
        });
    }

    pub(crate) fn set_theme(&mut self, theme_name: String) -> Result<(), Error> {
        if self.theme_set.themes.contains_key(&theme_name) {
            self.theme_name = theme_name;
            self.get_styles();
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

    pub(crate) fn highlight_block(
        &self,
        language_color_string: &Option<String>,
    ) -> Result<String, Error> {
        let mut language_color = [0; 3];
        if let Some(color) = language_color_string {
            language_color = <[u8; 3]>::from_hex(&color[1..])?;
        }
        Ok(self.highlight_string(
            &format!("■ {}", END_ANSI),
            Style::default().apply(StyleModifier {
                foreground: Some(Color {
                    r: language_color[0],
                    g: language_color[1],
                    b: language_color[2],
                    a: 0xFF,
                }),
                background: None,
                font_style: None,
            }),
        ))
    }

    pub(crate) fn highlight_string(&self, line: &str, style: Style) -> String {
        as_24_bit_terminal_escaped(&[(style, line)], false)
    }

    pub(crate) fn highlight_code(&self, code: &str, extension: &str) -> Result<Vec<String>, Error> {
        let mut colorized = vec![String::from("\n")];
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
            colorized.push(escaped);
        }
        colorized.push(String::from("\n"));
        colorized.push(String::from("\n"));
        colorized.push(String::from(utils::END_ANSI));
        Ok(colorized)
    }
}
