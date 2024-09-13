pub mod exif2date;
pub mod filename2date;
pub mod name_formatters;
#[cfg(feature = "video")]
pub mod video2date;

use anyhow::Result;
use chrono::NaiveDateTime;

use crate::analysis::filename2date::FileNameToDateTransformer;

/// This function tries to retrieve a file creation date and time from a file name.
///
/// The function accepts a list of `NameTransformer` instances that are used to match and transform the file name into a datetime.
/// Each `NameTransformer` instance contains a regular expression and a transformation function.
/// A list of standard `NameTransformer` instances can be generated using the `NameTransformer::get_standard_name_parsers` function.
///
/// # Arguments
///
/// * `name` - A reference to a string that represents the file name.
/// * `parsers` - A reference to a vector of `NameTransformer` instances.
///
/// # Returns
///
/// * `Result<Option<(NaiveDateTime, String)>>` - A `Result` that, if `Ok`, contains an `Option` with a tuple.
///   The first element of the tuple is the date and time from the file name.
///   The second element of the tuple is the file name with the matched part removed.
///   If the date and time could not be retrieved, the `Option` will be `None`.
///   If an error occurred during the process, the `Result` will be `Err`.
///
/// # Errors
///
/// This function will return an error if:
///
/// * A transformation function failed and errors
pub fn get_name_time(
    name: &str,
    parsers: &Vec<Box<dyn FileNameToDateTransformer>>,
) -> Result<Option<(NaiveDateTime, String)>> {
    for transformer in parsers {
        let result = transformer.try_transform_name(name);
        match result {
            Ok(Some((dt, name))) => return Ok(Some((dt, name))),
            Ok(None) => continue,
            Err(e) => {
                log::error!("Error: {:?}", e);
                continue;
            }
        }
    }

    Ok(None)
}
