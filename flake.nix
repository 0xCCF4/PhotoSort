{
  description = "PhotoSort";

  inputs = {
    flake-parts.url = "github:hercules-ci/flake-parts";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    naersk.url = "github:nix-community/naersk";
    naersk.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = inputs@{ flake-parts, naersk, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" "x86_64-darwin" ];
      perSystem = { config, self', inputs', pkgs, system, ... }:
        let
          naersk' = pkgs.callPackage naersk { };
        in
        {
          packages.default = naersk'.buildPackage {
            src = ./.;
            gitAllRefs = true;
          };
        };
    };
}
