use crate::analysis::name_formatters::{NameFormatter, NameFormatterInvocationInfo};
use anyhow::{anyhow, Result};
use regex::Regex;
use std::sync::LazyLock;

static DATE_FORMAT: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"^(ext|extension)(\?(.+))?$").expect("Failed to compile regex")
});

/// Formats a date format command {date} to a date string.
#[derive(Debug, Default)]
pub struct FormatExtension {}

impl NameFormatter for FormatExtension {
    fn argument_template(&self) -> &Regex {
        &DATE_FORMAT
    }
    fn replacement_text(
        &self,
        capture: regex::Captures<'_>,
        invocation_info: &NameFormatterInvocationInfo,
    ) -> Result<String> {
        let option = capture.get(3).map_or("copy", |m| m.as_str());

        let extension = match option {
            "lower" | "low" | "lowercase" | "l" => invocation_info.extension.to_lowercase(),
            "upper" | "up" | "uppercase" | "u" => invocation_info.extension.to_uppercase(),
            "copy" | "normal" | "standard" | "pass" | "p" => invocation_info.extension.clone(),
            _ => return Err(anyhow!("Unknown extension format")),
        };

        Ok(extension)
    }
}
