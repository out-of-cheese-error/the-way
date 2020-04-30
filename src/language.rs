//! Language specific code like highlighting and extensions
use std::collections::HashMap;

use anyhow::Error;
use hex::FromHex;
use path_abs::{PathDir, PathFile, PathInfo, PathOps};
use serde_yaml::Value;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Color, FontStyle, Style, StyleModifier, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

use crate::errors::LostTheWay;
use crate::utils;

/// Relevant information from languages.yml file
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
    name: String,
    pub(crate) extension: String,
    pub(crate) color: Color,
}

impl Default for Language {
    fn default() -> Self {
        Self::new(String::from("text"), String::from(".txt"), None).unwrap()
    }
}

impl Language {
    fn new(name: String, extension: String, color: Option<String>) -> Result<Self, Error> {
        Ok(Self {
            name,
            extension,
            color: Self::get_color(color)?,
        })
    }

    fn get_color(color_string: Option<String>) -> Result<Color, Error> {
        let mut language_color = [0; 3];
        if let Some(color) = color_string {
            language_color = <[u8; 3]>::from_hex(&color.get(1..).unwrap_or("FFFFFF"))?;
        }
        Ok(Color {
            r: language_color[0],
            g: language_color[1],
            b: language_color[2],
            a: 0xFF,
        })
    }

    /// Finds the appropriate file extension for a language
    pub(crate) fn get_extension(language_name: &str, languages: &HashMap<String, Self>) -> String {
        let default = Self::default();
        if let Some(l) = languages.get(language_name) {
            l.extension.to_owned()
        } else {
            eprintln!(
                "Couldn't find language {} in the list of extensions, defaulting to .txt",
                language_name
            );
            default.extension
        }
    }
}

/// Loads language information from GitHub's languages.yml file
// TODO: find a way to keep this up to date without downloading it every time
fn read_languages_from_yml(yml_string: &str) -> Result<HashMap<String, LanguageYML>, Error> {
    let language_strings: HashMap<String, Value> = serde_yaml::from_str(yml_string)?;
    let mut languages = HashMap::with_capacity(language_strings.len());
    for (key, value) in language_strings {
        languages.insert(key, serde_yaml::from_value(value)?);
    }
    Ok(languages)
}

/// Loads language extension and color information for each language and its aliases
pub(crate) fn get_languages(yml_string: &str) -> Result<HashMap<String, Language>, Error> {
    let languages = read_languages_from_yml(yml_string)?;
    let mut name_to_language = HashMap::new();
    for (name, language_yml) in languages {
        if let Some(extension) = language_yml.extensions.first() {
            let mut language =
                Language::new(name.clone(), extension.to_owned(), language_yml.color)?;
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
    pub(crate) main_style: Style,
    pub(crate) accent_style: Style,
    pub(crate) dim_style: Style,
}

impl CodeHighlight {
    /// Loads themes from theme_dir and default syntax set.
    /// Sets highlighting styles
    pub(crate) fn new(theme: &str, theme_dir: PathDir) -> Result<Self, Error> {
        let mut theme_set = ThemeSet::load_defaults();
        theme_set
            .add_from_folder(&theme_dir)
            .map_err(|_| LostTheWay::ThemeNotFound {
                theme_name: theme_dir.to_str().unwrap().into(),
            })?;
        let mut highlighter = Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_name: theme.into(),
            theme_set,
            theme_dir,
            main_style: Style::default(),
            accent_style: Style::default(),
            dim_style: Style::default(),
        };
        highlighter.set_styles();
        Ok(highlighter)
    }

    /// Sets styles according to current theme
    fn set_styles(&mut self) {
        self.set_main_style();
        self.set_accent_style();
        self.set_dim_style();
    }

    /// Style used to print description
    fn set_main_style(&mut self) {
        let main_color = self.theme_set.themes[&self.theme_name]
            .settings
            .foreground
            .unwrap_or(Color::WHITE);
        self.main_style = self.main_style.apply(StyleModifier {
            foreground: Some(main_color),
            background: None,
            font_style: Some(FontStyle::BOLD),
        });
    }

    /// Style used to print tags
    // TODO: this is too dim sometimes
    fn set_dim_style(&mut self) {
        let dim_color = self.theme_set.themes[&self.theme_name]
            .settings
            .selection
            .unwrap_or(Color::WHITE);
        self.dim_style = self.dim_style.apply(StyleModifier {
            foreground: Some(dim_color),
            background: None,
            font_style: Some(FontStyle::ITALIC),
        });
    }

    /// Style used to print language name
    fn set_accent_style(&mut self) {
        let accent_color = self.theme_set.themes[&self.theme_name]
            .settings
            .caret
            .unwrap_or(Color::WHITE);
        self.accent_style = self.accent_style.apply(StyleModifier {
            foreground: Some(accent_color),
            background: None,
            font_style: None,
        });
    }

    /// Sets the current theme
    pub(crate) fn set_theme(&mut self, theme_name: String) -> Result<(), Error> {
        if self.theme_set.themes.contains_key(&theme_name) {
            self.theme_name = theme_name;
            self.set_styles();
            Ok(())
        } else {
            Err(LostTheWay::ThemeNotFound { theme_name }.into())
        }
    }

    /// Gets currently available theme names
    pub(crate) fn get_themes(&self) -> Vec<String> {
        self.theme_set.themes.keys().cloned().collect()
    }

    /// Adds a new theme from a .tmTheme file.
    /// The file is copied to the themes folder
    // TODO: should it automatically be set?
    pub(crate) fn add_theme(&mut self, theme_file: &PathFile) -> Result<(), Error> {
        let basename =
            theme_file
                .file_stem()
                .and_then(|x| x.to_str())
                .ok_or(LostTheWay::ThemeNotFound {
                    theme_name: theme_file.as_path().to_str().unwrap().into(),
                })?;
        // Copy theme to theme file directory
        let theme_file = theme_file.copy(PathFile::create(
            self.theme_dir.join(format!("{}.tmTheme", basename)),
        )?)?;
        let theme = ThemeSet::get_theme(&theme_file).map_err(|_| LostTheWay::ThemeNotFound {
            theme_name: theme_file.as_path().to_str().unwrap().into(),
        })?;
        self.theme_set.themes.insert(basename.to_owned(), theme);
        Ok(())
    }

    /// Makes a box colored according to GitHub language colors
    pub(crate) fn highlight_block(&self, language_color: Color) -> Result<String, Error> {
        Ok(CodeHighlight::highlight_string(
            &format!("{} ", utils::BOX),
            Style::default().apply(StyleModifier {
                foreground: Some(language_color),
                background: None,
                font_style: None,
            }),
        ))
    }

    /// Applies a style to a single line
    pub(crate) fn highlight_string(line: &str, style: Style) -> String {
        as_24_bit_terminal_escaped(&[(style, line)], false)
    }

    pub(crate) fn highlight_code(&self, code: &str, extension: &str) -> Result<Vec<String>, Error> {
        let mut colorized = Vec::new();
        let extension = extension.split('.').nth(1).unwrap_or(".txt");
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
        Ok(colorized)
    }
}
