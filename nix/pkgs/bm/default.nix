{
  craneLib,
  commonArgs,
  cargoArtifacts,
  ...
}:
craneLib.buildPackage (
  commonArgs
  // {
    inherit cargoArtifacts;
    # We are already running nextest on ../../checks/nextest.nix
    doCheck = false;
  }
)
