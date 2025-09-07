use crate::analysis::name_formatters::{NameFormatter, NameFormatterInvocationInfo};
use anyhow::Result;
use regex::Regex;
use std::sync::LazyLock;

static NAME_FORMAT: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"^(name|n)$").expect("Failed to compile regex"));

/// Formats a name format command {name} to a name string.
#[derive(Debug, Default)]
pub struct FormatName {}

impl NameFormatter for FormatName {
    fn argument_template(&self) -> &Regex {
        &NAME_FORMAT
    }
    fn replacement_text(
        &self,
        _capture: regex::Captures<'_>,
        invocation_info: &NameFormatterInvocationInfo,
    ) -> Result<String> {
        Ok(invocation_info.cleaned_name.to_string())
    }
}
