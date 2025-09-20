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
  spy = callPackage ./spy {
    inherit
      craneLib
      pkgs
      commonArgs
      cargoArtifacts
      ;
  };
  default = spy;
}
