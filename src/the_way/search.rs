//! Fuzzy search capabilities
use std::borrow::Cow;
use std::sync::Arc;

use skim::prelude::{unbounded, Key, SkimOptionsBuilder};
use skim::{
    AnsiString, DisplayContext, ItemPreview, Matches, PreviewContext, Skim, SkimItem,
    SkimItemReceiver, SkimItemSender,
};

use crate::errors::LostTheWay;
use crate::language::Language;
use crate::the_way::{snippet::Snippet, TheWay};

/// searchable snippet information
#[derive(Debug)]
struct SearchSnippet {
    index: usize,
    /// Highlighted title
    text_highlight: String,
    /// Highlighted code
    code_highlight: String,
}

impl<'a> SkimItem for SearchSnippet {
    fn text(&self) -> Cow<str> {
        AnsiString::parse(&self.text_highlight).into_inner()
    }

    fn display<'b>(&'b self, context: DisplayContext<'b>) -> AnsiString<'b> {
        let mut text = AnsiString::parse(&self.text_highlight);
        match context.matches {
            Matches::CharIndices(indices) => {
                text.override_attrs(
                    indices
                        .iter()
                        // TODO: Why is this i+2 to i+3?
                        .map(|i| (context.highlight_attr, ((*i + 2) as u32, (*i + 3) as u32)))
                        .collect(),
                );
            }
            Matches::CharRange(start, end) => {
                text.override_attrs(vec![(context.highlight_attr, (start as u32, end as u32))]);
            }
            Matches::ByteRange(start, end) => {
                let start = text.stripped()[..start].chars().count();
                let end = start + text.stripped()[start..end].chars().count();
                text.override_attrs(vec![(context.highlight_attr, (start as u32, end as u32))]);
            }
            Matches::None => (),
        }
        text
    }

    fn preview(&self, _context: PreviewContext) -> ItemPreview {
        ItemPreview::AnsiText(self.code_highlight.to_owned())
    }
}

impl TheWay {
    /// Converts a list of snippets into searchable objects and opens a fuzzy search window with the
    /// bottom panel listing each snippet's index, description, language and tags (all searchable)
    /// and the top panel showing the code for the selected snippet.
    pub(crate) fn make_search(
        &mut self,
        snippets: Vec<Snippet>,
        highlight_color: &str,
    ) -> color_eyre::Result<()> {
        let default_language = Language::default();
        let search_snippets: Vec<_> = snippets
            .into_iter()
            .map(|snippet| SearchSnippet {
                code_highlight: self
                    .highlighter
                    .highlight_code(&snippet.code, &snippet.extension)
                    .unwrap_or_default()
                    .join(""),
                text_highlight: snippet
                    .pretty_print_header(
                        &self.highlighter,
                        self.languages
                            .get(&snippet.language)
                            .unwrap_or(&default_language),
                    )
                    .unwrap_or_default()
                    .join(""),
                index: snippet.index,
            })
            .collect();
        let color = format!("bg+:{}", highlight_color);
        let options = SkimOptionsBuilder::default()
            .height(Some("100%"))
            .preview(Some(""))
            .preview_window(Some("up:70%"))
            .bind(vec![
                "shift-left:accept",
                "shift-right:accept",
                "Enter:accept",
            ])
            .header(Some(
                "Press Enter to copy, Shift-right to delete, Shift-left to edit",
            ))
            .multi(true)
            .reverse(true)
            .color(Some(&color))
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
                        self.copy(snippet.index)?;
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
