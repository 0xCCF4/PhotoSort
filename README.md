# PhotoSort

PhotoSort is a robust command-line tool written in Rust, designed to streamline the organization of your photo/video
collections. It works by sourcing images/videos from a source directory, extracting the date from either the file name or
its EXIF/metadata data, and then moving or copying the file to a target directory.

PhotoSort solves the problem of having pictures/videos from different cameras and devices that use
different naming conventions to name created files. When viewing the photos/videos in a file browser
this can be confusing, as the photos/videos are not sorted by date. PhotoSort solves this problem by
renaming images/videos based on their EXIF/metadata data or file name, unifying the naming convention and making
to go through the photos/videos by date.

The documentation can be found here: https://docs.rs/photo_sort

## Features

- **Custom Target Format**: You can define your own target date and file name formats for the renamed files.
- **Analysis Mode**: Choose how you want to extract the date from your files. Only EXIF, only name, or a combination.
- **Move Mode**: Choose how you want to move the files to the target directory. Options are moving, coping, hardlinking,
  symlinking, or relative symlinking.
- **Recursive Source Directory**: PhotoSort can search the source directories recursively.
- **Dry Run Mode**: Test the tool without making any changes to your files. The tool will print the actions it would
  take without actually executing them.
- **Sort photos and videos**: PhotoSort can sort both photos and videos by their metadata.

## Usage

To use PhotoSort, you need to pass in a set of arguments to define how you want to sort your photos. Here is an example:

```bash
photo_sort \
  --source_dir /path/to/photos \
  --target_dir /path/to/sorted_photos
```

This command will sort the photos in the `/path/to/photos` directory, rename them based on their EXIF data or name and
then move
them to the `/path/to/sorted_photos` directory.

Another example:

```bash
photo_sort \
  --source_dir /path/to/photos \
  --recursive \
  --target_dir /path/to/sorted_photos \
  --analysis-mode "exif_then_name" \
  --date-format "%Y-%m-%d-_%H%M%S" \
  --file-format "{:date}{:?dup}" \
  --extensions "png,jpg" \
  --move-mode "hardlink"
```

This command will sort the photos in the `/path/to/photos` directory and its subdirectories, rename them based on their
EXIF date (if not found then its name) and then hardlink them to the `/path/to/sorted_photos` directory.
The files will be renamed to the format `YYYY-MM-DD_HHMMSS[_##]`, only `.png` and `.jpg` files will be processed.

For a full list of available options, run `photo_sort --help`:
```text
$ photo_sort --help

A tool to rename and sort photos by its EXIF date. It tries to extract the date
from the EXIF data or file name and renames the image file according to a given
format string.

Foreach source directory all images are processed and renamed to the target directory

Usage: photo_sort [OPTIONS] --source-dir <SOURCE_DIR>... --target-dir <TARGET_DIR>

Options:
  -s, --source-dir <SOURCE_DIR>...     The source directory to read the photos from
  -t, --target-dir <TARGET_DIR>        The target directory to write the sorted photos to
  -r, --recursive                      Whether to search the source directories recursively. If the flag is not set only immediate children of the source directories are considered
      --date-format <DATE_FORMAT>      Date format string to use for the target directory. The format string is passed to the `chrono` crate's `format` method [default: %Y%m%d-%H%M%S]
  -f, --file-format <FILE_FORMAT>      The target file format. {:date} is replaced with the date and {:name} with the original file name. {:dup} is replaced with a number if the file already exists. {:date} is replaced with the date and {:name} with the original file name. {:?dup} is replaced with _{:dup} if the file already exists [default: IMG_{:date}_{:name}{:?dup}]
  -e, --extensions [<EXTENSIONS>...]   A comma separated list of file extensions to include in the analysis [default: jpg,jpeg,png]
  -a, --analysis-mode <ANALYSIS_MODE>  The sorting mode, possible values are name_then_exif, exif_then_name, only_name, only_exif. Name analysis tries to extract the date from the file name, Exif analysis tries to extract the date from the EXIF data [default: exif_then_name]
  -m, --move-mode <MOVE_MODE>          The action mode, possible values are move, copy, hardlink, relative_symlink, absolute_symlink. Move will move the files, Copy will copy the files, Hardlink (alias: hard) will create hardlinks, RelativeSymlink (alias: relsym) will create relative symlinks, AbsoluteSymlink (alias: abssym) will create absolute symlinks [default: move]
  -n, --dry-run                        Dry-run If set, the tool will not move any files but only print the actions it would take
  -v, --verbose                        Be verbose, if set, the tool will print more information about the actions it takes. Setting the RUST_LOG env var overrides this flag
  -d, --debug                          Debug, if set, the tool will print debug information (including debug implies setting verbose). Setting the RUST_LOG env var overrides this flag
  -h, --help                           Print help
  -V, --version                        Print version        
  
When building with video support enabled (see below):
      --video-extensions [<VIDEO_EXTENSIONS>...]  A comma separated list of video extensions to include in the analysis [default: mp4,mov,avi]                                                                                                                                                                 
```

## Installation

To install PhotoSort, you need to have Cargo installed on your system.

```bash
cargo install photo_sort
```

or

```bash
git clone https://github.com/0xCCF4/photo_sort.git
cd photo_sort
cargo install --path .
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
git clone https://github.com/0xCCF4/photo_sort.git
cd photo_sort
cargo install --features video --path .
```

## Contributing

Contributions to PhotoSort are welcome! If you have a feature request, bug report, or want to contribute to the code,
please open an issue or a pull request.

## License

PhotoSort is licensed under the GPLv3 license. See the LICENSE file for more details.
