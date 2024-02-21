use std::{env};
use std::path::{Path};
use photo_sort::{action, AnalysisType, Analyzer};
use clap::{arg, Parser};
use log::{debug, LevelFilter};

/// A simple command line tool to sort photos by date.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Arguments {
    /// The source directory to read the photos from.
    #[arg(short, long, num_args = 1.., required = true)]
    source_dir: Vec<String>,
    /// The target directory to write the sorted photos to.
    #[arg(short, long)]
    target_dir: String,
    /// Whether to search the source directories recursively.
    /// If the flag is not set only immediate children of the source directories are considered.
    #[arg(short, long, default_value = "false")]
    recursive: bool,
    /// Date format string to use for the target directory.
    /// The format string is passed to the `chrono` crate's `format` method.
    #[arg(long, default_value = "%Y%m%d-%H%M%S")]
    date_format: String,
    /// The target file format. {:date} is replaced with the date and {:name} with the original file name.
    /// {:dup} is replaced with a number if the file already exists.
    /// {:date} is replaced with the date and {:name} with the original file name.
    /// {:?dup} is replaced with _{:dup} if the file already exists.
    #[arg(short, long, default_value = "IMG_{:date}_{:name}{:?dup}")]
    file_format: String,
    /// A comma separated list of file extensions to include in the analysis.
    #[arg(short, long, default_value = "jpg,jpeg,png", value_delimiter = ',', num_args = 0..)]
    extensions: Vec<String>,
    /// The sorting mode, possible values are name_then_exif, exif_then_name, only_name, only_exif.
    /// Name analysis tries to extract the date from the file name, Exif analysis tries to extract the date from the EXIF data.
    #[arg(short, long, default_value = "exif_then_name")]
    analysis_mode: AnalysisType,
    /// The action mode, possible values are move, copy, hardlink, relative_symlink, absolute_symlink.
    /// Move will move the files, Copy will copy the files, Hardlink (alias: hard) will create hardlinks, RelativeSymlink (alias: relsym) will create relative symlinks, AbsoluteSymlink (alias: abssym) will create absolute symlinks.
    #[arg(short, long, default_value = "move")]
    move_mode: action::ActionMode,
    /// Dry-run
    /// If set, the tool will not move any files but only print the actions it would take.
    #[arg(short = 'n', long, default_value = "false")]
    dry_run: bool,
    /// Don't remove source files. If flag set, the source files will be copied instead of moved.
    #[arg(short, long, default_value = "false")]
    copy: bool,
    /// Be verbose, if set, the tool will print more information about the actions it takes. Setting the RUST_LOG env var overrides this flag.
    #[arg(short, long, default_value = "false")]
    verbose: bool,
    /// Debug, if set, the tool will print debug information (including debug implies setting verbose). Setting the RUST_LOG env var overrides this flag.
    #[arg(short, long, default_value = "false")]
    debug: bool,
}

fn main() {
    let args = Arguments::parse();

    if !env::vars_os().any(|(key, _)| key == "RUST_LOG") {
        let mut log_level = LevelFilter::Warn;
        if args.verbose {
            log_level = LevelFilter::Info;
        }
        if args.debug {
            log_level = LevelFilter::Debug;
        }
        env::set_var("RUST_LOG", format!("photo_sort={}", log_level));
    }

    env_logger::init();

    debug!("Initializing program");

    let result = Analyzer::new(photo_sort::AnalyzerSettings {
        use_standard_transformers: true,
        analysis_type: args.analysis_mode,
        source_dirs: args.source_dir.iter().map(|x| Path::new(x)).collect(),
        target_dir: Path::new(args.target_dir.as_str()),
        recursive_source: args.recursive,
        file_format: args.file_format.clone(),
        date_format: args.date_format.clone(),
        extensions: args.extensions.clone(),
        action_type: if args.dry_run { action::ActionMode::DryRun } else { args.move_mode },
    });
    let analyzer;

    match result {
        Ok(a) => {
            debug!("Program initialized");
            analyzer = a;
        },
        Err(e) => {
            eprintln!("{:?}", e);
            return;
        }
    }

    debug!("Running program");

    match analyzer.run() {
        Ok(_) => {
            debug!("Program finished");
        },
        Err(e) => {
            eprintln!("{:?}", e);
        }
    }
}