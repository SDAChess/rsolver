{
  description = "Rust development environment for rsolver";

  inputs = {
    nixpkgs.url = "https://channels.nixos.org/nixpkgs-unstable/nixexprs.tar.zst";
    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs:
    inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [ inputs.treefmt-nix.flakeModule ];

      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];

      perSystem =
        { pkgs, system, ... }:
        let
          rustToolchain = pkgs.rust-bin.stable.latest.default.override {
            extensions = [
              "rust-src"
              "rust-analyzer"
            ];
          };
          procxy = pkgs.callPackage ./nix/package.nix {
            rustPlatform = pkgs.makeRustPlatform {
              cargo = rustToolchain;
              rustc = rustToolchain;
            };
          };
        in
        {
          _module.args.pkgs = import inputs.nixpkgs {
            inherit system;
            overlays = [ inputs.rust-overlay.overlays.default ];
          };

          packages = {
            inherit procxy;
            default = procxy;
          };

          devShells.default = pkgs.mkShell {
            packages = [
              rustToolchain
              pkgs.bazel_9
            ];
            RUSTC = "${rustToolchain}/bin/rustc";
            RUST_LINKER = "${pkgs.stdenv.cc}/bin/cc";
          };

          treefmt.programs = {
            rustfmt = {
              enable = true;
              package = rustToolchain;
            };
            nixfmt.enable = true;
            taplo.enable = true;
            buildifier.enable = true;
          };

          treefmt.settings.formatter.buildifier.options = [
            "--lint=warn"
            "--warnings=-module-docstring"
          ];
        };
    };
}
