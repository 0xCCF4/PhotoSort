use crate::analysis::name_formatters::{NameFormatter, NameFormatterInvocationInfo};
use anyhow::{anyhow, Result};
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref DATE_FORMAT: regex::Regex =
        regex::Regex::new(r"^(ext|extension)(\?(.+))?$").expect("Failed to compile regex");
}

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
        let option = capture
            .get(3)
            .map_or("copy", |m| m.as_str());
        
        let extension = match option {
            "lower"|"low"|"lowercase"|"l" => invocation_info.extension.to_lowercase(),
            "upper"|"up"|"uppercase"|"u" => invocation_info.extension.to_uppercase(),
            "copy"|"normal"|"standard"|"pass"|"p" => invocation_info.extension.to_owned(),
            _ => return Err(anyhow!("Unknown extension format")),
        };
        
        Ok(extension)
    }
}
