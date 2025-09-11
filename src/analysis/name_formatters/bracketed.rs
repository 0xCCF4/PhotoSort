use crate::analysis::name_formatters::{NameFormatter, NameFormatterInvocationInfo};
use anyhow::{anyhow, Result};
use log::warn;
use regex::Regex;
use std::sync::LazyLock;

static BRACKET_TYPE_FORMAT: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"^(bracket|bracketed|b)(\?([^\n]*))?$").expect("Failed to compile regex")
});

/// Formats a name format command {name} to a name string.
#[derive(Debug, Default)]
pub struct BracketedFormat {}

impl NameFormatter for BracketedFormat {
    fn argument_template(&self) -> &Regex {
        &BRACKET_TYPE_FORMAT
    }
    fn replacement_text(
        &self,
        capture: regex::Captures<'_>,
        invocation_info: &NameFormatterInvocationInfo,
    ) -> Result<String> {
        let bracket_argument = capture.get(2).map(|m| m.as_str());

        let formatted = match bracket_argument {
            Some("?seq"|"?index") => invocation_info.bracket_info.as_ref().map(|b|b.sequence_number.to_string()),
            Some("?name"|"?name_first"|"?first") => invocation_info.bracket_info.as_ref().map(|b|b.first.with_extension("").file_name().map(|x|x.to_string_lossy().to_string()).unwrap_or_default()),
            Some("?name_last"|"?last") => invocation_info.bracket_info.as_ref().map(|b|b.last.with_extension("").file_name().map(|x|x.to_string_lossy().to_string()).unwrap_or_default()),
            Some("?len"|"?length") => invocation_info.bracket_info.as_ref().map(|b|b.sequence_length.to_string()),
            Some("?group"|"?grp"|"?num") => invocation_info.bracket_info.as_ref().map(|b|b.group_index.to_string()),
            Some(x) => {
                return Err(anyhow!("Unknown format argument {x:?} for the {{bracket}} format specifier. Possible values are \"seq/name/name_first/name_last/len/length/group/num\""))
            }
            None => {
                return Err(anyhow!("No format argument for the {{bracket}} format string was specified. Possible values are \"seq/name/name_first/name_last/len/length/group/num\""));
            }
        };

        if let Some(formatted) = formatted {
            Ok(formatted)
        } else {
            warn!("Tried to format a non bracketed file using the {{bracket}} format string.");
            Ok(String::new())
        }
    }
}
