#![doc = include_str!("../README.md")]

use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use aho_corasick::AhoCorasick;
use chrono::NaiveDateTime;
use anyhow::{anyhow, Result};
use log::{debug, error, info, warn};
use action::ActionMode;

pub mod analysis;
pub mod name;
pub mod action;


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
            _ => Err(anyhow::anyhow!("Invalid analysis type"))
        }
    }
}

/// `AnalyzerSettings` is a struct that holds the settings for an `Analyzer`.
///
/// # Fields
///
/// * `use_standard_transformers` - A boolean that indicates whether to use standard name transformers.
/// * `analysis_type` - An `AnalysisType` that specifies the type of analysis to perform on a file.
/// * `source_dirs` - A vector of `Path` references that represent the source directories to analyze.
/// * `target_dir` - A `Path` reference that represents the target directory for the analysis results.
/// * `recursive_source` - A boolean that indicates whether to analyze source directories recursively.
/// * `file_format` - A string that represents the format of the files to analyze.
/// * `date_format` - A string that represents the format of the dates in the files to analyze.
/// * `extensions` - A vector of strings that represent the file extensions to consider during analysis.
/// * `action_type` - An `ActionMode` that specifies the type of action to perform on a file after analysis.
#[derive(Debug, Clone)]
pub struct AnalyzerSettings<'a> {
    pub use_standard_transformers: bool,
    pub analysis_type: AnalysisType,
    pub source_dirs: Vec<&'a Path>,
    pub target_dir: &'a Path,
    pub recursive_source: bool,
    pub file_format: String,
    pub date_format: String,
    pub extensions: Vec<String>,
    pub action_type: ActionMode,
}

/// `Analyzer` is a struct that represents an analyzer for files.
///
/// # Fields
///
/// * `name_transformers` - A vector of `NameTransformer` objects that are used to transform the names of files during analysis.
/// * `settings` - An `AnalyzerSettings` object that holds the settings for the `Analyzer`.
pub struct Analyzer<'a> {
    name_transformers: Vec<analysis::NameTransformer>,
    settings: AnalyzerSettings<'a>,
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
impl Analyzer<'_> {
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
            name_transformers: if settings.use_standard_transformers && (settings.analysis_type != AnalysisType::OnlyExif) {
                debug!("Using standard name transformers");
                analysis::NameTransformer::get_standard_name_parsers()?
            } else {
                debug!("Not using standard name transformers");
                Vec::new()
            },
            settings
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
    ///
    /// * `transformer` - A `NameTransformer` object that is used to transform the names of files during analysis.
    pub fn add_transformer(&mut self, transformer: analysis::NameTransformer) {
        self.name_transformers.push(transformer);
    }

    fn analyze_name(&self, name: &str) -> Result<(Option<NaiveDateTime>, String)> {
        let result = analysis::get_name_time(name, &self.name_transformers)?;
        match result {
            Some((time, name)) => Ok((Some(time), name)),
            None => Ok((None, name.to_string()))
        }
    }

    fn analyze_exif(&self, file: &File) -> Result<Option<NaiveDateTime>> {
        let exif_time = analysis::get_exif_time(file)?;
        Ok(exif_time)
    }

    /// Analyzes a file based on the `Analyzer`'s settings.
    ///
    /// # Arguments
    ///
    /// * `path` - A `PathBuf` that represents the path of the file to analyze.
    ///
    /// # Returns
    ///
    /// * `Result<(Option<NaiveDateTime>, String)>` - Returns a tuple containing an `Option<NaiveDateTime>` and a `String`.
    ///   The `Option<NaiveDateTime>` represents the date and time extracted from the file, if any.
    ///   The `String` represents the transformed name of the file.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// * The file name cannot be retrieved or is invalid.
    /// * The file cannot be opened.
    /// * An error occurs during the analysis of the file's Exif data or name.
    pub fn analyze(&self, path: &PathBuf) -> Result<(Option<NaiveDateTime>, String)> {
        let name = path.file_name().ok_or(anyhow::anyhow!("No file name"))?.to_str().ok_or(anyhow::anyhow!("Invalid file name"))?;

        Ok(match self.settings.analysis_type {
            AnalysisType::OnlyExif => {
                let file = File::open(&path)?;
                let exif_result = self.analyze_exif(&file)?;
                let name_result = self.analyze_name(name);

                match name_result {
                    Ok((_, name)) => (exif_result, name),
                    Err(_err) => (exif_result, name.to_string())
                }
            },
            AnalysisType::OnlyName => {
                self.analyze_name(name)?
            },
            AnalysisType::ExifThenName => {
                let file = File::open(&path)?;
                let exif_result = self.analyze_exif(&file)?;
                let name_result = self.analyze_name(name);

                match exif_result {
                    Some(date) => match name_result {
                        Ok((_, name)) => (Some(date), name),
                        Err(_err) => (Some(date), name.to_string())
                    },
                    None => name_result?
                }
            },
            AnalysisType::NameThenExif => {
                let name_result = self.analyze_name(name)?;
                if name_result.0.is_none() {
                    let file = File::open(&path)?;
                    (self.analyze_exif(&file)?, name_result.1)
                } else {
                    name_result
                }
            }
        })
    }

    fn compose_file_name(&self, date: &str, name: &str, dup: &str) -> Result<String> {
        let patterns = &["{:date}", "{:name}", "{:dup}", "{:?dup}"];
        let dup2 = if dup.len() > 0 { "_".to_string() + dup } else { "".to_string() };
        let replacements = &[date, name, dup, dup2.as_str()];
        let ac = AhoCorasick::new(patterns)?;
        // Replace all patterns at once to avoid being influenced by e.g. the file name
        Ok(ac.replace_all(self.settings.file_format.as_str(), replacements))
    }

    /// Performs the file action specified in the `Analyzer`'s settings on a file.
    ///
    /// # Arguments
    ///
    /// * `path` - A `PathBuf` that represents the path of the file to perform the action on.
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
    pub fn do_file_action(&self, path: &PathBuf) -> Result<()> {
        let (date, cleaned_name) = self.analyze(path)?;
        let cleaned_name = name::clean_image_name(cleaned_name.as_str());

        debug!("Analysis results: Date: {:?}, Cleaned name: {:?}", date, cleaned_name);

        let date_string = match date {
            None => "NODATE".to_string(),
            Some(date) => date.format(&self.settings.date_format).to_string()
        };

        let mut new_path = self.settings.target_dir.join(Path::new("")
            .with_file_name(self.compose_file_name(date_string.as_str(), cleaned_name.as_str(), "")?)
            .with_extension(path.extension()
                .ok_or(anyhow::anyhow!("No file extension"))?));
        let mut dup_counter = 0;

        while new_path.exists() {
            debug!("Target file already exists: {:?}", new_path);
            dup_counter += 1;
            new_path = self.settings.target_dir.join(Path::new("")
                .with_file_name(self.compose_file_name(date_string.as_str(), cleaned_name.as_str(), &dup_counter.to_string())?)
                .with_extension(path.extension()
                    .ok_or(anyhow::anyhow!("No file extension"))?));
        }

        if dup_counter > 0 {
            info!("De-deplicated target file: {:?}", new_path);
        }

        action::file_action(path, &new_path, &self.settings.action_type)?;
        Ok(())
    }

    fn is_valid_extension(&self, ext: Option<&OsStr>) -> Result<bool> {
        match ext {
            None => Ok(false),
            Some(ext) => {
                let ext = ext.to_str().ok_or(anyhow::anyhow!("Invalid file extension"))?.to_lowercase();
                Ok(self.settings.extensions.iter().any(|valid_ext| ext == valid_ext.as_str()))
            }
        }
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
    pub fn process_files_in_folder(&self, root_source: &Path, target_path: &Path, recursive: bool) -> Result<()> {
        let entries = fs::read_dir(root_source)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                if recursive {
                    debug!("Processing subfolder: {:?}", path);
                    self.process_files_in_folder(&path, target_path, recursive)?;
                }
            } else {
                let valid_ext = self.is_valid_extension(path.extension());
                match valid_ext {
                    Ok(false) => continue,
                    Ok(true) => {
                        debug!("Processing file: {:?}", path);
                        let result = self.do_file_action(&path);
                        if let Err(err) = result {
                            error!("Error renaming file: {}", err);
                        }
                    },
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
            let result = self.process_files_in_folder(
                source,
                self.settings.target_dir,
                self.settings.recursive_source);
            if let Err(err) = result {
                eprintln!("Error processing folder: {}", err);
            }
        }
        Ok(())
    }
}
