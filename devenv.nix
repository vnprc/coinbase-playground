{ pkgs, ... }:

let
  garrys-mod = builtins.getFlake "github:vnprc/bitcoin-garrys-mod";
  bitcoind = garrys-mod.packages.${pkgs.system}.gmodBitcoind;
  datadir = "./data";
  bitcoinConf = ./config/bitcoin.conf;
  electrsConf = ./electrs.toml;
in
{
  packages = [
    bitcoind
    pkgs.electrs
    pkgs.nodejs
    pkgs.nodePackages.npm
    pkgs.nodePackages.serve
    pkgs.miniserve
    pkgs.openssl
    pkgs.pkg-config
  ];

  languages.rust.enable = true;

  processes.bitcoind.exec = ''
    mkdir -p ${datadir}
    ${bitcoind}/bin/bitcoind -datadir=${datadir} -conf=${bitcoinConf}
  '';

  processes.electrs.exec = ''
    mkdir -p ./electrs-db

    echo "Waiting for .cookie file..."
    while [ ! -f ${datadir}/regtest/.cookie ]; do
      sleep 1
    done

    ${pkgs.electrs}/bin/electrs \
      --network regtest \
      --daemon-dir ${datadir} \
      --db-dir ./electrs-db \
      --conf ${electrsConf}
  '';

  processes.esplora-ui.exec = ''
    ${pkgs.nodePackages.serve}/bin/serve \
      -l 5001 \
      -s esplora-frontend
  '';

  processes.proxy.exec = ''
    cat > /tmp/Caddyfile <<'CFG'
    {
      auto_https off
    }
    :5000 {
      reverse_proxy /api/* 127.0.0.1:3000
      reverse_proxy        127.0.0.1:5001
    }
    CFG
    ${pkgs.caddy}/bin/caddy run --config /tmp/Caddyfile --adapter caddyfile
  '';

  enterShell = ''
    alias bitcoin-cli='bitcoin-cli -datadir=${datadir} -conf=${bitcoinConf} -regtest'
    echo "Bitcoin Core (regtest) running at: 127.0.0.1:18443"
  '';
}
