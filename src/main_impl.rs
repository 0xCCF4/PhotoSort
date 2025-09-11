use crate::indicatif_log_bridge::LogWrapper;
use chrono::Utc;
use clap::{arg, Parser};
use fern::colors::{Color, ColoredLevelConfig};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use log::{debug, error, info, trace, LevelFilter};
use photo_sort::analysis::bracketed::get_bracketing_info;
use photo_sort::analysis::name_formatters::BracketInfo;
use photo_sort::{action, find_files_in_source, AnalysisType, Analyzer, BracketEXIFInformation};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use threadpool::ThreadPool;

/// A simple command line tool to sort photos by date.
#[derive(Parser, Debug)]
#[command(
    version,
    about,
    long_about = "A tool to rename and sort photos/videos by its EXIF date/metadata. It tries to extract the date
from the EXIF data or file name and renames the image file according to a given
format string."
)]
#[allow(clippy::struct_excessive_bools)]
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
    /// See <https://docs.rs/chrono/latest/chrono/format/strftime/index.html> for more information.
    #[arg(long, default_value = "%Y%m%d-%H%M%S")]
    date_format: String,
    /// The target file format. Everything outside a {...} block is copied as is. The target file format may contain "/" to
    /// indicate that the file should be placed in a subdirectory. Use the `--mkdir` flag to create the subdirectories.
    /// `{name}` is replaced with a filename without the date part.
    /// `{dup}` is replaced with a number if a file with the target name already exists.
    /// `{date}` is replaced with the date string, formatted according to the `date_format` parameter.
    /// `{date?format}` is replaced with the date string, formatted according to the "format" parameter. See <https://docs.rs/chrono/latest/chrono/format/strftime/index.html> for more information.
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
    /// of possible format values. By using `--unknown others/{name}{.:ext}` all unknown files are moved to the subdirectory "others" relative
    /// to the target directory (specified by `--target-dir`).
    #[arg(long = "unknown")]
    unknown_file_format: Option<String>,
    /// The target file format for files that can be identified as bracketed photo set with different exposure levels. By using `--bracket` all
    /// files identified as bracketed are moved/renamed/copied with the specified format string instead of the default one specified by `--target-dir`.
    /// The `--bracket` argument provides the following additional format specifiers:
    /// `{bracket?num}` an increasing number, unique for each group of bracketed images,
    /// `{bracket?seq}` the index of the current photo in the sequence,
    /// `{bracket?len}` the length of the bracketing sequence,
    /// `{bracket?first}`/`{bracket?last}` the name of the first/last image in the sequence.
    /// Bracketed photos sequences are detected via manufacturer-specific EXIF information.
    /// Note that using the `--bracket` option requires each file to
    /// be analyzed using the EXIF analyzer, even if the Analysis type is set to Name-only.
    /// Currently only works for Sony's cameras. Feel free to open an issue requesting support for other vendors at <https://github.com/0xCCF4/PhotoSort/issues>.
    #[arg(long = "bracket", alias = "bracketed")]
    bracketed_file_format: Option<String>,
    /// If the file format contains a "/", indicating that the file should be placed in a subdirectory,
    /// the mkdir flag controls if the tool is allowed to create non-existing subdirectories. No folder is created in dry-run mode.
    #[arg(long, default_value = "false", alias = "mkdirs")]
    mkdir: bool,
    /// A comma separated list of file extensions to include in the analysis.
    #[arg(short, long, alias= "ext", default_value = "jpg,jpeg,png,tiff,heif,heic,avif,webp", value_delimiter = ',', num_args = 0..)]
    extensions: Vec<String>,
    #[cfg(feature = "video")]
    /// A comma separated list of video extensions to include in the analysis.
    #[arg(long, default_value = "mp4,mov,avi", value_delimiter = ',', num_args = 0..)]
    video_extensions: Vec<String>,
    /// The sorting mode, possible values are `name_then_exif`, `exif_then_name`, `only_name`, `only_exif`.
    /// Name analysis tries to extract the date from the file name, Exif analysis tries to extract the date from the EXIF data.
    #[arg(short, long, default_value = "exif_then_name")]
    analysis_mode: AnalysisType,
    /// The action mode, possible values are `move`, `copy`, `hardlink`, `relative_symlink`, `absolute_symlink`.
    /// `Move` will move the files, `Copy` will copy the files, `Hardlink` (alias: `hard`) will create hardlinks, `RelativeSymlink` (alias: `relsym`) will create relative symlinks, `AbsoluteSymlink` (alias: `abssym`) will create absolute symlinks.
    #[arg(short, long, default_value = "move")]
    move_mode: action::ActualAction,
    /// Dry-run
    /// If set, the tool will not move any files but only print the actions it would take.
    #[arg(short = 'n', long, default_value = "false")]
    dry_run: bool,
    /// Be verbose, if set, the tool will print more information about the actions it takes.
    #[arg(short, long, default_value = "false")]
    verbose: bool,
    /// Debug, if set, the tool will print debug information (including debug implies setting verbose).
    #[arg(short, long, default_value = "false", alias = "vv")]
    debug: bool,
    /// Logfile, if set, the tool will log its output to the specified file. Appending to the specified file if it already exists.
    #[arg(short, long = "log")]
    logfile: Option<String>,
    /// If set, suppresses the output of the tool to stdout/stderr. Only displaying error messages. Specifying a logfile at the same
    /// time will redirect the full output that would have been displayed to stdout/stderr to the logfile. Specifying `--debug` or `--verbose`
    /// plus `--quiet` without a logfile will result in an error.
    #[arg(short, long, default_value = "false")]
    quiet: bool,
    /// If set, display a progress bar while processing files.
    #[arg(short, long, default_value = "false")]
    progress: bool,
    /// If set, use multi-threading
    #[arg(long)]
    threads: Option<usize>,
}

fn setup_loggers<Q: AsRef<Path>>(
    general_log_level: LevelFilter,
    stdout_log_level: LevelFilter,
    file: Option<Q>,
    progress: Option<MultiProgress>,
) -> anyhow::Result<()> {
    let colors = ColoredLevelConfig::new().info(Color::Green);

    let mut config = fern::Dispatch::new().level(general_log_level);

    if let Some(file) = file {
        config = config.chain(
            fern::Dispatch::new()
                .format(move |out, message, record| {
                    if message.to_string().starts_with('[') {
                        out.finish(format_args!("{message}"));
                    } else {
                        out.finish(format_args!(
                            "[{} {} {}] {}",
                            Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                            record.level(),
                            record.target(),
                            message
                        ));
                    }
                })
                .chain(
                    fern::log_file(file)
                        .map_err(|err| anyhow::anyhow!("Failed to open log file: {:?}", err))?,
                ),
        );
    }

    let (max_level, log) = config
        .chain(
            fern::Dispatch::new()
                .format(move |out, message, record| {
                    if message.to_string().starts_with('[') {
                        out.finish(format_args!("{message}"));
                    } else {
                        out.finish(format_args!(
                            "[{} {} {}] {}",
                            Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                            colors.color(record.level()),
                            record.target(),
                            message
                        ));
                    }
                })
                .level(stdout_log_level)
                .chain(std::io::stdout()),
        )
        .into_log();

    if let Some(bar) = progress {
        LogWrapper::new(bar, log)
            .try_init()
            .expect("Failed to setup progressbar logging");

        log::set_max_level(max_level);
    } else {
        log::set_boxed_logger(log)?;
        log::set_max_level(max_level);
    }

    Ok(())
}

#[allow(clippy::too_many_lines)]
pub fn main() {
    let args = Arguments::parse();

    let log_level_general = {
        let mut log_level = LevelFilter::Warn;
        if args.verbose {
            log_level = LevelFilter::Info;
        }
        if args.debug {
            log_level = LevelFilter::Trace;
        }

        if args.quiet && args.logfile.is_none() {
            if args.debug || args.verbose {
                eprintln!("Error: Cannot use --debug/--verbose with --quiet. Maybe you wanted to specify a --logfile to log the full output to, while suppressing the STDOUT/STDERR output?");
                return;
            }

            log_level = LevelFilter::Error;
        }

        log_level
    };

    let console_log_level = if args.quiet {
        LevelFilter::Error
    } else {
        log_level_general
    };

    let multi = MultiProgress::new();
    let multi_clone = args.progress.then_some(multi.clone());

    if let Err(e) = setup_loggers(
        log_level_general,
        console_log_level,
        args.logfile,
        multi_clone,
    ) {
        eprintln!("Error starting application: {e:?}");
        return;
    }

    debug!("Initializing program");

    debug!("Video features enabled: {}", cfg!(feature = "video"));

    let bracket_mode = args.bracketed_file_format.is_some();
    let result = Analyzer::new(photo_sort::AnalyzerSettings {
        analysis_type: args.analysis_mode,
        source_dirs: args.source_dir.iter().map(PathBuf::from).collect(),
        target_dir: PathBuf::from(args.target_dir.as_str()),
        recursive_source: args.recursive,
        file_format: args.file_format.clone(),
        nodate_file_format: args.nodate_file_format.unwrap_or(args.file_format.clone()),
        unknown_file_format: args.unknown_file_format,
        bracketed_file_format: args.bracketed_file_format,
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
            eprintln!("{e:?}");
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
    analyzer.add_formatter(photo_sort::analysis::name_formatters::BracketedFormat::default());

    debug!("Running program");

    let mut files = Vec::new();

    for source_dir in &analyzer.settings.source_dirs {
        info!("Processing source folder: {}", source_dir.display());
        let result = find_files_in_source(
            source_dir.clone(),
            analyzer.settings.recursive_source,
            &mut files,
        );
        if let Err(err) = result {
            error!("Error processing folder: {err}");
        }
    }

    debug!("Found {} files in source folders", files.len());

    let threadpool = args.threads.map(|v| v.max(1)).map(ThreadPool::new);
    let (sender, receiver) = channel();

    let context = match threadpool {
        None => ExecutionContext::SingleThreaded(Box::new(NormalContext { analyzer })),
        Some(pool) => ExecutionContext::MultiThreaded(ThreadPoolContext {
            output: sender,
            receiver,
            pool,
            analyzer: Arc::new(analyzer),
        }),
    };

    let jobs = files.len();

    let bar = args.progress.then(|| {
        let bar = ProgressBar::new(files.len() as u64);
        bar.set_style(
            ProgressStyle::with_template(
                "{spinner:.green} [{elapsed_precise}] [{wide_bar:.green/grey}] {pos}/{len} ({eta})",
            )
            .unwrap()
            .progress_chars("=>-"),
        );
        multi.add(bar.clone());
        bar
    });

    let mut bracketed_queue = VecDeque::<(PathBuf, BracketEXIFInformation)>::new();
    let mut bracket_group_index = 0;

    let file_count = files.len();
    for (i, file) in files.into_iter().enumerate() {
        if let Some(bar) = &bar {
            bar.set_message(format!("{}", file.display()));
        }

        if bracket_mode {
            match get_bracketing_info(&file) {
                Ok(Some(info)) => {
                    let drain = if let Some(last) = bracketed_queue.back() {
                        if last.0.parent() != file.parent() {
                            trace!("Detected end of bracket sequence: parent path mismatch");
                            true
                        } else if last.1.index + 1 != info.index {
                            trace!(
                                "Detected end of bracket sequence: index mismatch {} -> {}",
                                last.1.index,
                                info.index
                            );
                            true
                        } else {
                            false
                        }
                    } else {
                        false
                    };
                    if drain {
                        drain_bracketed_queue(
                            &mut bracketed_queue,
                            &context,
                            bar.as_ref(),
                            i,
                            &mut bracket_group_index,
                        );
                    }

                    bracketed_queue.push_back((file, info));
                }
                Ok(None) => {
                    if !bracketed_queue.is_empty() {
                        trace!("Detected end of bracket sequence: non-bracketed file");
                        drain_bracketed_queue(
                            &mut bracketed_queue,
                            &context,
                            bar.as_ref(),
                            i,
                            &mut bracket_group_index,
                        );
                    }

                    process_file(file, &context, None);
                }
                Err(e) => {
                    error!("Error processing file {}: {e}", file.display());

                    process_file(file, &context, None);
                }
            }
        } else {
            process_file(file, &context, None);
        }

        if let Some(bar) = &bar {
            bar.set_position(i.saturating_sub(bracketed_queue.len()) as u64);
        }
    }

    if !bracketed_queue.is_empty() {
        trace!("Detected end of bracket sequence: processing end");
        drain_bracketed_queue(
            &mut bracketed_queue,
            &context,
            bar.as_ref(),
            file_count,
            &mut bracket_group_index,
        );
    }

    if let Some(context) = context.multi_threading() {
        if let Some(bar) = &bar {
            bar.set_position(0);
        }
        let mut count = 0;
        for (i, ()) in context.iter().take(jobs).enumerate() {
            if let Some(bar) = &bar {
                bar.set_position(i as u64);
            }
            count += 1;
        }

        if count != jobs {
            error!("Not all jobs got executed");
        }
    }

    if let Some(bar) = &bar {
        bar.finish_with_message("Finished processing files");
    }

    debug!("Finished execution");
}

fn drain_bracketed_queue(
    queue: &mut VecDeque<(PathBuf, BracketEXIFInformation)>,
    context: &ExecutionContext,
    bar: Option<&ProgressBar>,
    target_progress: usize,
    group_index: &mut usize,
) {
    if queue.is_empty() {
        return;
    }

    let first = queue.front().unwrap();
    let last = queue.back().unwrap();
    let sequence_length = queue.len();

    let info = BracketInfo {
        first: first.0.clone(),
        last: last.0.clone(),
        sequence_number: 0,
        sequence_length,
        group_index: *group_index,
    };
    *group_index += 1;

    for (i, file) in queue.iter().enumerate() {
        let mut info = info.clone();
        info.sequence_number = file.1.index;
        process_file(file.0.clone(), context, Some(info));

        if let Some(bar) = bar {
            bar.set_position(target_progress.saturating_sub(queue.len() - 1 - i) as u64);
        }
    }

    queue.clear();
}

struct ThreadPoolContext {
    pub pool: ThreadPool,
    pub output: Sender<()>,
    pub receiver: Receiver<()>,
    pub analyzer: Arc<Analyzer>,
}

struct NormalContext {
    pub analyzer: Analyzer,
}

enum ExecutionContext {
    MultiThreaded(ThreadPoolContext),
    SingleThreaded(Box<NormalContext>),
}

impl ExecutionContext {
    pub fn multi_threading(self) -> Option<Receiver<()>> {
        if let ExecutionContext::MultiThreaded(context) = self {
            Some(context.receiver)
        } else {
            None
        }
    }
}

fn process_file(file: PathBuf, context: &ExecutionContext, bracket_info: Option<BracketInfo>) {
    match context {
        ExecutionContext::SingleThreaded(context) => {
            let result = context.analyzer.run_file(&file, &bracket_info);
            if let Err(err) = result {
                error!("Error processing file: {err}");
            }
        }
        ExecutionContext::MultiThreaded(context) => {
            let output = context.output.clone();
            let analyzer = context.analyzer.clone();
            context.pool.execute(move || {
                let result = analyzer.run_file(&file, &bracket_info);
                if let Err(err) = result {
                    error!("Error processing file: {err}");
                }
                output.send(()).expect("thread pool channel closed");
            });
        }
    }
}
