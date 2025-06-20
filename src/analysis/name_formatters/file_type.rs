use crate::analysis::name_formatters::{FileType, NameFormatter, NameFormatterInvocationInfo};
use anyhow::Result;
use regex::Regex;
use std::sync::LazyLock;

static FILE_TYPE_FORMAT: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"^(ftype|type|t)(\?(([^,\n]*)(,([^,\n]*))?))?$")
        .expect("Failed to compile regex")
});

/// Formats a file type format command {ftype} to a file type string.
#[derive(Debug, Default)]
pub struct FormatFileType {}

impl NameFormatter for FormatFileType {
    fn argument_template(&self) -> &Regex {
        &FILE_TYPE_FORMAT
    }
    fn replacement_text(
        &self,
        capture: regex::Captures<'_>,
        invocation_info: &NameFormatterInvocationInfo,
    ) -> Result<String> {
        let regex_image_name = capture.get(4).map(|m| m.as_str());
        let regex_video_name = capture.get(6).map(|m| m.as_str());

        let file_type = match invocation_info.file_type {
            FileType::Image => regex_image_name.unwrap_or("IMG"),
            FileType::Video => regex_video_name.unwrap_or("MOV"),
            FileType::None => "",
        };

        Ok(file_type.to_string())
    }
}
