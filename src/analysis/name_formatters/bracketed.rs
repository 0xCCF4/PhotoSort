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
            None | Some("?seq") => invocation_info.bracket_info.as_ref().map(|b|b.sequence_number.to_string()),
            //Some("?name")|Some("?name_first") => invocation_info.bracket_info.as_ref().map(|b|b.first_file_name.to_string()),
            //Some("?name_last") => invocation_info.bracket_info.as_ref().map(|b|b.last_file_name.to_string()),
            //Some("?len")|Some("?length") => invocation_info.bracket_info.as_ref().map(|b|b.sequence_length.to_string()),
            Some(x) => {
                return Err(anyhow!("Unknown format argument {x:?} for the {{bracket}} format specifier. Possible values are \"seq/name/name_first/name_last/len/length\""))
            }
            //None => {
            //    return Err(anyhow!("No format argument for the {{bracket}} format string was specified. Possible values are \"seq/name/name_first/name_last/len/length\""));
            //}
        };

        match formatted {
            Some(formatted) => Ok(formatted),
            None => {
                warn!("Tried to format a non bracketed file using the {{bracket}} format string.");
                Ok("".to_string())
            }
        }
    }
}
