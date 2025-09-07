use crate::analysis::name_formatters::{NameFormatter, NameFormatterInvocationInfo};
use anyhow::Result;
use regex::Regex;
use std::sync::LazyLock;

static DUPLICATE_FORMAT: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"^(dup|duplicate)$").expect("Failed to compile regex"));

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
            .map_or(String::new(), |c| c.to_string());
        Ok(duplicate_string)
    }
}
