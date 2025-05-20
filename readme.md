# CTV + CSFS Coinbase Playground 🥪🪙🏰🛝

This repository sets up a full regtest environment for experimenting with [`OP_CHECKTEMPLATEVERIFY`](https://github.com/bitcoin/bips/blob/master/bip-0119.mediawiki) and [`OP_CHECKSIGFROMSTACK`](https://github.com/bitcoin/bips/blob/master/bip-0348.md) using:

- [bitcoin-garrys-mod](https://github.com/average-gary/bitcoin-garrys-mod): a custom Bitcoin Core fork with CTV+CSFS enabled
- electrs + esplora browser based blockchain explorer
- Rust script to generate a CTV coinbase and spend from it
  - shoutout to stutxo for the [inspiration](https://github.com/stutxo/simple_ctv)
- A `justfile` to simplify common actions

---

## 🌟 Getting Star-ted

To run Coinbase Playground, first clone the repository and follow the instructions to [install nix and devenv](https://devenv.sh/getting-started/).

Once set up, cd into the `coinbase-playground` directory and run:

```
devenv up
```

Open another tab and run

```
devenv shell
```

Once running:

- use the devenv shell to interact with the playground
- type `just` to see available actions
- Esplora UI is available at [http://localhost:5000](http://localhost:5000)

## 🛠️ Just Recipes

| Command                                | Description                                                  |
|----------------------------------------|--------------------------------------------------------------|
| `just mine-and-send`                   | Mine initial coins and send 1 BTC to a new address           |
| `just mine-ctv-coinbase`               | Mine and spend a CTV coinbase transaction |
| `just mine-ctv-coinbase outputs 25`    | Mine and spend a CTV with 25 outputs                   |
| `just build-esplora`                   | Clone and build the Esplora frontend                         |
| `just reset-chain`                     | Wipe chain data and reset to block 0                         |

---

## 🔍 Explore Transactions

After running `mine-ctv-coinbase`, you can explore transactions via:

- **UI**: [http://localhost:5000](http://localhost:5000)
- **API**: `curl http://localhost:5000/api/tx/<txid>`

---

## 📁 Repo Layout

```text
./
├── scripts/              # Rust scripts for interacting with the blockchain
├── config/bitcoin.conf   # bitcoind regtest config
├── devenv.nix            # full environment definition
├── flake.nix             # reproducible nix build definition
├── justfile              # helper commands for scripts and setup
└── esplora-frontend/     # (created by `just build-esplora`)
```

Chain data is stored in `./data/` and Electrs DB in `./electrs-db/`.

---

## 🧹 Reset

If you want to start from scratch:

```sh
just reset-chain
```

Be sure to restart `devenv` afterward.

---
