use crate::analysis::name_formatters::{NameFormatter, NameFormatterInvocationInfo};
use anyhow::Result;
use regex::Regex;
use std::sync::LazyLock;

static NAME_FORMAT_ON: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"^(original_name|on)$").expect("Failed to compile regex"));

/// Formats a date format command {date} to a date string.
#[derive(Debug, Default)]
pub struct FormatOriginalName {}

impl NameFormatter for FormatOriginalName {
    fn argument_template(&self) -> &Regex {
        &NAME_FORMAT_ON
    }
    fn replacement_text(
        &self,
        _capture: regex::Captures<'_>,
        invocation_info: &NameFormatterInvocationInfo,
    ) -> Result<String> {
        Ok(invocation_info.original_name.to_string())
    }
}

static NAME_FORMAT_OFN: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"^(original_filename|ofn)$").expect("Failed to compile regex")
});

/// Formats a date format command {date} to a date string.
#[derive(Debug, Default)]
pub struct FormatOriginalFileName {}

impl NameFormatter for FormatOriginalFileName {
    fn argument_template(&self) -> &Regex {
        &NAME_FORMAT_OFN
    }
    fn replacement_text(
        &self,
        _capture: regex::Captures<'_>,
        invocation_info: &NameFormatterInvocationInfo,
    ) -> Result<String> {
        Ok(invocation_info.original_filename.to_string())
    }
}
