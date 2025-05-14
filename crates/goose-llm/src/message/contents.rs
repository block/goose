use std::{iter::FromIterator, ops::Deref};

use crate::message::MessageContent;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

/// Holds the heterogeneous fragments that make up one chat message.
///
/// *   Up to two items are stored inline on the stack.
/// *   Falls back to a heap allocation only when necessary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct Contents(SmallVec<[MessageContent; 2]>);

impl Contents {
    /*----------------------------------------------------------
     * 1-line ergonomic helpers
     *---------------------------------------------------------*/

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, MessageContent> {
        self.0.iter_mut()
    }

    pub fn push(&mut self, item: impl Into<MessageContent>) {
        self.0.push(item.into());
    }

    pub fn texts(&self) -> impl Iterator<Item = &str> {
        self.0.iter().filter_map(|c| c.as_text())
    }

    pub fn concat_text_str(&self) -> String {
        self.texts().collect::<Vec<_>>().join("\n")
    }

    /// Returns `true` if *any* item satisfies the predicate.
    pub fn any_is<P>(&self, pred: P) -> bool
    where
        P: FnMut(&MessageContent) -> bool,
    {
        self.iter().any(pred)
    }

    /// Returns `true` if *every* item satisfies the predicate.
    pub fn all_are<P>(&self, pred: P) -> bool
    where
        P: FnMut(&MessageContent) -> bool,
    {
        self.iter().all(pred)
    }
}

impl From<Vec<MessageContent>> for Contents {
    fn from(v: Vec<MessageContent>) -> Self {
        Contents(SmallVec::from_vec(v))
    }
}

impl FromIterator<MessageContent> for Contents {
    fn from_iter<I: IntoIterator<Item = MessageContent>>(iter: I) -> Self {
        Contents(SmallVec::from_iter(iter))
    }
}

/*--------------------------------------------------------------
 * Allow &message.content to behave like a slice of fragments.
 *-------------------------------------------------------------*/
impl Deref for Contents {
    type Target = [MessageContent];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// — Register the contents type with UniFFI, converting to/from Vec<MessageContent> —
// We need to do this because UniFFI’s FFI layer supports only primitive buffers (here Vec<u8>),
uniffi::custom_type!(Contents, Vec<MessageContent>, {
    lower: |contents: &Contents| {
        contents.0.to_vec()
    },
    try_lift: |contents: Vec<MessageContent>| {
        Ok(Contents::from(contents))
    },
});
