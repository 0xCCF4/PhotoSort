use crate::analysis::name_formatters::{NameFormatter, NameFormatterInvocationInfo};
use anyhow::Result;
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref DUPLICATE_FORMAT: regex::Regex =
        regex::Regex::new(r"^(dup|duplicate)$").expect("Failed to compile regex");
}

/// Formats a duplicate format command {dup} to a duplicate counter string.
#[derive(Debug, Default)]
pub struct FormatDuplicate {}

impl NameFormatter for FormatDuplicate {
    fn argument_template(&self) -> &Regex {
        &DUPLICATE_FORMAT
    }
    fn replacement_text(
        &self,
        _capture: regex::Captures<'_>,
        invocation_info: &NameFormatterInvocationInfo,
    ) -> Result<String> {
        let duplicate_string = invocation_info
            .duplicate_counter
            .map_or("".to_string(), |c| c.to_string());
        Ok(duplicate_string)
    }
}
