# CTV + CSFS Coinbase Playground 🥪🪙🏰🛝

This repository sets up a full regtest environment for experimenting with [`OP_CHECKTEMPLATEVERIFY`](https://github.com/bitcoin/bips/blob/master/bip-0119.mediawiki) and [`OP_CHECKSIGFROMSTACK`](https://github.com/bitcoin/bips/blob/master/bip-0348.md) using:

- [bitcoin-garrys-mod](https://github.com/average-gary/bitcoin-garrys-mod): a custom Bitcoin Core fork with CTV+CSFS enabled
- [esplora + electrs](https://github.com/blockstream/electrs?ref=new-index) blockchain indexer and browser UI
- rust script to generate a CTV coinbase and spend from it
  - shoutout to stutxo for the [inspiration](https://github.com/stutxo/simple_ctv)
- `justfile` to simplify common actions

---

## 🌟 Getting Star-ted

To run Coinbase Playground, first clone the repository and follow the instructions to [install nix and devenv](https://devenv.sh/getting-started/).

Once set up, cd into the `coinbase-playground` directory and run:

```sh
devenv up --impure
```

Note: electrs [doesn't build on mac](https://github.com/vnprc/coinbase-playground/issues/1). If this is a problem for you, you should fix it and open a PR. (Or get a real dev machine. =P)

This will take a while to run. It's building bitcoin core from source, among other things. When it finishes open another tab and run

```sh
devenv shell --impure
```

Once running:

- use the devenv shell to interact with the playground
- type `just` to see available actions
- Esplora UI is available at [http://localhost:5000](http://localhost:5000)

## 🛠️ Just Recipes

| Command                                | Description                                                  |
|----------------------------------------|--------------------------------------------------------------|
| `mine-and-send`                | Mine initial coins and send 1 BTC to a new address |
| `mine-ctv-coinbase`            | Mine and spend a CTV coinbase transaction |
| `mine-ctv-coinbase <outputs>`  | Mine and spend a CTV coinbase with 25 outputs |
| `build-esplora`                | Clone and build the Esplora frontend |
| `reset-chain`                  | Wipe chain data and reset to block 0 |
| `parse-witness <txid>`         | Parse all input witness scripts for a transaction |
| `parse-witness <txid> <index>` | Parse one input witness script for a transaction |
| `mine-layered-ctv-coinbase`    | Mine and spend a 2 level CTV tree with fixed fees |

---

## 🔍 Explore Transactions

You can explore the blockchain and transactions via:

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

## 🤔 Ok, But What Does All This Mean?

Good question! Put simply, CTV enables noncustodial mining pools.

Every mining pool today, with the exception of [Ocean](https://ocean.xyz), is a custodial pool. They take posession of all new bitcoin mined by the pool and you have to trust that they will pay you what you are owed. It's the 'trust me bro' payout model. As bitcoiners, we should not accept this state of affairs. We can do better. In order to save bitcoin from the mining cartels, we must do better.

It doesn't have to be this way. Pools could pay out to their largest miners directly in the coinbase (the first transaction of each block where new bitcoin originates). Ocean does this, but they are severely limited in the amount of outputs they can put in the coinbase. One problem is that making the coinbase larger takes away from the potential transaction fees. A much bigger problem, though, is miner firmware restrictions.

Antmain, the largest ASIC manufacturer by far, limits the size of the coinbase transaction in their miner firmware. They put this limit in place to stifle competition from decentralized alternatives. And it worked! The decentralized mining software P2Pool died a slow death, in large part due to Antminer firmware restrictions.

Ocean is the only pool that bothers to work around this limitation to provide non-custodial payouts for their customers. In [this talk](https://www.youtube.com/watch?v=EKQvDfmQkt8&t=8910s) at 3:03:00 Jason Hughes details the extensive measures they take to make it work. They fingerprint the hardware in use by their miners and keep track of multiple work templates based on the results. They also have to be very loose with the validation of miner submitted blocks because there's no telling what the coinbase will look like until after the block is found. This is a really tough and completely unnecessary engineering problem.

CTV coinbase transactions eliminate this problem. With CTV we can construct a large transaction tree with a great number of outputs and commit to the entire payout structure in a very small transaction footprint.

There are three big wins, in order of importance:
1. Break Antmain's stranglehold on the coinbase. gfy Jihan!
1. Enable non-custodial mining pools at any scale of operation
1. Maximize fee revenue in each block the pool mines

There are two downsides:
1. Users must get additional transactions mined to claim their rewards
1. Someone must make the unroll transaction data available

This repo is a tool to explore the possibilities of different coinbase structures. Once I got the custom bitcoin node and block explorer working (which was no small feat 😅) I built two payout mechanisms: a flat tree structure, and a layered binary tree.

### Flat Payout Tree

The flat payout is intended to be broadcast to the mempool immediately with a 1 sat/vb fee taken from the miner payout. It also includes a 330 sat anchor output that anyone can spend to fee bump the transaction. Users could potentially crowdsource the fee transaction using sighash_anyonecanpay. This solution solves the data availability problem by avoiding nested CTV transactions and immediately broadcasting the CTV spend to the mempool after the block is mined. The transaction will sit in the mempool for 100 blocks and get mined as soon as prevailing fee rates are low enough. Users can bump the fees if they don't want to wait. My testing showed 319 possible outputs for this transaction before running against TRUC policy limits. Not too bad!

### Layered Payout Tree

The layered tree structure is a much more complicated proposition. I built a simple binary tree with 2 layers and 4 leaves. Each transaction carries a fixed 500 sat fee. I did not have time to iterate further but I think the next steps are to replace the fixed fee with 0-value anchor outputs, make the number of leaves, depth of the tree, and the radix (number of children per parent node) configurable.

### Endgame

I think the end game is to create a tree with an n of n musig locking script at each node. Then the owners of the leaves could spend the 100 blocks after confirmation trading outputs to consolidate the tree into fewer nodes. For example, if you swap off-chain funds for the signature of your leaf's siblings you can collapse the subtree by one level and get a larger on-chain payout. This use case fits perfectly with the P2Pool reboot as Kulpreet explains in this [blog post](https://blog.opdup.com/2025/02/26/trading-shares-for-bitcoin-user-story.html).