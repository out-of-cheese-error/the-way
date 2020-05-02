//! Sled database related code
use std::path::Path;

use anyhow::Error;
use chrono::{DateTime, Utc};

use crate::errors::LostTheWay;
use crate::the_way::{snippet::Snippet, TheWay};
use crate::utils;

/// If key exists, add value to existing values - join with a semicolon
fn merge_index(_key: &[u8], old_indices: Option<&[u8]>, new_index: &[u8]) -> Option<Vec<u8>> {
    let mut ret = old_indices.map_or_else(Vec::new, |old| old.to_vec());
    if !ret.is_empty() {
        ret.extend_from_slice(&[utils::SEMICOLON]);
    }
    ret.extend_from_slice(new_index);
    Some(ret)
}

impl TheWay {
    /// Gets the `sled` database with all the-way info.
    /// Makes a new one the first time round
    pub(crate) fn get_db(db_dir: &Path) -> Result<sled::Db, Error> {
        Ok(sled::open(db_dir)?)
    }

    /// Merge function for appending items to an existing key, uses semicolons
    pub(crate) fn set_merge(&self) -> Result<(), Error> {
        self.language_tree()?.set_merge_operator(merge_index);
        self.tag_tree()?.set_merge_operator(merge_index);
        Ok(())
    }

    /// Gets snippet index: snippet tree
    fn snippets_tree(&self) -> Result<sled::Tree, Error> {
        Ok(self.db.open_tree("snippets")?)
    }

    pub(crate) fn get_current_snippet_index(&self) -> Result<usize, Error> {
        match self.db.get("snippet_index")? {
            Some(index) => Ok(std::str::from_utf8(&index)?.parse::<usize>()?),
            None => Ok(0),
        }
    }

    /// resets snippet_index to 0
    pub(crate) fn reset_index(&self) -> Result<(), Error> {
        self.db.insert("snippet_index", 0.to_string().as_bytes())?;
        Ok(())
    }

    /// Get the language: snippet_indices tree
    fn language_tree(&self) -> Result<sled::Tree, Error> {
        Ok(self.db.open_tree("language_to_snippet")?)
    }

    /// Get the tag: snippet indices tree
    fn tag_tree(&self) -> Result<sled::Tree, Error> {
        Ok(self.db.open_tree("tag_to_snippet")?)
    }

    /// Map a snippet index to a language
    pub(crate) fn add_to_language(
        &mut self,
        language_key: &[u8],
        index_key: &[u8],
    ) -> Result<(), Error> {
        self.language_tree()?
            .merge(language_key.to_vec(), index_key.to_vec())?;
        Ok(())
    }

    /// Retrieve a snippet by index
    pub(crate) fn get_snippet(&self, index: usize) -> Result<Snippet, Error> {
        let index_key = index.to_string();
        let index_key = index_key.as_bytes();
        Ok(Snippet::from_bytes(
            &self
                .snippets_tree()?
                .get(index_key)?
                .ok_or(LostTheWay::SnippetNotFound { index })?,
        )?)
    }

    /// Retrieve snippets at indices
    pub(crate) fn get_snippets(&self, indices: &[usize]) -> Result<Vec<Snippet>, Error> {
        indices.iter().map(|i| self.get_snippet(*i)).collect()
    }

    /// List snippets in date range
    pub(crate) fn list_snippets_in_date_range(
        &self,
        from_date: DateTime<Utc>,
        to_date: DateTime<Utc>,
    ) -> Result<Vec<Snippet>, Error> {
        Ok(self
            .snippets_tree()?
            .iter()
            .map(|item| {
                item.map_err(|_| {
                    LostTheWay::OutOfCheeseError {
                        message: "sled PageCache Error".into(),
                    }
                    .into()
                })
                .and_then(|(_, snippet)| Snippet::from_bytes(&snippet))
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .filter(|snippet| snippet.in_date_range(from_date, to_date))
            .collect())
    }

    // TODO: think about how deletions should affect snippet indices
    pub(crate) fn increment_snippet_index(&mut self) -> Result<(), Error> {
        self.db.insert(
            "snippet_index",
            (self.get_current_snippet_index()? + 1)
                .to_string()
                .as_bytes(),
        )?;
        Ok(())
    }

    /// Add a snippet index to each of the tags it's associated with
    pub(crate) fn add_to_tags(&mut self, tags: &[String], index_key: &[u8]) -> Result<(), Error> {
        for tag in tags {
            let tag_key = tag.as_bytes();
            self.tag_tree()?
                .merge(tag_key.to_vec(), index_key.to_vec())?;
        }
        Ok(())
    }

    /// Add a serialized snippet to the snippets tree
    pub(crate) fn add_to_snippet(
        &self,
        index_key: &[u8],
        snippet_bytes: &[u8],
    ) -> Result<(), Error> {
        self.snippets_tree()?.insert(index_key, snippet_bytes)?;
        Ok(())
    }

    /// Add a snippet (with all attached data) to the database and change metadata accordingly
    pub(crate) fn add_snippet(&mut self, snippet: &Snippet) -> Result<usize, Error> {
        let language_key = snippet.language.as_bytes();
        let index_key = snippet.index.to_string();
        let index_key = index_key.as_bytes();
        self.add_to_snippet(index_key, &snippet.to_bytes()?)?;
        self.add_to_language(language_key, index_key)?;
        self.add_to_tags(&snippet.tags, index_key)?;
        Ok(snippet.index)
    }

    /// Delete a language (if no snippets are written in it)
    fn delete_language(&mut self, language_key: &[u8]) -> Result<(), Error> {
        self.language_tree()?.remove(language_key)?;
        Ok(())
    }

    /// Delete a snippet index from the language tree
    fn delete_from_language(&mut self, language_key: &[u8], index: usize) -> Result<(), Error> {
        let language = utils::u8_to_str(language_key)?;
        let new_indices: Vec<_> = utils::split_indices_usize(
            &self
                .language_tree()?
                .get(language_key)?
                .ok_or(LostTheWay::LanguageNotFound { language })?,
        )?
        .into_iter()
        .filter(|index_i| *index_i != index)
        .collect();
        if new_indices.is_empty() {
            self.delete_language(language_key)?;
        } else {
            self.language_tree()?.insert(
                language_key.to_vec(),
                utils::make_indices_string(&new_indices)?,
            )?;
        }
        Ok(())
    }

    /// Delete a snippet index from the tag tree
    fn delete_from_tag(
        &mut self,
        tag_key: &[u8],
        index: usize,
        batch: &mut sled::Batch,
    ) -> Result<(), Error> {
        let tag = utils::u8_to_str(tag_key)?;
        let new_indices: Vec<_> = utils::split_indices_usize(
            &self
                .tag_tree()?
                .get(tag_key)?
                .ok_or(LostTheWay::TagNotFound { tag })?,
        )?
        .into_iter()
        .filter(|index_i| *index_i != index)
        .collect();
        if new_indices.is_empty() {
            batch.remove(tag_key);
        } else {
            batch.insert(tag_key.to_vec(), utils::make_indices_string(&new_indices)?);
        }
        Ok(())
    }

    /// Delete snippet from database
    pub(crate) fn delete_snippet(&mut self, index: usize) -> Result<Snippet, Error> {
        let snippet = self.delete_from_snippets_tree(index)?;
        self.delete_from_trees(&snippet, index)?;
        Ok(snippet)
    }

    /// Delete snippet from language and tag trees
    fn delete_from_trees(&mut self, snippet: &Snippet, index: usize) -> Result<(), Error> {
        let language_key = snippet.language.as_bytes();
        self.delete_from_language(language_key, index)?;
        let mut tag_batch = sled::Batch::default();
        for tag in &snippet.tags {
            self.delete_from_tag(tag.as_bytes(), index, &mut tag_batch)?;
        }
        self.tag_tree()?.apply_batch(tag_batch)?;
        Ok(())
    }

    /// Delete snippet from the snippet tree
    fn delete_from_snippets_tree(&mut self, index: usize) -> Result<Snippet, Error> {
        let index_key = index.to_string();
        let index_key = index_key.as_bytes();
        Ok(Snippet::from_bytes(
            &self
                .snippets_tree()?
                .remove(index_key)?
                .ok_or(LostTheWay::SnippetNotFound { index })?,
        )?)
    }

    /// Retrieve snippets written in a given language
    pub(crate) fn get_language_snippets(&self, language: &str) -> Result<Vec<usize>, Error> {
        utils::split_indices_usize(
            &self
                .language_tree()?
                .get(&language.to_ascii_lowercase().as_bytes())?
                .ok_or(LostTheWay::LanguageNotFound {
                    language: language.to_owned(),
                })?,
        )
    }

    /// Retrieve snippets associated with a given tag
    pub(crate) fn get_tag_snippets(&self, tag: &str) -> Result<Vec<usize>, Error> {
        utils::split_indices_usize(&self.tag_tree()?.get(tag.as_bytes())?.ok_or(
            LostTheWay::TagNotFound {
                tag: tag.to_owned(),
            },
        )?)
    }
}
