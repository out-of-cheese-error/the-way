//! Language specific code like highlighting and extensions
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use color_eyre::Help;
use hex::FromHex;
use serde_yaml::Value;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Color, FontStyle, Style, StyleModifier, ThemeSet};
use syntect::parsing::{SyntaxDefinition, SyntaxSet};
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
pub struct Language {
    name: String,
    extension: String,
    pub(crate) color: Color,
}

impl Default for Language {
    fn default() -> Self {
        Self::new(String::from("text"), String::from(".txt"), None).unwrap()
    }
}

impl Language {
    fn new(name: String, extension: String, color: Option<String>) -> color_eyre::Result<Self> {
        Ok(Self {
            name,
            extension,
            color: Self::get_color(color)?,
        })
    }

    fn get_color(color_string: Option<String>) -> color_eyre::Result<Color> {
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
fn read_languages_from_yml(yml_string: &str) -> color_eyre::Result<HashMap<String, LanguageYML>> {
    let language_strings: HashMap<String, Value> = serde_yaml::from_str(yml_string)?;
    let mut languages = HashMap::with_capacity(language_strings.len());
    for (key, value) in language_strings {
        languages.insert(key, serde_yaml::from_value(value)?);
    }
    Ok(languages)
}

/// Loads language extension and color information for each language and its aliases
pub fn get_languages(yml_string: &str) -> color_eyre::Result<HashMap<String, Language>> {
    let languages = read_languages_from_yml(yml_string)?;
    let mut name_to_language = HashMap::new();
    for (name, language_yml) in languages {
        if let Some(extension) = language_yml.extensions.first() {
            let mut language =
                Language::new(name.to_owned(), extension.to_owned(), language_yml.color)?;
            name_to_language.insert(name.to_ascii_lowercase(), language.clone());
            name_to_language.insert(name, language.clone());
            for alias in language_yml.aliases {
                language.name = alias.clone();
                name_to_language.insert(alias, language.clone());
            }
        }
    }
    Ok(name_to_language)
}

pub(crate) struct CodeHighlight {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
    theme_name: String,
    syntect_dir: PathBuf,
    /// Style used to print description
    pub(crate) main_style: Style,
    /// Style used to print language name
    pub(crate) accent_style: Style,
    /// Style used to print tags
    pub(crate) tag_style: Style,
    /// Style in `skim` when selecting during search
    pub(crate) highlight_style: Style,
}

impl CodeHighlight {
    /// Loads themes from `theme_dir` and default syntax set.
    /// Sets highlighting styles
    pub(crate) fn new(theme: &str, syntect_dir: PathBuf) -> color_eyre::Result<Self> {
        let mut theme_set = ThemeSet::load_defaults();
        theme_set
            .add_from_folder(&syntect_dir)
            .map_err(|_| LostTheWay::ThemeError {
                theme: String::from((&syntect_dir).to_str().unwrap()),
            })
            .suggestion(format!(
                "Make sure {:#?} is a valid directory that has .tmTheme files",
                &syntect_dir
            ))?;
        let mut syntax_set = SyntaxSet::load_defaults_newlines().into_builder();
        syntax_set
            .add_from_folder(&syntect_dir, true)
            .map_err(|_| LostTheWay::ThemeError {
                theme: String::from((&syntect_dir).to_str().unwrap()),
            })
            .suggestion(format!(
                "Make sure {:#?} is a valid directory that has .sublime-syntax files",
                &syntect_dir
            ))?;
        let syntax_set = syntax_set.build();
        let mut highlighter = Self {
            syntax_set,
            theme_name: theme.into(),
            theme_set,
            syntect_dir,
            main_style: Style::default(),
            accent_style: Style::default(),
            tag_style: Style::default(),
            highlight_style: Style::default(),
        };
        highlighter.set_styles();
        Ok(highlighter)
    }

    /// Sets styles according to current theme
    fn set_styles(&mut self) {
        self.set_main_style();
        self.set_accent_style();
        self.set_tag_style();
        self.set_highlight_style();
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
    fn set_tag_style(&mut self) {
        let tag_color = self.theme_set.themes[&self.theme_name]
            .settings
            .tags_foreground
            .unwrap_or_else(|| {
                self.theme_set.themes[&self.theme_name]
                    .settings
                    .line_highlight
                    .unwrap_or(self.main_style.foreground)
            });
        self.tag_style = self.tag_style.apply(StyleModifier {
            foreground: Some(tag_color),
            background: self.theme_set.themes[&self.theme_name].settings.background,
            font_style: Some(FontStyle::ITALIC),
        });
    }

    /// Style used to print language name
    fn set_accent_style(&mut self) {
        let accent_color = self.theme_set.themes[&self.theme_name]
            .settings
            .caret
            .unwrap_or(self.main_style.foreground);
        self.accent_style = self.accent_style.apply(StyleModifier {
            foreground: Some(accent_color),
            background: None,
            font_style: None,
        });
    }

    /// Style used to highlight lines in search
    fn set_highlight_style(&mut self) {
        let highlight_color = self.theme_set.themes[&self.theme_name]
            .settings
            .selection
            .unwrap_or(Color::WHITE);
        self.highlight_style = self.highlight_style.apply(StyleModifier {
            foreground: Some(highlight_color),
            background: None,
            font_style: None,
        });
    }

    /// Sets the current theme
    pub(crate) fn set_theme(&mut self, theme_name: String) -> color_eyre::Result<()> {
        if self.theme_set.themes.contains_key(&theme_name) {
            self.theme_name = theme_name;
            self.set_styles();
            Ok(())
        } else {
            let error: color_eyre::Result<()> =
                Err(LostTheWay::ThemeError { theme: theme_name }.into());
            error.suggestion(
                "That theme doesn't exist. \
            Use `the-way themes list` to see all theme possibilities \
            or use `the-way themes add` to add a new one.",
            )
        }
    }

    /// Gets currently available theme names
    pub(crate) fn get_themes(&self) -> Vec<String> {
        self.theme_set.themes.keys().cloned().collect()
    }

    /// Gets current theme name
    pub(crate) fn get_theme_name(&self) -> String {
        self.theme_name.to_owned()
    }

    /// Adds a new theme from a .tmTheme file.
    /// The file is copied to the themes folder
    // TODO: should it automatically be set?
    pub(crate) fn add_theme(&mut self, theme_file: &Path) -> color_eyre::Result<()> {
        let theme = ThemeSet::get_theme(&theme_file)
            .map_err(|_| LostTheWay::ThemeError {
                theme: theme_file.to_str().unwrap().into(),
            })
            .suggestion(format!(
                "Couldn't load a theme from {}, are you sure this is a valid .tmTheme file?",
                theme_file.display()
            ))?;
        let basename = theme_file
            .file_stem()
            .and_then(|x| x.to_str())
            .ok_or(LostTheWay::ThemeError {
                theme: theme_file.to_str().unwrap().into(),
            })
            .suggestion("Something's fishy with the filename, valid Unicode?")?;
        // Copy theme to theme file directory
        let new_theme_file = self.syntect_dir.join(format!("{}.tmTheme", basename));
        fs::copy(theme_file, new_theme_file)?;
        self.theme_set.themes.insert(basename.to_owned(), theme);
        Ok(())
    }

    /// Adds a new language syntax from a .sublime-syntax file.
    /// The file is copied to the themes folder
    pub(crate) fn add_syntax(&mut self, syntax_file: &Path) -> color_eyre::Result<()> {
        SyntaxDefinition::load_from_str(
            &fs::read_to_string(&syntax_file)?,
            true,
            None,
        )
            .map_err(|_| LostTheWay::SyntaxError {
                syntax: syntax_file.to_str().unwrap().into(),
            })
            .suggestion(format!(
                "Couldn't load a syntax from {}, are you sure this is a valid .sublime-syntax file with a \'name\' key?",
                syntax_file.display()
            ))?;
        let filename = syntax_file
            .file_name()
            .and_then(|x| x.to_str())
            .ok_or(LostTheWay::SyntaxError {
                syntax: syntax_file.to_str().unwrap().into(),
            })
            .suggestion("Something's fishy with the filename, valid Unicode?")?;
        // Copy syntax file to syntect dir
        let new_syntax_file = self.syntect_dir.join(filename);
        fs::copy(syntax_file, new_syntax_file)?;
        Ok(())
    }

    /// Makes a box colored according to GitHub language colors
    pub(crate) fn highlight_block(language_color: Color) -> color_eyre::Result<String> {
        Ok(Self::highlight_string(
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

    /// Syntax highlight code block
    pub(crate) fn highlight_code(
        &self,
        code: &str,
        extension: &str,
    ) -> color_eyre::Result<Vec<String>> {
        let mut colorized = Vec::new();
        let extension = extension.split('.').nth(1).unwrap_or("txt");
        let syntax = self.syntax_set.find_syntax_by_extension(extension);
        let syntax = match syntax {
            Some(syntax) => syntax,
            None => self.syntax_set.find_syntax_by_extension("txt").unwrap(),
        };
        let mut h = HighlightLines::new(syntax, &self.theme_set.themes[&self.theme_name]);
        for line in LinesWithEndings::from(code) {
            let ranges: Vec<(Style, &str)> = h.highlight(line, &self.syntax_set);
            let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
            colorized.push(escaped);
        }
        Ok(colorized)
    }
}
