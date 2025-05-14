{ pkgs, ... }:

let
  garrys-mod = builtins.getFlake "github:vnprc/bitcoin-garrys-mod";
  bitcoind = garrys-mod.packages.${pkgs.system}.gmodBitcoind;
in
{
  packages = [
    bitcoind
    pkgs.openssl
    pkgs.pkg-config
  ];

  languages.rust.enable = true;

  processes.bitcoind.exec = ''
    ${bitcoind}/bin/bitcoind \
      -regtest -txindex=1 -fallbackfee=0.0001 \
      -rpcuser=admin -rpcpassword=password \
      -rpcbind=127.0.0.1 -rpcallowip=127.0.0.1
  '';
}
