{
  description = "PhotoSort - rename and sort photos and videos by EXIF date";

  inputs = {
    flake-parts.url = "github:hercules-ci/flake-parts";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];

      perSystem =
        { pkgs, ... }:
        let
          photo_sort = pkgs.callPackage ./nix/package.nix { };

          devToolchain = with pkgs; [
            cargo
            rustc
            rustfmt
            clippy
            rust-analyzer
            pkg-config
            clang
            ffmpeg
            llvmPackages.libclang
          ];
        in
        {
          packages = {
            default = photo_sort;
            inherit photo_sort;
          };

          checks.default = photo_sort;

          apps.default = {
            type = "app";
            program = "${photo_sort}/bin/photo_sort";
            meta.description = "Rename and sort photos and videos by their EXIF date";
          };

          devShells.default = pkgs.mkShell {
            name = "photo_sort-dev";
            packages = devToolchain;

            # ffmpeg-sys-next
            LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
          };
        };
    };
}
