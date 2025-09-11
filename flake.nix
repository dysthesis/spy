{
  description = "bm - a plaintext bookmark manager";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane.url = "github:ipetkov/crane";
    flake-parts.url = "github:hercules-ci/flake-parts";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
    systems.url = "github:nix-systems/default";
  };

  outputs =
    inputs@{
      crane,
      flake-parts,
      systems,
      advisory-db,
      ...
    }:
    flake-parts.lib.mkFlake { inherit inputs; } (_: {
      systems = import systems;
      perSystem =
        { config, pkgs, ... }:
        let
          craneLib = crane.mkLib pkgs;
          # Common arguments can be set here to avoid repeating them later
          # NOTE: changes here will rebuild all dependency crates
          src = craneLib.cleanCargoSource ./.;
          commonArgs = {
            inherit src;
            strictDeps = true;
            # Build tools needed at compile time (host)
            nativeBuildInputs = pkgs.lib.optionals pkgs.stdenv.isLinux [
              pkgs.pkg-config
            ];

            # Target libraries for linking
            buildInputs =
              (pkgs.lib.optionals pkgs.stdenv.isDarwin [ pkgs.libiconv ])
              ++ (pkgs.lib.optionals pkgs.stdenv.isLinux [ pkgs.openssl ]);
          };

          cargoArtifacts = craneLib.buildDepsOnly commonArgs;
        in
        rec {
          checks = import ./nix/checks {
            inherit (packages) bm;
            inherit (pkgs) lib;
            inherit
              craneLib
              cargoArtifacts
              commonArgs
              src
              advisory-db
              ;
          };
          packages = import ./nix/pkgs {
            inherit
              craneLib
              pkgs
              inputs
              commonArgs
              cargoArtifacts
              ;
          };
          devShells = import ./nix/shell {
            inherit craneLib config pkgs;
            inherit (packages) kani;
          };

          apps.default = {
            type = "app";
            program = "${packages.bm}/bin/bm";
          };
        };
    });
}
