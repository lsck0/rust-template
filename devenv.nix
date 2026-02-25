{ pkgs, ... }:

{
  languages.rust = {
    enable = true;
    toolchainFile = ./rust-toolchain.toml;
  };

  packages = [
    pkgs.cargo-deny
    pkgs.cargo-generate
    pkgs.cargo-make
    pkgs.cargo-tarpaulin
    pkgs.git-cliff
    pkgs.hotspot
    pkgs.mold
    pkgs.nushell
    pkgs.perf
    pkgs.typos
    pkgs.zizmor
  ];

  git-hooks.hooks = {
    actionlint.enable = true;
    clippy.enable = true;
    commitizen.enable = true;
    rustfmt.enable = true;
    trufflehog.enable = true;
    typos.enable = true;
  };
}
