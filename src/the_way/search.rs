//! Fuzzy search capabilities
use std::borrow::Cow;
use std::collections::HashSet;
use std::sync::Arc;

use skim::prelude::{unbounded, ExactOrFuzzyEngineFactory, Key, SkimOptionsBuilder};
use skim::{
    AnsiString, DisplayContext, FuzzyAlgorithm, ItemPreview, MatchEngineFactory, MatchRange,
    Matches, PreviewContext, Skim, SkimItem, SkimItemReceiver, SkimItemSender,
};
use syntect::highlighting::Style;

use crate::errors::LostTheWay;
use crate::language::Language;
use crate::the_way::{snippet::Snippet, TheWay};
use crate::utils;

/// searchable snippet information
#[derive(Debug)]
struct SearchSnippet {
    /// Snippet index
    index: usize,
    /// Highlighted title
    text_highlight: String,
    /// Code for search
    code: SearchCode,
}

// searchable snippet code
#[derive(Debug, Clone)]
struct SearchCode {
    /// Code highlighted fragments
    code_fragments: Vec<(Style, String)>,
    /// Style for matched text
    selection_style: Style,
    /// Highlighted code
    code_highlight: String,
    /// Use exact search
    exact: bool,
}

impl SkimItem for SearchCode {
    fn text(&self) -> Cow<str> {
        AnsiString::parse(&self.code_highlight).into_inner()
    }
}

impl SkimItem for SearchSnippet {
    fn text(&self) -> Cow<str> {
        AnsiString::parse(&self.text_highlight).into_inner()
            + AnsiString::parse(&self.code.code_highlight).into_inner()
    }

    fn display<'b>(&'b self, context: DisplayContext<'b>) -> AnsiString<'b> {
        let mut text = AnsiString::parse(&self.text_highlight);
        match context.matches {
            Matches::CharIndices(indices) => {
                text.override_attrs(
                    indices
                        .iter()
                        .filter(|&i| *i < self.text_highlight.len())
                        .map(|i| (context.highlight_attr, (*i as u32, (*i + 1) as u32)))
                        .collect(),
                );
            }
            Matches::CharRange(start, end) => {
                if end < self.text_highlight.len() {
                    text.override_attrs(vec![(context.highlight_attr, (start as u32, end as u32))]);
                }
            }
            Matches::ByteRange(start, end) => {
                if end < text.stripped().len() {
                    let start = text.stripped()[..start].chars().count();
                    let end = start + text.stripped()[start..end].chars().count();
                    text.override_attrs(vec![(context.highlight_attr, (start as u32, end as u32))]);
                }
            }
            Matches::None => (),
        }
        text
    }

    fn preview(&self, context: PreviewContext) -> ItemPreview {
        if context.selected_indices.contains(&context.current_index) {
            let fuzzy_engine = ExactOrFuzzyEngineFactory::builder()
                .exact_mode(self.code.exact)
                .fuzzy_algorithm(FuzzyAlgorithm::SkimV2)
                .build()
                .create_engine(context.query);
            fuzzy_engine
                .match_item(Arc::new(self.code.clone()))
                .map_or_else(
                    || ItemPreview::AnsiText(self.code.code_highlight.clone()),
                    |match_result| {
                        let indices: HashSet<_> = match match_result.matched_range {
                            MatchRange::ByteRange(start, end) => (start..end).collect(),
                            MatchRange::Chars(indices) => indices.into_iter().collect(),
                        };
                        ItemPreview::AnsiText(
                            self.code
                                .code_fragments
                                .iter()
                                .flat_map(|(style, line)| {
                                    line.chars().map(move |c| (*style, c.to_string()))
                                })
                                .enumerate()
                                .map(|(i, (style, line))| {
                                    if indices.contains(&i) {
                                        utils::highlight_strings(
                                            &[(self.code.selection_style, line)],
                                            true,
                                        )
                                    } else {
                                        utils::highlight_string(&line, style)
                                    }
                                })
                                .collect::<Vec<_>>()
                                .join(""),
                        )
                    },
                )
        } else {
            ItemPreview::AnsiText(self.code.code_highlight.clone())
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum SkimCommand {
    Copy,
    Delete,
    Edit,
    View,
    All,
}

impl SkimCommand {
    pub fn keys(&self) -> Vec<&'static str> {
        match self {
            SkimCommand::Copy | SkimCommand::Delete | SkimCommand::Edit | SkimCommand::View => {
                vec!["Enter"]
            }
            SkimCommand::All => vec!["Enter", "shift-left", "shift-right"],
        }
    }

    pub fn names(&self) -> Vec<&'static str> {
        match self {
            SkimCommand::Copy => vec!["copy"],
            SkimCommand::Delete => vec!["delete"],
            SkimCommand::Edit => vec!["edit"],
            SkimCommand::View => vec!["view"],
            SkimCommand::All => vec!["copy", "delete", "edit"],
        }
    }
}

impl TheWay {
    /// Converts a list of snippets into searchable objects and opens a fuzzy search window with the
    /// bottom panel listing each snippet's index, description, language and tags
    /// and the top panel showing the code for the selected snippet (all searchable).
    pub(crate) fn make_search(
        &mut self,
        snippets: Vec<Snippet>,
        skim_theme: String,
        selection_style: Style,
        exact: bool,
        command: SkimCommand,
        stdout: bool,
        force: bool,
    ) -> color_eyre::Result<()> {
        let default_language = Language::default();

        let mut search_snippets = Vec::with_capacity(snippets.len());
        for snippet in snippets {
            let language = self
                .languages
                .get(&snippet.language)
                .unwrap_or(&default_language);
            let code_fragments = self
                .highlighter
                .highlight_code(&snippet.code, &snippet.extension)?;
            let code_highlight = utils::highlight_strings(&code_fragments, false);
            search_snippets.push(SearchSnippet {
                code: SearchCode {
                    code_fragments,
                    selection_style,
                    code_highlight,
                    exact,
                },
                text_highlight: utils::highlight_strings(
                    &snippet.pretty_print_header(&self.highlighter, language),
                    false,
                ),
                index: snippet.index,
            });
        }
        let bind = command
            .keys()
            .into_iter()
            .map(|s| format!("{s}:accept"))
            .collect::<Vec<_>>();
        let header = format!(
            "Press {}",
            command
                .keys()
                .into_iter()
                .zip(command.names().into_iter())
                .map(|(key, name)| format!("{key} to {name}"))
                .collect::<Vec<_>>()
                .join(", "),
        );

        let options = SkimOptionsBuilder::default()
            .height(Some("100%"))
            .preview(Some(""))
            .preview_window(Some("up:70%:wrap"))
            .bind(bind.iter().map(|s| s.as_ref()).collect())
            .header(Some(&header))
            .exact(exact)
            .multi(true)
            .reverse(true)
            .color(Some(&skim_theme))
            .build()
            .map_err(|_e| LostTheWay::SearchError)?;

        let (tx_item, rx_item): (SkimItemSender, SkimItemReceiver) = unbounded();
        for item in search_snippets {
            tx_item.send(Arc::new(item))?;
        }
        drop(tx_item); // so that skim could know when to stop waiting for more items.

        if let Some(output) = Skim::run_with(&options, Some(rx_item)) {
            let key = output.final_key;
            for item in &output.selected_items {
                let snippet: &SearchSnippet = (*item)
                    .as_any()
                    .downcast_ref::<SearchSnippet>()
                    .ok_or(LostTheWay::SearchError)?;

                match (command, key) {
                    (SkimCommand::Copy, Key::Enter) => {
                        self.copy(snippet.index, stdout)?;
                    }
                    (SkimCommand::Delete, Key::Enter) => {
                        self.delete(snippet.index, force)?;
                    }
                    (SkimCommand::Edit, Key::Enter) => {
                        self.edit(snippet.index)?;
                    }
                    (SkimCommand::View, Key::Enter) => {
                        self.view(snippet.index)?;
                    }
                    (SkimCommand::All, Key::Enter) => {
                        self.copy(snippet.index, stdout)?;
                    }
                    (SkimCommand::All, Key::ShiftLeft) => {
                        self.delete(snippet.index, force)?;
                    }
                    (SkimCommand::All, Key::ShiftRight) => {
                        self.edit(snippet.index)?;
                    }
                    _ => (),
                }
            }
        }
        Ok(())
    }
}
