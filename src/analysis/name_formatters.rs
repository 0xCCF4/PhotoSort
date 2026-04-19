use anyhow::Result;
use chrono::NaiveDateTime;
use regex::Regex;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FileType {
    Image,
    Video,
    None,
}

#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub enum BracketingFormattingPriority {
    #[default]
    First,
    Last,
    Current,
}

impl FromStr for BracketingFormattingPriority {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "first" | "f" => Ok(BracketingFormattingPriority::First),
            "last" | "l" => Ok(BracketingFormattingPriority::Last),
            "current" | "c" => Ok(BracketingFormattingPriority::Current),
            _ => Err(anyhow::anyhow!(
                "Invalid bracketing formatting priority: {s}. Possible values are first/last/current"
            )),
        }
    }
}

#[derive(Debug, Clone)]
pub struct NameFormatterInvocationInfo<'a> {
    pub date: Option<NaiveDateTime>,
    pub date_string: String,
    pub date_default_format: String,
    pub file_type: FileType,
    pub cleaned_name: String,
    pub duplicate_counter: Option<u32>,
    pub extension: String,
    pub bracket_info: Option<&'a BracketInfo<'a>>,
    pub original_name: String,     // name without extension
    pub original_filename: String, // original_name + extension
    pub bracketing_formatting: BracketingFormattingPriority,
}

impl<'a> NameFormatterInvocationInfo<'a> {
    /// Converts the invocation info to have a different lifetime.
    ///
    /// # Safety
    /// This function is safe to call, since the struct contains no references, so the lifetime is effectively 'static.
    ///
    /// # Errors
    /// If the invocation info contains a reference, the function will return an error, since the lifetime cannot be changed.
    #[allow(clippy::result_large_err)]
    pub fn to_lifetime<'target>(
        self,
    ) -> Result<NameFormatterInvocationInfo<'target>, NameFormatterInvocationInfo<'a>> {
        if self.bracket_info.is_none() {
            unsafe {
                Ok(std::mem::transmute::<
                    NameFormatterInvocationInfo<'_>,
                    NameFormatterInvocationInfo<'_>,
                >(self))
            } // safe because there are no references in the struct, so the lifetime is effectively 'static
        } else {
            Err(self)
        }
    }
}

#[derive(Debug, Clone)]
pub struct BracketInfo<'a> {
    pub sequence_number: u32,
    pub first: PathBuf,
    pub last: PathBuf,
    pub sequence_length: usize,
    pub group_index: usize,

    pub analysis_first: Option<Arc<NameFormatterInvocationInfo<'a>>>,
    pub analysis_last: Option<Arc<NameFormatterInvocationInfo<'a>>>,
}

pub trait NameFormatter {
    fn argument_template(&self) -> &Regex;

    /// Computes the target text for a given format string command (matched by the regex).
    ///
    /// # Arguments
    /// * `matched` - The regex captures from the matched format string.
    /// * `invocation_info` - Information about the file to format
    ///
    /// # Returns
    /// A `Result<String>` containing the formatted text or an error if the formatting fails.
    ///
    /// # Errors
    /// If the formatting fails, an `anyhow::Error` is returned.
    fn replacement_text(
        &self,
        matched: regex::Captures,
        invocation_info: &NameFormatterInvocationInfo,
    ) -> Result<String>;
}

mod date;
pub use date::*;
mod name;
pub use name::*;
mod duplicate;
pub use duplicate::*;
mod file_type;
pub use file_type::*;
mod extension;
pub use extension::*;
mod bracketed;
pub use bracketed::*;
mod original_name;
pub use original_name::*;
