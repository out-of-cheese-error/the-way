use std::borrow::Cow;
use std::sync::Arc;

use anyhow::Error;
use skim::prelude::{unbounded, SkimOptionsBuilder};
use skim::{AnsiString, ItemPreview, Skim, SkimItem, SkimItemReceiver, SkimItemSender};

use crate::utils::copy_to_clipboard;

#[derive(Debug)]
pub(crate) struct SearchSnippet {
    pub(crate) index: usize,
    pub(crate) text: String,
    pub(crate) code_highlight: String,
    pub(crate) code: String,
}

impl<'a> SkimItem for SearchSnippet {
    fn display(&self) -> Cow<AnsiString> {
        Cow::Owned(AnsiString::parse(&self.text))
    }

    fn text(&self) -> Cow<str> {
        Cow::Borrowed(&self.text)
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

pub(crate) fn search(input: Vec<SearchSnippet>) -> Result<(), Error> {
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
