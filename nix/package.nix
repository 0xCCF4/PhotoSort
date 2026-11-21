{
  lib,
  rustPlatform,
  installShellFiles,
  pkg-config,
  ffmpeg,
  withVideo ? true,
}:

rustPlatform.buildRustPackage {
  pname = "photo_sort";
  version = (lib.importTOML ../Cargo.toml).package.version;

  src = ../.;
  cargoLock.lockFile = ../Cargo.lock;

  nativeBuildInputs =
    [ installShellFiles ]
    ++ lib.optionals withVideo [
      pkg-config
      rustPlatform.bindgenHook
    ];
  buildInputs = lib.optionals withVideo [ ffmpeg ];

  buildFeatures = lib.optionals withVideo [ "video" ];

  postInstall = ''
    manpage=$(mktemp -d)
    BUILD_MANPAGE="$manpage" "$out/bin/photo_sort"
    mv "$manpage/photo-sort.1" "$manpage/photo_sort.1"
    installManPage "$manpage/photo_sort.1"

    installShellCompletion --cmd photo_sort \
      --bash <(BUILD_SHELL=bash "$out/bin/photo_sort") \
      --fish <(BUILD_SHELL=fish "$out/bin/photo_sort") \
      --zsh <(BUILD_SHELL=zsh "$out/bin/photo_sort")
  '';

  meta = {
    description = "Rename and sort photos and videos by their EXIF date";
    homepage = "https://github.com/0xCCF4/PhotoSort";
    license = lib.licenses.gpl3Plus;
    mainProgram = "photo_sort";
  };
}
