use clap::{arg, Parser};
use log::{debug, LevelFilter};
use photo_sort::{action, AnalysisType, Analyzer};
use std::env;
use std::path::PathBuf;

/// A simple command line tool to sort photos by date.
#[derive(Parser, Debug)]
#[command(
    version,
    about,
    long_about = "A tool to rename and sort photos/videos by its EXIF date/metadata. It tries to extract the date
from the EXIF data or file name and renames the image file according to a given
format string."
)]
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
    /// Date format string to use as default date format.
    /// See [https://docs.rs/chrono/latest/chrono/format/strftime/index.html] for more information.
    #[arg(long, default_value = "%Y%m%d-%H%M%S")]
    date_format: String,
    /// The target file format. Everything outside a {...} block is copied as is. The target file format may contain "/" to
    /// indicate that the file should be placed in a subdirectory. Use the `--mkdir` flag to create the subdirectories.
    /// `{name}` is replaced with a filename without the date part.
    /// `{dup}` is replaced with a number if a file with the target name already exists.
    /// `{date}` is replaced with the date string, formatted according to the date_format parameter.
    /// `{date?format}` is replaced with the date string, formatted according to the "format" parameter. See [https://docs.rs/chrono/latest/chrono/format/strftime/index.html] for more information.
    /// `{type}` is replaced with MOV or IMG.
    /// `{type?img,vid}` is replaced with `img` if the file is an image, `vid` if the file is a video. Note that, when using other types than IMG or MOV,
    /// and rerunning the program again, the custom type will be seen as part of the file name.
    /// `{ext?upper/lower/copy}` is replaced with the original file extension. If `?upper` or `?lower` is specified, the extension will be made lower/upper case.
    ///      leaving out `?...` or using `copy` copies the original file extension.
    /// Commands of the form {label:cmd} are replaced by {cmd}; if the replacement string is not empty then a prefix of "label" is added.
    /// This might be useful to add separators only if there is e.g. a {dup} part.
    #[arg(short, long, default_value = "{type}{_:date}{-:name}{-:dup}.{ext}")]
    file_format: String,
    /// The target format for files that have no date. The `analysis_mode` allows specifying which method
    /// should be used to derive a date for a file. See the `file_format` option for an extensive description of possible
    /// format values. If not specified, uses the same format as for normal files.
    #[arg(long = "nodate")]
    nodate_file_format: Option<String>,
    /// The target file format for files that do not match the specified extensions list. If not present
    /// files that do not match the extension list are ignored, hence not moved, copied etc. See the `file_format` for an extensive description
    /// of possible format values.
    #[arg(long = "unknown")]
    unknown_file_format: Option<String>,
    /// If the file format contains a "/", indicating that the file should be placed in a subdirectory,
    /// the mkdir flag controls if the tool is allowed to create non-existing subdirectories. No folder is created in dry-run mode.
    #[arg(long, default_value = "false", alias = "mkdirs")]
    mkdir: bool,
    /// A comma separated list of file extensions to include in the analysis.
    #[arg(short, long, default_value = "jpg,jpeg,png,tiff,heif,heic,avif,webp", value_delimiter = ',', num_args = 0..)]
    extensions: Vec<String>,
    #[cfg(feature = "video")]
    /// A comma separated list of video extensions to include in the analysis.
    #[arg(long, default_value = "mp4,mov,avi", value_delimiter = ',', num_args = 0..)]
    video_extensions: Vec<String>,
    /// The sorting mode, possible values are name_then_exif, exif_then_name, only_name, only_exif.
    /// Name analysis tries to extract the date from the file name, Exif analysis tries to extract the date from the EXIF data.
    #[arg(short, long, default_value = "exif_then_name")]
    analysis_mode: AnalysisType,
    /// The action mode, possible values are move, copy, hardlink, relative_symlink, absolute_symlink.
    /// Move will move the files, Copy will copy the files, Hardlink (alias: hard) will create hardlinks, RelativeSymlink (alias: relsym) will create relative symlinks, AbsoluteSymlink (alias: abssym) will create absolute symlinks.
    #[arg(short, long, default_value = "move")]
    move_mode: action::ActualAction,
    /// Dry-run
    /// If set, the tool will not move any files but only print the actions it would take.
    #[arg(short = 'n', long, default_value = "false")]
    dry_run: bool,
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
        analysis_type: args.analysis_mode,
        source_dirs: args.source_dir.iter().map(PathBuf::from).collect(),
        target_dir: PathBuf::from(args.target_dir.as_str()),
        recursive_source: args.recursive,
        file_format: args.file_format.clone(),
        nodate_file_format: args.nodate_file_format.unwrap_or(args.file_format.clone()),
        unknown_file_format: args.unknown_file_format,
        date_format: args.date_format.clone(),
        extensions: args.extensions.clone(),
        mkdir: args.mkdir,
        action_type: if args.dry_run {
            action::ActionMode::DryRun(args.move_mode)
        } else {
            action::ActionMode::Execute(args.move_mode)
        },
        #[cfg(feature = "video")]
        video_extensions: args.video_extensions.clone(),
    });
    let mut analyzer = match result {
        Ok(a) => {
            debug!("Program initialized");
            a
        }
        Err(e) => {
            eprintln!("{:?}", e);
            return;
        }
    };

    // add file name -> date parsers
    analyzer.add_transformer(photo_sort::analysis::filename2date::NaiveFileNameParser::default());

    // add date -> file name formatters
    analyzer.add_formatter(photo_sort::analysis::name_formatters::FormatName::default());
    analyzer.add_formatter(photo_sort::analysis::name_formatters::FormatDuplicate::default());
    analyzer.add_formatter(photo_sort::analysis::name_formatters::FormatDate::default());
    analyzer.add_formatter(photo_sort::analysis::name_formatters::FormatFileType::default());
    analyzer.add_formatter(photo_sort::analysis::name_formatters::FormatExtension::default());

    debug!("Running program");

    match analyzer.run() {
        Ok(_) => {
            debug!("Program finished");
        }
        Err(e) => {
            eprintln!("{:?}", e);
        }
    }
}
