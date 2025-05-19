_default:
  @echo "Available commands:"
  @just --summary | tr ' ' '\n'

mine-and-send:
    cargo run -p scripts --bin mine_and_send

mine-ctv-coinbase outputs="50":
    cargo run -p scripts --bin mine_ctv_coinbase -- {{outputs}}

build-esplora:
  rm -rf tmp-esplora esplora-frontend
  git clone https://github.com/Blockstream/esplora tmp-esplora
  bash -c 'cd tmp-esplora && npm install && npm run dist'
  mv tmp-esplora/dist esplora-frontend
  rm -rf tmp-esplora

reset-chain:
  rm -rf ./data/regtest
  @echo "🧹 Regtest chain wiped. Next run will start from block 0. Be sure to restart devenv."
