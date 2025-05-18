{ pkgs, ... }:

let
  garrys-mod = builtins.getFlake "github:vnprc/bitcoin-garrys-mod";
  bitcoind = garrys-mod.packages.${pkgs.system}.gmodBitcoind;

  electrs-flake = builtins.getFlake "github:blockstream/electrs?ref=new-index";
  electrs-bin = electrs-flake.packages.${pkgs.system}.bin;

  # Re-compile electrs-esplora-server target
  electrs-esplora =
    electrs-bin.overrideAttrs (old: {
      pname           = "electrs-esplora-server";
      # crane respects cargoExtraArgs
      cargoExtraArgs  = "--bin electrs-esplora-server";
      doCheck         = false;      # skip the flaky test_rest integration test
    });

  datadir = "./data";
  bitcoinConf = ./config/bitcoin.conf;
in
{
  packages = [
    bitcoind
    electrs-esplora
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

    echo "Waiting for bitcoind RPC to become ready..."
    until ${bitcoind}/bin/bitcoin-cli \
            -datadir=${datadir} -conf=${bitcoinConf} -regtest \
            getblockchaininfo >/dev/null 2>&1; do
        sleep 1
    done

    COOKIE_PATH=${datadir}/regtest/.cookie

    ${electrs-esplora}/bin/electrs \
      --network regtest \
      --daemon-dir ${datadir} \
      --daemon-rpc-addr 127.0.0.1:18443 \
      --cookie $(cat $COOKIE_PATH) \
      --db-dir ./electrs-db \
      --http-addr 0.0.0.0:3000 -vvv
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
      handle_path /api/* {
        uri strip_prefix /api
        reverse_proxy http://127.0.0.1:3000
      }
      reverse_proxy http://127.0.0.1:5001
    }
    CFG
    ${pkgs.caddy}/bin/caddy run --config /tmp/Caddyfile --adapter caddyfile
  '';

  enterShell = ''
    alias bitcoin-cli='bitcoin-cli -datadir=${datadir} -conf=${bitcoinConf} -regtest'
    echo "Bitcoin Core (regtest) running at: 127.0.0.1:18443"
  '';
}
