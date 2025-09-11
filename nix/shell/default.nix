{
  craneLib,
  config,
  pkgs,
  kani,
  ...
}:
{
  default = craneLib.devShell {
    inherit (config) checks;
    packages = with pkgs; [
      nixd
      nixfmt
      statix
      deadnix

      cargo-audit
      cargo-expand
      cargo-nextest
      bacon
      rust-analyzer
      kani
    ];
  };
}
