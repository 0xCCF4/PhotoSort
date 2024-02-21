use lazy_static::lazy_static;
use log::debug;
use regex::Regex;

// Regular expressions for matching and cleaning image names.
lazy_static! {
    /// Matches image names with optional prefixes and suffixes.
    ///
    /// This regex matches image names that optionally start with "IMG", "img", "NO_DATE", or "no_date",
    /// followed by any characters, and ending with a file extension.
    static ref RE_IMAGE_NAME: Regex = Regex::new(r"^((IMG|img|NO_?DATE|no_?date)?[-_]*)*(.*?)[-_]*?\.([A-Za-z]+)$").expect("Failed to compile regex");

    /// Matches and removes file extensions.
    ///
    /// This regex matches any sequence of characters followed by a period and one or more alphabetic characters,
    /// effectively matching file extensions for removal.
    static ref RE_REMOVE_EXT: Regex = Regex::new(r"\.[A-Za-z]+$").expect("Failed to compile regex");

    /// Matches and removes "NO_DATE" or "no_date" from image names.
    ///
    /// This regex matches "NO_DATE" or "no_date", with or without an underscore, for removal from image names.
    static ref RE_REMOVE_NODATE: Regex = Regex::new(r"(NO_?DATE|no_?date)").expect("Failed to compile regex");
}

/// Cleans an image name by removing certain prefixes, suffixes, and file extensions.
///
/// # Arguments
///
/// * `name` - A string slice that holds the name of the image.
///
/// # Returns
///
/// * `String` - The cleaned image name.
pub fn clean_image_name(name: &str) -> String {
    let caps = RE_IMAGE_NAME.captures(name);
    let result = match caps {
        None => RE_REMOVE_EXT.replace(name, "").to_string(),
        Some(caps) => if let (Some(cap_name), Some(_cap_ext)) = (caps.get(3), caps.get(4)) {
            RE_REMOVE_NODATE.replace(cap_name.as_str(), "").to_string()
        } else {
            RE_REMOVE_NODATE.replace(RE_REMOVE_EXT.replace(name, "").as_ref(), "").to_string()
        }
    };
    debug!("Cleaned name: {:?} -> {:?}", name, result);
    result
}