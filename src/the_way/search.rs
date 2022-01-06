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
    index: usize,
    /// Highlighted title
    text_highlight: String,
    /// Code for search
    code: SearchCode,
}

#[derive(Debug, Clone)]
struct SearchCode {
    /// Code highlighted fragments
    code_fragments: Vec<(Style, String)>,
    /// Style for matched text
    selection_style: Style,
}

impl SearchCode {
    fn get_code(&self) -> String {
        self.code_fragments
            .iter()
            .map(|(_, line)| line.as_ref())
            .collect::<Vec<_>>()
            .join("")
    }
}

impl SkimItem for SearchCode {
    fn text(&self) -> Cow<str> {
        AnsiString::parse(&self.get_code()).into_inner()
    }
}

impl<'a> SkimItem for SearchSnippet {
    fn text(&self) -> Cow<str> {
        AnsiString::parse(&self.text_highlight).into_inner()
            + AnsiString::parse(&self.code.get_code()).into_inner()
    }

    fn display<'b>(&'b self, context: DisplayContext<'b>) -> AnsiString<'b> {
        let mut text = AnsiString::parse(&self.text_highlight);
        match context.matches {
            Matches::CharIndices(indices) => {
                text.override_attrs(
                    indices
                        .iter()
                        .filter(|&i| *i < self.text_highlight.len() - 1)
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
                .exact_mode(true)
                .fuzzy_algorithm(FuzzyAlgorithm::SkimV2)
                .build()
                .create_engine(context.query);
            if let Some(match_result) = fuzzy_engine.match_item(Arc::new(self.code.clone())) {
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
                                utils::highlight_strings(&[(self.code.selection_style, line)], true)
                            } else {
                                utils::highlight_string(&line, style)
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(""),
                )
            } else {
                ItemPreview::AnsiText(utils::highlight_strings(&self.code.code_fragments, false))
            }
        } else {
            ItemPreview::AnsiText(utils::highlight_strings(&self.code.code_fragments, false))
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
        stdout: bool,
    ) -> color_eyre::Result<()> {
        let default_language = Language::default();

        let search_snippets: Vec<_> = snippets
            .into_iter()
            .map(|snippet| {
                let language = self
                    .languages
                    .get(&snippet.language)
                    .unwrap_or(&default_language);
                SearchSnippet {
                    code: SearchCode {
                        code_fragments: self
                            .highlighter
                            .highlight_code(&snippet.code, &snippet.extension),
                        selection_style,
                    },
                    text_highlight: utils::highlight_strings(
                        &snippet.pretty_print_header(&self.highlighter, &language),
                        false,
                    ),
                    index: snippet.index,
                }
            })
            .collect();
        let options = SkimOptionsBuilder::default()
            .height(Some("100%"))
            .preview(Some(""))
            .preview_window(Some("up:70%:wrap"))
            .bind(vec![
                "shift-left:accept",
                "shift-right:accept",
                "Enter:accept",
            ])
            .header(Some(
                "Press Enter to copy, Shift-left to delete, Shift-right to edit",
            ))
            .exact(true)
            .multi(true)
            .reverse(true)
            .color(Some(&skim_theme))
            .build()
            .map_err(|_| LostTheWay::SearchError)?;

        let (tx_item, rx_item): (SkimItemSender, SkimItemReceiver) = unbounded();
        for item in search_snippets {
            let _ = tx_item.send(Arc::new(item));
        }
        drop(tx_item); // so that skim could know when to stop waiting for more items.

        if let Some(output) = Skim::run_with(&options, Some(rx_item)) {
            let key = output.final_key;
            for item in &output.selected_items {
                let snippet: &SearchSnippet =
                    (*item).as_any().downcast_ref::<SearchSnippet>().unwrap();
                match key {
                    Key::Enter => {
                        self.copy(snippet.index, stdout)?;
                    }
                    Key::ShiftLeft => {
                        self.delete(snippet.index, false)?;
                    }
                    Key::ShiftRight => {
                        self.edit(snippet.index)?;
                    }
                    _ => (),
                }
            }
        }
        Ok(())
    }
}
