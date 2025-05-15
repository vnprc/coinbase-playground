{ pkgs, ... }:

let
  garrys-mod = builtins.getFlake "github:vnprc/bitcoin-garrys-mod";
  bitcoind = garrys-mod.packages.${pkgs.system}.gmodBitcoind;
  datadir = "./data";
  conf = ./config/bitcoin.conf;
in
{
  packages = [
    bitcoind
    pkgs.openssl
    pkgs.pkg-config
  ];

  languages.rust.enable = true;

  processes.bitcoind.exec = ''
    mkdir -p ${datadir}
    ${bitcoind}/bin/bitcoind -datadir=${datadir} -conf=${conf}
  '';

  enterShell = ''
    alias bitcoin-cli='bitcoin-cli -datadir=${datadir} -conf=${conf} -regtest'
  '';
}
