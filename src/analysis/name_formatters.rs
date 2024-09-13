use anyhow::Result;
use chrono::NaiveDateTime;
use regex::Regex;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FileType {
    Image,
    Video,
    None,
}

#[derive(Debug)]
pub struct NameFormatterInvocationInfo<'a> {
    pub date: &'a Option<NaiveDateTime>,
    pub date_string: &'a str,
    pub date_default_format: &'a str,
    pub file_type: &'a FileType,
    pub cleaned_name: &'a str,
    pub duplicate_counter: Option<u32>,
}

pub trait NameFormatter {
    fn argument_template(&self) -> &Regex;
    fn replacement_text(
        &self,
        matched: regex::Captures,
        invocation_info: &NameFormatterInvocationInfo,
    ) -> Result<String>;
}

mod date;
pub use date::*;
mod name;
pub use name::*;
mod duplicate;
pub use duplicate::*;
mod file_type;
pub use file_type::*;
