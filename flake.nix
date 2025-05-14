{
  description = "CTV+CSFS bitcoind node, scripts to generate CTV coinbases, mempool visualizer";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    devenv.url = "github:cachix/devenv";
    garrys-mod.url = "github:vnprc/bitcoin-garrys-mod";
  };

  outputs = inputs@{ self, nixpkgs, flake-utils, devenv, garrys-mod, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        bitcoind = garrys-mod.packages.${system}.default;
      in {
        packages.default = bitcoind;

        devenv.shells.default = {
          inherit pkgs;
          inputs = { bitcoind = bitcoind; };
        };
      });
}
