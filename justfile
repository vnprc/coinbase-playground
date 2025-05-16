_default:
  @echo "Available commands:"
  @just --summary | tr ' ' '\n'

mine_and_send:
    cargo run -p scripts --bin mine_and_send

build-esplora:
  rm -rf tmp-esplora esplora-frontend
  git clone https://github.com/Blockstream/esplora tmp-esplora
  bash -c 'cd tmp-esplora && npm install && npm run dist'
  mv tmp-esplora/dist esplora-frontend
  rm -rf tmp-esplora
