use crate::analysis::name_formatters::{NameFormatter, NameFormatterInvocationInfo};
use anyhow::Result;
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref DATE_FORMAT: regex::Regex =
        regex::Regex::new(r"^(date|d)(\?(.+))?$").expect("Failed to compile regex");
}

/// Formats a date format command {date} to a date string.
#[derive(Debug, Default)]
pub struct FormatDate {}

impl NameFormatter for FormatDate {
    fn argument_template(&self) -> &Regex {
        &DATE_FORMAT
    }
    fn replacement_text(
        &self,
        capture: regex::Captures<'_>,
        invocation_info: &NameFormatterInvocationInfo,
    ) -> Result<String> {
        let format_string = capture
            .get(3)
            .map_or(invocation_info.date_default_format, |m| m.as_str());
        Ok(invocation_info.date.map_or("NODATE".to_string(), |x| {
            x.format(format_string).to_string()
        }))
    }
}
