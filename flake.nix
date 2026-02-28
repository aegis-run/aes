{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs =
    inputs:
    inputs.flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = inputs.nixpkgs.outputs.legacyPackages.${system};

        toolchain = inputs.fenix.packages.${system}.fromToolchainFile {
          file = ./rust-toolchain.toml;
          sha256 = "sha256-vra6TkHITpwRyA5oBKAHSX0Mi6CBDNQD+ryPSpxFsfg=";
        };

        rustPlatform = pkgs.makeRustPlatform {
          cargo = toolchain;
          rustc = toolchain;
        };
      in
      {
        packages.aes = pkgs.callPackage ./aes.nix { inherit rustPlatform; };
        packages.default = inputs.self.outputs.packages.${system}.aes;

        devShells.default = pkgs.mkShell {
          name = "aes";

          nativeBuildInputs = [
            toolchain

            pkgs.cargo-audit
            pkgs.cargo-deny
            pkgs.cargo-insta
            pkgs.cargo-fuzz
            pkgs.cargo-nextest
            pkgs.cargo-outdated
            pkgs.cargo-watch

            pkgs.jq
            pkgs.just
          ];

          RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
        };
      }
    )
    // {
      overlays.default = final: prev: {
        inherit (inputs.self.packages.${final.system}) aes;
      };
    };
}
