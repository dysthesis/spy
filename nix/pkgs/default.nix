{
  craneLib,
  pkgs,
  inputs,
  commonArgs,
  cargoArtifacts,
  ...
}:
let

  inherit (pkgs) callPackage;
in
rec {
  kani = callPackage ./kani {
    inherit (inputs) rust-overlay;
  };
  bm = callPackage ./bm {
    inherit
      craneLib
      pkgs
      commonArgs
      cargoArtifacts
      ;
  };
  default = bm;
}
