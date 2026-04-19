# `PhotoSort`

`PhotoSort` is a robust command-line tool written in Rust, designed to streamline the organization of your photo/video
collections. It works by sourcing images/videos from a source directory, extracting the date from either the file name or
its EXIF/metadata data, and then moving or copying the file to a target directory.

`PhotoSort` solves the problem of having pictures/videos from different cameras and devices that use
different naming conventions to name created files. When viewing the photos/videos in a file browser
this can be confusing, as the photos/videos are not sorted by date. `PhotoSort` solves this problem by
renaming images/videos based on their EXIF/metadata data or file name, unifying the naming convention and making
to go through the photos/videos by date.

The documentation can be found here: <https://docs.rs/photo_sort>

## Features

- **Custom Target Format**: You can define your own target date and file name formats for the renamed files.
- **Analysis Mode**: Choose how you want to extract the date from your files. Only EXIF, only name, or a combination.
- **Move Mode**: Choose how you want to move the files to the target directory. Options are moving, coping, hardlinking,
  symlinking, or relative symlinking.
- **Recursive Source Directory**: `PhotoSort` can search the source directories recursively.
- **Dry Run Mode**: Test the tool without making any changes to your files. The tool will print the actions it would
  take without actually executing them.
- **Sort photos and videos**: `PhotoSort` can sort both photos and videos by their metadata.
- **Bracketing Support**: `PhotoSort` can detect bracketed photo sets with different exposure levels and sort them together.
- **File Inclusion/Exclusion**: You can specify which files to include or exclude from processing based on patterns or regular expressions.
- **Multi-threading**: `PhotoSort` can utilize multiple threads to speed up file I/O operations.
- **Progress Bar**: Display a progress bar while processing files to keep track of the sorting progress.

## Usage

To use `PhotoSort`, you need to pass in a set of arguments to define how you want to sort your photos. Here is an example:

```bash
photo_sort \
  --source-dir /path/to/photos \
  --target-dir /path/to/sorted_photos
```

This command will sort the photos in the `/path/to/photos` directory, rename them based on their EXIF data or name and
then move
them to the `/path/to/sorted_photos` directory.

> You are not sure what the tool will do? Run it with the `--dry-run` flag to see what it would do without actually
> changing anything.

Another example:

```bash
photo_sort \
  --source_dir /path/to/photos \
  --recursive \
  --target-dir /path/to/sorted_photos \
  --analysis-mode "exif_then_name" \
  --date-format "%Y-%m-%d-_%H%M%S" \
  --file-format "{date?%Y}/{date}{_:dup}.{ext}" \
  --mkdir \
  --extensions "png,jpg" \
  --move-mode "hardlink"
```

This command will sort the photos in the `/path/to/photos` directory and its subdirectories, rename them based on their
EXIF date (if not found then its name) and then hardlink them to the `/path/to/sorted_photos` directory.
The files will be renamed to the format `YYYY-MM-DD_HHMMSS[_##]`, only `.png` and `.jpg` files will be processed.
The files will be placed in subdirectories based on the year part of the extracted date. Subdirectories will be automatically
created.

For a full list of available options, run `photo_sort --help`:
```text
$ photo_sort --help

A tool to rename and sort photos/videos by its EXIF date/metadata. It tries to extract the date
from the EXIF data or file name and renames the image file according to a given
format string.

Usage: photo_sort [OPTIONS] --source-dir <SOURCE_DIR>... --target-dir <TARGET_DIR>

Options:
  -s, --source-dir <SOURCE_DIR>...     The source directory to read the photos from
  
  -t, --target-dir <TARGET_DIR>        The target directory to write the sorted photos to
  
  -r, --recursive                      Whether to search the source directories recursively. If the flag is not set only
                                       immediate children of the source directories are considered
                                       
      --date-format <DATE_FORMAT>      Date format string to use as default date format. See [https://docs.rs/chrono/latest/chrono/format/strftime/index.html]
                                       for more information [default: %Y%m%d-%H%M%S]
                                       
  -f, --file-format <FILE_FORMAT>      The target file format. Everything outside a {...} block is copied as is. The
                                       target file format may contain "/" to indicate that the file should be placed in
                                       a subdirectory. Use the `--mkdir` flag to create the subdirectories. `{name}` is
                                       replaced with a filename without the date part. `{original_name}` is replaced with
                                       the original filename without modification (without extension). `{original_filename}`
                                       is replaced with the original filename without modification (with extension).
                                       `{dup}` is replaced with a number if a file with the target name already exists.
                                       `{date}` is replaced with the date string, formatted according
                                       to the date_format parameter. `{date?format}` is
                                       replaced with the date string, formatted according to the "format" parameter.
                                       See [https://docs.rs/chrono/latest/chrono/format/strftime/index.html] for more
                                       information. `{type}` is replaced with MOV or IMG. `{type?img,vid}` is replaced
                                       with `img` if the file is an image, `vid` if the file is a video. Note that, when
                                       using other types than IMG or MOV, and rerunning the program again, the custom
                                       type will be seen as part of the file name. `{ext?upper/lower/copy}` is replaced
                                       with the original file extension. If `?upper` or `?lower` is specified, the
                                       extension will be made lower/upper case. leaving out `?...` or using `copy` copies
                                       the original file extension. Commands of the form {label:cmd} are
                                       replaced by {cmd}; if the replacement string is not empty then a prefix of "label"
                                       is added. This might be useful to add separators only if there is e.g. a {dup}
                                       part [default: {type}{_:date}{-:name}{-:dup}]
                                       
      --bracket <BRACKETED_FILE_FORMAT>The target file format for files that can be identified as bracketed photo set with
                                       different exposure levels. By using `--bracket` all files identified as bracketed are
                                       moved/renamed/copied with the specified format string instead of the default one
                                       specified by `--target-dir`. The `--bracket` argument provides the following additional
                                       format specifiers: `{bracket?group}` an increasing number, unique for each group of
                                       bracketed images, `{bracket?index}` the index of the current photo in the sequence,
                                       `{bracket?length}` the length of the bracketing sequence, `{bracket?first}`/`{bracket?last}`
                                       the name of the first/last image in the sequence. Bracketed photos sequences are detected
                                       via manufacturer-specific EXIF information. Note that using the `--bracket` option
                                       requires each file to be analyzed using the EXIF analyzer, even if the Analysis type
                                       is set to Name-only. Currently only works for Sony's cameras. Feel free to open an
                                       issue requesting support for other vendors at https://github.com/0xCCF4/PhotoSort/issues
                                       
       --bracket-fmt <BRACKETING_FORMATTING_PRIORITY> When using the `--bracket` option, this flag controls which e.g. date information
                                       is used for the file name formatting of intermediary directories. For example, with
                                       `{date}/{date}-{name}.{ext}` as `--bracket` format files of the same bracket group
                                       will be placed into different subdirectories when the date changes between shots, if
                                       using the respective date information of each file (`current` mode). Using the `first`/`last`
                                       mode changes the data source for intermediary directory formatting (except final file name)
                                       to use the date information of the first respective last image in the bracketing sequence,
                                       hence keeping files of the same bracket group together in the same subdirectory.
          
          [default: first]

                                       
      --nodate <NODATE_FILE_FORMAT>    The target format for files that have no date. The `analysis_mode` allows specifying
                                       which method should be used to derive a date for a file. See the `file_format` option
                                       for an extensive description of possible format values. If not specified, uses the
                                       same format as for normal files
                                       
      --unknown <UNKNOWN_FILE_FORMAT>  The target file format for files that do not match the specified extensions list.
                                       If not present files that do not match the extension list are ignored, hence not moved,
                                       copied etc. See the `file_format` for an extensive description of possible format values.
                                       By using `--unknown others/{name}{.:ext}` all unknown files are moved to the subdirectory
                                       "others" relative to the target directory (specified by `--target-dir`)
                                       
      --exclude <EXCLUDE_FILES>        Files to exclude completely from processing. Files matched by this pattern are never touched,
                                       even by the `--unknown` argument. This option could be useful to exclude files like `Thumbs.db`
                                       or `.DS_Store` from being moved to the `--unknown` folder. The `--exclude` option matches the
                                       relative file path to the respective source directory to the given string (prefixed by the os path separator).
                                       `*` may be used to indicate any number of any characters, excluding path separators. `**` may
                                       be used to indicate any number of any characters, including path separators. For example,
                                       `--exclude /abc/*/test/**/Thumbs.db` would exclude any file named `Thumbs.db` in a
                                       `abc/<subdir>/test/<any subdirs>/Thumbs.db` folder structure. On Windows use backslash
                                       instead. The `--exclude` may be used multiple times to exclude multiple patterns. If any
                                       pattern matches, the file is excluded. The `--exclude-regex` option works the same but
                                       accepts a regular expression instead of literals with wildcards. By default, the pattern
                                       matching is ignoring case. To enable case matching set `--exclude-case`

      --exclude-regex <EXCLUDE_FILES_REGEX> Same as `--exclude` but accepts regular expressions instead of literals with wildcards.
                                       `*` and `**` wildcards do not work in the regex patterns but are interpreted as regex match the
                                       last pattern character 0 or more times. For a list of supported regex patterns,
                                       see <https://docs.rs/regex/latest/regex/#syntax>

      --exclude-case                   When set the exclude pattern to do not ignore upper/lower case

      --include <INCLUDE_FILES>        When set to any, all files are by default ignored, only files matching any pattern provided via
                                       `--include` or `--include-regex` argument are analysed. See `--exclude` for an explanation of
                                       supported patterns. To enable case matching set `--include-case`. To specify regex patterns use
                                       `--include-regex`. Note that `--exclude` will take priority over `--include`, meaning that a
                                       file matching both an exclude and an include pattern will be excluded

      --include_regex <INCLUDE_FILES_REGEX> Same as `--include` but accepts regular expressions instead of literals with wildcards. `*` and
                                       `**` wildcards do not work in the regex patterns but are interpreted as regex match the last pattern
                                       character 0 or more times. For a list of supported regex patterns, see <https://docs.rs/regex/latest/regex/#syntax>

      --include-case                   When set the include pattern to do not ignore upper/lower case
                                       
      --mkdir                          If the file format contains a "/", indicating that the file should be placed in a
                                       subdirectory, the mkdir flag controls if the tool is allowed to create non-existing subdirectories. No folder is
                                       created in dry-run mode
                                       
  -e, --extensions [<EXTENSIONS>...]   A comma separated list of file extensions to include in the analysis
                                       [default: jpg,jpeg,png,tiff,heif,heic,avif,webp]
                                       
  -a, --analysis-mode <ANALYSIS_MODE>  The sorting mode, possible values are `name_then_metadata`, `metadata_then_name`,
                                       `only_name`, `only_metadata`. Name analysis tries to extract the date from the file name,
                                       Metadata analysis tries to extract the date from the EXIF data/video metadata
                                       [default: exit/video metadata then name]
                                       
       --date-field <EXIF_DATE_TYPE>   The EXIF date field to use, possible values are `modify`, `creation`, `digitized`.
                                       EXIF data contains several date fields. `Modify` is the modification date, which
                                       is updated when the file is edited. `Create` is the creation date, which is usually
                                       the date when the photo was taken. `Digitize` is the digitized date, which is the
                                       date when the photo was digitized (for example, when converting a film photo to
                                       a digital image). For digital cameras, this is usually the same as the creation
                                       date. The default is `create`
                                       [default: create]
                                       
  -m, --move-mode <MOVE_MODE>          The action mode, possible values are move, copy, hardlink, relative_symlink,
                                       absolute_symlink. Move will move the files, Copy will copy the files, Hardlink
                                       (alias: hard) will create hardlinks, RelativeSymlink (alias: relsym) will create
                                       relative symlinks, AbsoluteSymlink (alias: abssym) will create absolute symlinks
                                       [default: move]
                                       
  -n, --dry-run                        Dry-run If set, the tool will not move any files but only print the actions it would take
  
  -v, --verbose                        Be verbose, if set, the tool will print more information about the actions it takes.
                                       
  -d, --debug                          Debug, if set, the tool will print debug information (including debug implies
                                       setting verbose).
                                      
  -l, --log <LOGFILE>                  Logfile, if set, the tool will log its output to the specified file. Appending to
                                       the specified file if it already exists

  -q, --quiet                          If set, suppresses the output of the tool to stdout/stderr. Only displaying error
                                       messages. Specifying a logfile at the same time will redirect the full output that
                                       would have been displayed to stdout/stderr to the logfile. Specifying `--debug` or
                                       `--verbose` plus `--quiet` without a logfile will result in an error
                                      
  -p, --progress                       If set, display a progress bar while processing files
  
  --threads <THREADS>                  If set, use multi-threading

  -h, --help                           Print help
  
  -V, --version                        Print version 
  
When building with video support enabled (see below):
      --video-extensions [<VIDEO_EXTENSIONS>...]  A comma separated list of video extensions to include in the analysis [default: mp4,mov,avi]                                                                                                                                                                 
```

## Installation

To install `PhotoSort`, you need to have Cargo installed on your system.

```bash
cargo install photo_sort
```

or

```bash
cargo install --git https://github.com/0xCCF4/PhotoSort
```

or

```bash
nix run github:0xCCF4/PhotoSort -- <args>
```

The `photo_sort` binary will then be available.

For using the video sorting feature follow the instructions on <https://crates.io/crates/ffmpeg-next> and respective
their wiki <https://github.com/zmwangx/rust-ffmpeg/wiki/Notes-on-building>. After installing the
dependencies, you can install the `photo_sort` binary with the `video` feature enabled:

```bash
cargo install --features video photo_sort
```

or

```bash
cargo install --git https://github.com/0xCCF4/PhotoSort --features video
```

## Contributing

Contributions to `PhotoSort` are welcome! If you have a feature request, bug report, or want to contribute to the code,
please open an issue or a pull request.

### Something works differently than expected?

Try running the tool with the `--debug` argument (and without the `--threads` argument) to get more information about what the tool
is doing and open an issue with the output.

```bash
photo_sort --debug --source_dir /path/to/photos --target_dir /path/to/sorted_photos ...
```

## License

`PhotoSort` is licensed under the GPLv3 license. See the LICENSE file for more details.
