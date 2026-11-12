use log::trace;
use regex::Regex;
use std::sync::LazyLock;

/// Matches image names with optional trailing separators, ending with a file extension.
///
/// The first group contains the name without trailing `-`/`_` characters and without extension.
static RE_IMAGE_NAME: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(.*?)[-_]*?\.([A-Za-z0-9]+)$").expect("Failed to compile regex")
});

/// Prefixes that are removed from the start of an image name.
const NAME_PREFIXES: [&str; 12] = [
    "MOV", "VID", "mov", "vid", "IMG", "img", "NO_DATE", "NODATE", "no_date", "nodate", "-", "_",
];

/// Removes all leading prefixes (`IMG`, `VID`, `MOV`, `NO_DATE`, ...) and separators from a name.
///
/// A prefix is only removed if it is not directly followed by a letter. Otherwise it is the
/// beginning of a word, for example `movie`, `video` or `IMGP1234`, and not a prefix.
fn strip_name_prefixes(mut name: &str) -> &str {
    'outer: loop {
        for prefix in NAME_PREFIXES {
            if let Some(rest) = name.strip_prefix(prefix) {
                let is_separator = prefix == "-" || prefix == "_";
                if is_separator || !rest.chars().next().is_some_and(char::is_alphabetic) {
                    name = rest;
                    continue 'outer;
                }
            }
        }
        return name;
    }
}

/// Matches and removes file extensions.
///
/// This regex matches any sequence of characters followed by a period and one or more alphabetic characters,
/// effectively matching file extensions for removal.
static RE_REMOVE_EXT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\.[A-Za-z]+$").expect("Failed to compile regex"));

/// Matches and removes `NO_DATE` or `no_date` from image names.
///
/// This regex matches `NO_DATE` or `no_date`, with or without an underscore, for removal from image names.
static RE_REMOVE_NODATE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(NO_?DATE|no_?date)").expect("Failed to compile regex"));

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
        Some(caps) => {
            if let (Some(cap_name), Some(_cap_ext)) = (caps.get(1), caps.get(2)) {
                RE_REMOVE_NODATE
                    .replace(strip_name_prefixes(cap_name.as_str()), "")
                    .to_string()
            } else {
                RE_REMOVE_NODATE
                    .replace(RE_REMOVE_EXT.replace(name, "").as_ref(), "")
                    .to_string()
            }
        }
    };
    trace!("Cleaned name: {name:?} -> {result:?}");
    result
}
