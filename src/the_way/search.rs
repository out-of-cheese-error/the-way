use std::borrow::Cow;
use std::sync::Arc;

use anyhow::Error;
use skim::prelude::{unbounded, SkimOptionsBuilder};
use skim::{AnsiString, ItemPreview, Skim, SkimItem, SkimItemReceiver, SkimItemSender};

use crate::the_way::snippet::Snippet;
use crate::the_way::TheWay;
use crate::utils::copy_to_clipboard;

#[derive(Debug)]
struct SearchSnippet {
    index: usize,
    text_highlight: String,
    text: String,
    code_highlight: String,
    code: String,
}

impl<'a> SkimItem for SearchSnippet {
    fn display(&self) -> Cow<AnsiString> {
        Cow::Owned(AnsiString::parse(&self.text_highlight))
    }

    fn text(&self) -> Cow<str> {
        Cow::Owned(self.text.to_owned())
    }

    fn preview(&self) -> ItemPreview {
        ItemPreview::AnsiText(self.code_highlight.to_owned())
    }

    fn output(&self) -> Cow<str> {
        copy_to_clipboard(self.code.to_owned());
        let text = format!("Copied snippet #{} to clipboard", self.index);
        Cow::Owned(text)
    }
}

impl<'a> TheWay<'a> {
    pub(crate) fn make_search(&self, snippets: Vec<Snippet>) -> Result<(), Error> {
        let search_snippets: Vec<_> = snippets
            .into_iter()
            .map(|snippet| SearchSnippet {
                code_highlight: snippet
                    .pretty_print_code(&self.highlighter)
                    .unwrap_or_default()
                    .join(""),
                text_highlight: snippet
                    .pretty_print_header(&self.highlighter, &self.languages[&snippet.language])
                    .unwrap()
                    .join(""),
                text: snippet.get_header(),
                code: snippet.code,
                index: snippet.index,
            })
            .collect();
        search(search_snippets)?;
        Ok(())
    }
}

fn search(input: Vec<SearchSnippet>) -> Result<(), Error> {
    let options = SkimOptionsBuilder::default()
        .height(Some("100%"))
        .preview(Some(""))
        .preview_window(Some("up:60%"))
        .multi(true)
        .reverse(true)
        .build()
        .unwrap();

    let (tx_item, rx_item): (SkimItemSender, SkimItemReceiver) = unbounded();
    for item in input.into_iter() {
        let _ = tx_item.send(Arc::new(item));
    }
    drop(tx_item); // so that skim could know when to stop waiting for more items.

    let selected_items = Skim::run_with(&options, Some(rx_item))
        .map(|out| out.selected_items)
        .unwrap_or_else(Vec::new);
    for item in &selected_items {
        print!("{}{}", item.output(), "\n");
    }
    Ok(())
}
