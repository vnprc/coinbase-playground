{
  description = "CTV+CSFS bitcoind node, scripts to generate CTV coinbases, mempool visualizer";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    devenv.url = "github:cachix/devenv";
    garrys-mod.url = "github:vnprc/bitcoin-garrys-mod";
    electrs-rest.url = "github:mempool/electrs?ref=mempool";
  };

  outputs = inputs@{ self, nixpkgs, flake-utils, electrs-rest, devenv, garrys-mod, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlay = (final: prev: {
          electrs = electrs-rest.packages.${system}.default.overrideAttrs (_: {
            doCheck   = false;   # nuke every test
            dontCheck = true;
          });
        });
        pkgs = import nixpkgs { inherit system overlays = [ overlay ]; };
        bitcoind = garrys-mod.packages.${system}.default;
      in {
        packages.default = bitcoind;

        devenv.shells.default = {
          inherit pkgs;
          inputs = { bitcoind = bitcoind; };
        };
      });
}
