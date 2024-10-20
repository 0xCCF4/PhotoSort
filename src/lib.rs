#![doc = include_str!("../README.md")]

use crate::analysis::name_formatters::{FileType, NameFormatterInvocationInfo};
use action::ActionMode;
use anyhow::{anyhow, Result};
use chrono::NaiveDateTime;
use lazy_static::lazy_static;
use log::{debug, error, info, trace, warn};
use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::path::PathBuf;
use std::str::FromStr;

pub mod action;
pub mod analysis;
pub mod name;

/// `AnalysisType` is an enumeration that defines the different types of analysis that can be performed on a file.
///
/// # Variants
///
/// * `OnlyExif` - Represents the action of analyzing a file based only on its Exif data.
/// * `OnlyName` - Represents the action of analyzing a file based only on its name.
/// * `ExifThenName` - Represents the action of analyzing a file based first on its Exif data, then on its name if the Exif data is not sufficient.
/// * `NameThenExif` - Represents the action of analyzing a file based first on its name, then on its Exif data if the name is not sufficient.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum AnalysisType {
    OnlyExif,
    OnlyName,
    ExifThenName,
    NameThenExif,
}
/// Implementation of the `FromStr` trait for `AnalysisType`.
///
/// This allows a string to be parsed into the `AnalysisType` enum.
///
/// # Arguments
///
/// * `s` - A string slice that should be parsed into an `AnalysisType`.
///
/// # Returns
///
/// * `Result<Self, Self::Err>` - Returns `Ok(AnalysisType)` if the string could be parsed into an `AnalysisType`, `Err(anyhow::Error)` otherwise.
impl FromStr for AnalysisType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "only_exif" => Ok(AnalysisType::OnlyExif),
            "exif" => Ok(AnalysisType::OnlyExif),
            "only_name" => Ok(AnalysisType::OnlyName),
            "name" => Ok(AnalysisType::OnlyName),
            "exif_then_name" => Ok(AnalysisType::ExifThenName),
            "exif_name" => Ok(AnalysisType::ExifThenName),
            "name_then_exif" => Ok(AnalysisType::NameThenExif),
            "name_exif" => Ok(AnalysisType::NameThenExif),
            _ => Err(anyhow::anyhow!("Invalid analysis type")),
        }
    }
}

/// `AnalyzerSettings` is a struct that holds the settings for an `Analyzer`.
///
/// # Fields
/// * `analysis_type` - An `AnalysisType` that specifies the type of analysis to perform on a file.
/// * `source_dirs` - A vector of `Path` references that represent the source directories to analyze.
/// * `target_dir` - A `Path` reference that represents the target directory for the analysis results.
/// * `recursive_source` - A boolean that indicates whether to analyze source directories recursively.
/// * `file_format` - A string that represents the target format of the files to analyze.
/// * `nodate_file_format` - A string that represent the target format of files with no date.
/// * `unknown_file_format` - An optional string that represents the target format of files not matching the list of extensions
/// * `date_format` - A string that represents the format of the dates in the files to analyze.
/// * `extensions` - A vector of strings that represent the file extensions to consider during analysis.
/// * `action_type` - An `ActionMode` that specifies the type of action to perform on a file after analysis.
/// * `mkdir` - A boolean that indicates whether to create the target directory if it does not exist.
#[derive(Debug, Clone)]
pub struct AnalyzerSettings {
    pub analysis_type: AnalysisType,
    pub source_dirs: Vec<PathBuf>,
    pub target_dir: PathBuf,
    pub recursive_source: bool,
    pub file_format: String,
    pub nodate_file_format: String,
    pub unknown_file_format: Option<String>,
    pub date_format: String,
    pub extensions: Vec<String>,
    #[cfg(feature = "video")]
    pub video_extensions: Vec<String>,
    pub action_type: ActionMode,
    pub mkdir: bool,
}

lazy_static! {
    static ref RE_DETECT_NAME_FORMAT_COMMAND: regex::Regex = regex::Regex::new(
        r"\{([^\}]*)\}" // finds { ... } blocks
    ).expect("Failed to compile regex");

    static ref RE_COMMAND_SPLIT: regex::Regex = regex::Regex::new(
        r"^(([^:]*):)?(.*)$" // splits command into modifiers:command
    ).expect("Failed to compile regex");
}

/// `Analyzer` is a struct that represents an analyzer for files.
///
/// # Fields
///
/// * `name_transformers` - A list of `NameTransformer` objects that are used to transform the names of files during analysis.
/// * `name_formatters` - A list of `NameFormatter` objects that are used to generate the new names of files after analysis.
/// * `settings` - An `AnalyzerSettings` object that holds the settings for the `Analyzer`.
pub struct Analyzer {
    name_transformers: Vec<Box<dyn analysis::filename2date::FileNameToDateTransformer>>,
    name_formatters: Vec<Box<dyn analysis::name_formatters::NameFormatter>>,
    settings: AnalyzerSettings,
}

/// Implementation of methods for the `Analyzer` struct.
///
/// # Methods
///
/// * [`new`](#method.new) - Creates a new `Analyzer` with the given settings.
/// * [`add_transformer`](#method.add_transformer) - Adds a name transformer to the `Analyzer`.
/// * [`analyze_name`](#method.analyze_name) - Analyzes the name of a file.
/// * [`analyze_exif`](#method.analyze_exif) - Analyzes the Exif data of a file.
/// * [`analyze`](#method.analyze) - Analyzes a file based on the `Analyzer`'s settings.
/// * [`compose_file_name`](#method.compose_file_name) - Composes a file name based on the given date, name, and duplicate counter.
/// * [`do_file_action`](#method.do_file_action) - Performs the file action specified in the `Analyzer`'s settings on a file.
/// * [`is_valid_extension`](#method.is_valid_extension) - Checks if a file has a valid extension.
/// * [`rename_files_in_folder`](#method.rename_files_in_folder) - Renames files in a folder based on the `Analyzer`'s settings.
/// * [`run`](#method.run) - Runs the `Analyzer`, renaming files in the source directories based on the `Analyzer`'s settings.
impl Analyzer {
    /// Creates a new `Analyzer` with the given settings.
    ///
    /// # Arguments
    ///
    /// * `settings` - An `AnalyzerSettings` object that holds the settings for the `Analyzer`.
    ///
    /// # Returns
    ///
    /// * `Result<Analyzer>` - Returns `Ok(Analyzer)` if the `Analyzer` could be created successfully, `Err(anyhow::Error)` otherwise.
    ///
    /// # Errors
    ///
    /// * If the target directory does not exist.
    /// * If a source directory does not exist.
    /// * If an error occurs while getting the standard name transformers.
    pub fn new(settings: AnalyzerSettings) -> Result<Analyzer> {
        let analyzer = Analyzer {
            name_transformers: Vec::default(),
            name_formatters: Vec::default(),
            settings,
        };

        if !analyzer.settings.target_dir.exists() {
            return Err(anyhow!("Target directory does not exist"));
        }
        for source in &analyzer.settings.source_dirs {
            if !source.exists() {
                return Err(anyhow!("Source directory {:?} does not exist", source));
            }
        }

        Ok(analyzer)
    }

    /// Adds a name transformer to the `Analyzer`.
    ///
    /// # Arguments
    /// * `transformer` - A `NameTransformer` object that is used to transform the names of files during analysis.
    pub fn add_transformer<T: 'static + analysis::filename2date::FileNameToDateTransformer>(
        &mut self,
        transformer: T,
    ) {
        self.name_transformers.push(Box::new(transformer));
    }

    /// Adds a name formatter to the `Analyzer`.
    ///
    /// # Arguments
    /// * `formatter` - A `NameFormatter` object that is used to generate the new names of files after analysis.
    pub fn add_formatter<T: 'static + analysis::name_formatters::NameFormatter>(
        &mut self,
        formatter: T,
    ) {
        self.name_formatters.push(Box::new(formatter));
    }

    fn analyze_name(&self, name: &str) -> Result<(Option<NaiveDateTime>, String)> {
        let result = analysis::get_name_time(name, &self.name_transformers)?;
        match result {
            Some((time, name)) => Ok((Some(time), name)),
            None => Ok((None, name.to_string())),
        }
    }

    fn analyze_photo_exif(&self, file: &File) -> Result<Option<NaiveDateTime>> {
        let exif_time = analysis::exif2date::get_exif_time(file)?;
        Ok(exif_time)
    }

    #[cfg(feature = "video")]
    fn analyze_video_metadata(&self, path: &PathBuf) -> Result<Option<NaiveDateTime>> {
        let video_time = analysis::video2date::get_video_time(path)?;
        Ok(video_time)
    }

    fn analyze_exif(&self, path: &PathBuf) -> Result<Option<NaiveDateTime>> {
        #[cfg(feature = "video")]
        let video = self.is_valid_video_extension(path.extension())?;
        let photo = self.is_valid_photo_extension(path.extension())?;

        #[cfg(feature = "video")]
        {
            if video && photo {
                return Err(anyhow::anyhow!("File has both photo and video extensions. Do not include the same extension in both settings"));
            }
        }

        if photo {
            let file = File::open(path)?;
            return self.analyze_photo_exif(&file);
        }
        #[cfg(feature = "video")]
        if video {
            return self.analyze_video_metadata(path);
        }

        Err(anyhow::anyhow!("File extension is not valid"))
    }

    /// Analyzes a file for a date based on the `Analyzer`'s settings.
    ///
    /// # Arguments
    /// * `path` - A `PathBuf` that represents the path of the file to analyze.
    ///
    /// # Returns
    /// * `Result<(Option<NaiveDateTime>, String)>` - Returns a tuple containing an `Option<NaiveDateTime>` and a `String`.
    ///   The `Option<NaiveDateTime>` represents the date and time extracted from the file, if any.
    ///   The `String` represents the transformed name of the file.
    ///
    /// # Errors
    /// This function will return an error if:
    /// * The file name cannot be retrieved or is invalid.
    /// * The file cannot be opened.
    /// * An error occurs during the analysis of the file's Exif data or name.
    pub fn analyze(&self, path: &PathBuf) -> Result<(Option<NaiveDateTime>, String)> {
        let name = path
            .file_name()
            .ok_or(anyhow::anyhow!("No file name"))?
            .to_str()
            .ok_or(anyhow::anyhow!("Invalid file name"))?;

        let valid_extension = self
            .is_valid_extension(path.extension())
            .unwrap_or_else(|err| {
                warn!("Error checking file extension: {}", err);
                false
            });
        if !valid_extension {
            warn!("Skipping file with invalid extension: {:?}", path);
            return Err(anyhow::anyhow!("Invalid file extension"));
        }

        Ok(match self.settings.analysis_type {
            AnalysisType::OnlyExif => {
                let exif_result = self
                    .analyze_exif(path)
                    .map_err(|e| anyhow!("Error analyzing Exif data: {}", e))?;
                let name_result = self.analyze_name(name);

                match name_result {
                    Ok((_, name)) => (exif_result, name),
                    Err(_err) => (exif_result, name.to_string()),
                }
            }
            AnalysisType::OnlyName => self.analyze_name(name)?,
            AnalysisType::ExifThenName => {
                let exif_result = self.analyze_exif(path);
                let exif_result = match exif_result {
                    Err(e) => {
                        warn!("Error analyzing Exif data: {} for {:?}", e, path);
                        info!("Falling back to name analysis");
                        None
                    }
                    Ok(date) => date,
                };
                let name_result = self.analyze_name(name);

                match exif_result {
                    Some(date) => match name_result {
                        Ok((_, name)) => (Some(date), name),
                        Err(_err) => (Some(date), name.to_string()),
                    },
                    None => name_result?,
                }
            }
            AnalysisType::NameThenExif => {
                let name_result = self.analyze_name(name)?;
                if name_result.0.is_none() {
                    (self.analyze_exif(path)?, name_result.1)
                } else {
                    name_result
                }
            }
        })
    }

    /// Replaces {name}, {date}, ... in a format with actual values
    fn replace_filepath_parts<'a, 'b>(
        &self,
        format_string: &'b str,
        info: &'a NameFormatterInvocationInfo,
    ) -> Result<String> {
        let detect_commands = RE_DETECT_NAME_FORMAT_COMMAND.captures_iter(format_string);

        #[derive(Debug)]
        enum FormatString<'a> {
            Literal(String),
            Command(&'a str, String),
        }
        impl FormatString<'_> {
            fn formatted_string(self) -> String {
                match self {
                    FormatString::Literal(str) => str,
                    FormatString::Command(_, str) => str,
                }
            }
        }

        let mut final_string: Vec<FormatString<'b>> = Vec::new();

        let mut current_string_index = 0;
        for capture in detect_commands {
            let match_all = capture.get(0).expect("Capture group 0 should always exist");
            let start = match_all.start();
            let end = match_all.end();

            if start > current_string_index {
                final_string.push(FormatString::Literal(
                    format_string[current_string_index..start].to_string(),
                ));
            }

            // {prefix:cmd}
            // let full_match_string = match_all.as_str();
            // prefix:cmd
            let inner_command_string = capture
                .get(1)
                .expect("Capture group 2 should always exist")
                .as_str();

            let inner_command_capture = RE_COMMAND_SPLIT
                .captures(inner_command_string)
                .expect("Should always match");

            // prefix
            let command_modifier = inner_command_capture
                .get(2)
                .map(|x| x.as_str())
                .unwrap_or("");
            // cmd
            let actual_command = inner_command_capture
                .get(3)
                .map(|x| x.as_str())
                .unwrap_or("");

            let mut found_command = false;

            for formatter in &self.name_formatters {
                if let Some(matched) = formatter.argument_template().captures(actual_command) {
                    let mut command_substitution = match formatter.replacement_text(matched, info) {
                        Ok(replaced_text) => replaced_text,
                        Err(err) => {
                            return Err(anyhow!("Failed to format the file name with the given format string: {:?}. Got error: {{{}}}", actual_command, err));
                        }
                    };

                    if !command_substitution.is_empty() && !command_modifier.is_empty() {
                        // prefix_substitution
                        command_substitution =
                            format!("{}{}", command_modifier, command_substitution);
                    }
                    found_command = true;
                    final_string.push(FormatString::Command(
                        inner_command_string,
                        command_substitution,
                    ));
                    break;
                }
            }

            if !found_command {
                return Err(anyhow!("Failed to format file name with the given format string. There exists no formatter for the format command: {{{}}}", actual_command));
            }

            current_string_index = end;
        }
        if format_string.len() > current_string_index {
            final_string.push(FormatString::Literal(
                format_string[current_string_index..].to_string(),
            ));
        }

        trace!("Parsed format string {:?} to", format_string);
        for part in &final_string {
            match part {
                FormatString::Literal(str) => trace!(" - Literal: {:?}", str),
                FormatString::Command(cmd, str) => trace!(" - Command: {:?}\t{:?}", cmd, str),
            }
        }

        Ok(final_string
            .into_iter()
            .map(|v| v.formatted_string())
            .collect::<Vec<_>>()
            .join(""))
    }

    /// Performs the file action specified in the `Analyzer`'s settings on a file.
    ///
    /// # Arguments
    ///
    /// * `path` - A `PathBuf` that represents the path of the file to perform the action on.
    /// * `is_unknown_file` - If set to true no date parsing will happen, instead the unknown file format string will be used to format the file.
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Returns `Ok(())` if the file action could be performed successfully, `Err(anyhow::Error)` otherwise.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// * The analysis of the file fails.
    /// * An IO error occurs while analyzing the date
    /// * An IO error occurs while doing the file action
    /// * If `unknown_file_switch` is set to `true` but no unknown file format string was set.
    pub fn run_file(&self, path: &PathBuf, is_unknown_file: bool) -> Result<()> {
        let (date, cleaned_name) = if !is_unknown_file {
            let (date, cleaned_name) = self.analyze(path).map_err(|err| {
                error!("Error extracting date: {}", err);
                err
            })?;
            let cleaned_name = name::clean_image_name(cleaned_name.as_str());

            debug!(
                "Analysis results: Date: {:?}, Cleaned name: {:?}",
                date, cleaned_name
            );

            if date.is_none() {
                warn!("No date was derived for file {:?}.", path);
            }

            (date, cleaned_name)
        } else {
            (
                None,
                path.with_extension("")
                    .file_name()
                    .ok_or(anyhow::anyhow!("No file name"))?
                    .to_str()
                    .ok_or(anyhow::anyhow!("Invalid file name"))?
                    .to_string(),
            )
        };

        let date_string = match date {
            None => "NODATE".to_string(),
            Some(date) => date.format(&self.settings.date_format).to_string(),
        };

        let mut ftype = FileType::None;
        if self.is_valid_photo_extension(path.extension())? {
            ftype = FileType::Image;
        }
        #[cfg(feature = "video")]
        if self.is_valid_video_extension(path.extension())? {
            ftype = FileType::Video
        }

        let mut file_name_info = NameFormatterInvocationInfo {
            date: &date,
            date_string: &date_string,
            date_default_format: &self.settings.date_format,
            file_type: &ftype,
            cleaned_name: &cleaned_name,
            duplicate_counter: None,
            extension: path
                .extension()
                .map(|ext| ext.to_string_lossy().to_string())
                .unwrap_or("".to_owned()),
        };

        let new_file_path = |file_name_info: &NameFormatterInvocationInfo| -> Result<PathBuf> {
            let format_string = if is_unknown_file {
                self.settings
                    .unknown_file_format
                    .as_ref()
                    .ok_or(anyhow!("No unknown format string specified"))?
                    .as_str()
            } else if date.is_some() {
                self.settings.file_format.as_str()
            } else {
                self.settings.nodate_file_format.as_str()
            };

            let path_split: Vec<_> = format_string
                .split('/')
                .map(|component| self.replace_filepath_parts(component, file_name_info))
                .collect();
            for entry in &path_split {
                if let Err(err) = entry {
                    return Err(anyhow!("Failed to format filename: {}", err));
                }
            }
            let path_split = path_split.into_iter().map(Result::unwrap);

            let mut target_path = self.settings.target_dir.clone();
            for path_component in path_split {
                let component = path_component.replace("/", "").replace("\\", "");
                if component != ".." {
                    target_path.push(component);
                }
            }
            Ok(target_path)
        };

        let mut new_path = new_file_path(&file_name_info)?;
        let mut dup_counter = 0;

        while new_path.exists() {
            debug!("Target file already exists: {:?}", new_path);
            dup_counter += 1;
            file_name_info.duplicate_counter = Some(dup_counter);
            new_path = new_file_path(&file_name_info)?;
        }

        if dup_counter > 0 {
            info!("De-duplicated target file: {:?}", new_path);
        }

        action::file_action(
            path,
            &new_path,
            &self.settings.action_type,
            self.settings.mkdir,
        )?;
        Ok(())
    }

    fn is_valid_photo_extension(&self, ext: Option<&OsStr>) -> Result<bool> {
        match ext {
            None => Ok(false),
            Some(ext) => {
                let ext = ext
                    .to_str()
                    .ok_or(anyhow::anyhow!("Invalid file extension"))?
                    .to_lowercase();
                Ok(self
                    .settings
                    .extensions
                    .iter()
                    .any(|valid_ext| ext == valid_ext.as_str()))
            }
        }
    }

    #[cfg(feature = "video")]
    fn is_valid_video_extension(&self, ext: Option<&OsStr>) -> Result<bool> {
        match ext {
            None => Ok(false),
            Some(ext) => {
                let ext = ext
                    .to_str()
                    .ok_or(anyhow::anyhow!("Invalid file extension"))?
                    .to_lowercase();
                Ok(self
                    .settings
                    .video_extensions
                    .iter()
                    .any(|valid_ext| ext == valid_ext.as_str()))
            }
        }
    }

    fn is_valid_extension(&self, ext: Option<&OsStr>) -> Result<bool> {
        let valid_photo = self.is_valid_photo_extension(ext)?;
        #[cfg(feature = "video")]
        let valid_video = self.is_valid_video_extension(ext)?;
        #[cfg(not(feature = "video"))]
        let valid_video = false;
        Ok(valid_photo || valid_video)
    }

    /// Executes the analyzer on a folder based on the `Analyzer`'s settings.
    ///
    /// # Arguments
    ///
    /// * `root_source` - A `Path` reference that represents the root source directory to rename files in.
    /// * `target_path` - A `Path` reference that represents the target directory for the renamed files.
    /// * `recursive` - A boolean that indicates whether to rename files in subdirectories of the root source directory.
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Returns `Ok(())` if the files could be renamed successfully, `Err(anyhow::Error)` otherwise.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// * The analysis of the file fails.
    /// * An IO error occurs while analyzing the date
    /// * An IO error occurs while doing the file action
    pub fn run_files_in_folder(
        &self,
        root_source: &PathBuf,
        _target_path: &PathBuf,
        recursive: bool,
    ) -> Result<()> {
        let entries = fs::read_dir(root_source)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                if recursive {
                    debug!("Processing subfolder: {:?}", path);
                    self.run_files_in_folder(&path, _target_path, recursive)?;
                }
            } else {
                let valid_ext = self.is_valid_extension(path.extension());
                match valid_ext {
                    Ok(false) => match self.settings.unknown_file_format {
                        None => {
                            debug!(
                                "Skipping file because extension is not in the list: {:?}",
                                path
                            );
                            continue;
                        }
                        Some(_) => {
                            debug!("Processing unknown file: {:?}", path);
                            let result = self.run_file(&path, true);
                            if let Err(err) = result {
                                error!("Error renaming file: {}", err);
                            }
                        }
                    },
                    Ok(true) => {
                        debug!("Processing file: {:?}", path);
                        let result = self.run_file(&path, false);
                        if let Err(err) = result {
                            error!("Error renaming file: {}", err);
                        }
                    }
                    Err(err) => {
                        warn!("Error checking file extension: {}", err);
                        continue;
                    }
                }
            }
        }
        Ok(())
    }

    /// Runs the `Analyzer`, renaming files in the source directories based on the `Analyzer`'s settings.
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Returns `Ok(())` if the files could be renamed successfully, `Err(anyhow::Error)` otherwise.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// * The analysis of the file fails.
    /// * An IO error occurs while analyzing the date
    /// * An IO error occurs while doing the file action
    pub fn run(&self) -> Result<()> {
        for source in &self.settings.source_dirs {
            info!("Processing source folder: {:?}", source);
            let result = self.run_files_in_folder(
                source,
                &self.settings.target_dir,
                self.settings.recursive_source,
            );
            if let Err(err) = result {
                eprintln!("Error processing folder: {}", err);
            }
        }
        Ok(())
    }
}
