# PhotoSort

PhotoSort is a robust command-line tool written in Rust, designed to streamline the organization of your photo
collections. It works by sourcing images from a source directory, extracting the date from either the file name or
its EXIF data, and then moving or copying the file to a target directory.

PhotoSort solves the problem of having pictures from different cameras and devices that use
different naming conventions to name created files. When viewing the photos in a file browser
this can be confusing, as the photos are not sorted by date. PhotoSort solves this problem by
renaming images based on their EXIF data or file name, unifying the naming convention and making
to go through the photos by date.

The documentation can be found here: https://docs.rs/photo_sort/latest/photo_sort/

## Features

- **Custom Target Format**: You can define your own target date and file name formats for the renamed files.
- **Analysis Mode**: Choose how you want to extract the date from your files. Only EXIF, only name, or a combination.
- **Move Mode**: Choose how you want to move the files to the target directory. Options are moving, coping, hardlinking,
  symlinking, or relative symlinking.
- **Recursive Source Directory**: PhotoSort can search the source directories recursively.
- **Dry Run Mode**: Test the tool without making any changes to your files. The tool will print the actions it would
  take without actually executing them.

## Usage

To use PhotoSort, you need to pass in a set of arguments to define how you want to sort your photos. Here is an example:

```bash
photo_sort --source_dir /path/to/photos --target_dir /path/to/sorted_photos
```

This command will sort the photos in the `/path/to/photos` directory, rename them based on their EXIF data or name and
then move
them to the `/path/to/sorted_photos` directory.

Another example:

```bash
photo_sort
  --source_dir /path/to/photos
  --recursive
  --target_dir /path/to/sorted_photos
  --analysis-mode "exif_then_name"
  --date-format "%Y-%m-%d-_%H%M%S"
  --file-format "{:date}{:?dup}"
  --extensions "png,jpg"
  --move-mode "hardlink"
```

This command will sort the photos in the `/path/to/photos` directory and its subdirectories, rename them based on their
EXIF date (if not found then its name) and then hardlink them to the `/path/to/sorted_photos` directory.
The files will be renamed to the format `YYYY-MM-DD_HHMMSS[_##]`, only `.png` and `.jpg` files will be processed.

For a full list of available options, run `photo_sort --help`.

## Installation

To install PhotoSort, you need to have Cargo installed on your system.

```bash
cargo install photo_sort
```

or

```bash
git clone https://github.com/username/photo_sort.git
cd photo_sort
cargo install --path .
```

The `photo_sort` binary will be available in the `target/release` directory.

## Contributing

Contributions to PhotoSort are welcome! If you have a feature request, bug report, or want to contribute to the code,
please open an issue or a pull request.

## License

PhotoSort is licensed under the GPLv3 license. See the LICENSE file for more details.
