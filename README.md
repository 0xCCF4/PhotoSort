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
                                       replaced with a filename without the date part. `{dup}` is replaced with a number
                                       if a file with the target name already exists. `{date}` is replaced with the date
                                       string, formatted according to the date_format parameter. `{date?format}` is
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
                                       moved to this folder instead of the one specified by `--target-dir`. The photo's date
                                       will be extracted from the first bracketed photo in the sequence. The sequence number
                                       can be accessed with the format specifier `{bracket}`. Bracketed photos sequences are
                                       detected via manufacturer-specific EXIF information. Note that using the `--bracket`
                                       option requires each file to be analyzed using the EXIF analyzer, even if the Analysis
                                       type is set to Name-only. Currently only works for Sony's cameras. Feel free to open
                                       an issue requesting support for other vendors at https://github.com/0xCCF4/PhotoSort/issues.

                                       
      --nodate <NODATE_FILE_FORMAT>    The target format for files that have no date. The `analysis_mode` allows specifying
                                       which method should be used to derive a date for a file. See the `file_format` option
                                       for an extensive description of possible format values. If not specified, uses the
                                       same format as for normal files
                                       
      --unknown <UNKNOWN_FILE_FORMAT>  The target file format for files that do not match the specified extensions list.
                                       If not present files that do not match the extension list are ignored, hence not moved,
                                       copied etc. See the `file_format` for an extensive description of possible format values.
                                       By using `--unknown others/{name}{.:ext}` all unknown files are moved to the subdirectory
                                       "others" relative to the target directory (specified by `--target-dir`)
                                       
      --mkdir                          If the file format contains a "/", indicating that the file should be placed in a
                                       subdirectory, the mkdir flag controls if the tool is allowed to create non-existing subdirectories. No folder is
                                       created in dry-run mode
                                       
  -e, --extensions [<EXTENSIONS>...]   A comma separated list of file extensions to include in the analysis
                                       [default: jpg,jpeg,png,tiff,heif,heic,avif,webp]
                                       
  -a, --analysis-mode <ANALYSIS_MODE>  The sorting mode, possible values are name_then_exif, exif_then_name, only_name,
                                       only_exif. Name analysis tries to extract the date from the file name, Exif
                                       analysis tries to extract the date from the EXIF data [default: exif_then_name]
                                       
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
