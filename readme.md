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

## 🌴 Flat Payout Tree

The flat payout is intended to be broadcast to the mempool immediately with a 1 sat/vb fee taken from the coinbase reward. It also includes a 330 sat anchor output that anyone can spend to fee bump the transaction. Users could potentially crowdsource the fee transaction using `SIGHASH_ANYONECANPAY`. This solution solves the data availability problem by avoiding nested CTV transactions and immediately broadcasting the CTV spend to the mempool after the block is mined. The transaction will sit in the mempool for 100 blocks and get mined as soon as prevailing fee rates are low enough. Users can bump the fees if they don't want to wait. My testing showed an upper limit of 319 payout outputs for this transaction before running against TRUC transaction size policy limits. Not too bad!

```sh
(devenv) bash-5.2$ just mine-ctv-coinbase 12
cargo run -p scripts --bin mine_ctv_coinbase -- 12
...
Mining to CTV contract address: bcrt1p5uhmqtuymnfryf8zjjv99dsvl9kr7kjvnwemj46fpq244ps5uksqfvek9m
Spending tx: 03000000000101d7016d67a893c77ef4dadccfa5ec90e14bf9c26d2d5d0e4c0e173a21667aca200000000000fdffffff0de9d3d518000000001600148d8cbed03aafe3940d06df99eff965ad47b5f35ee9d3d518000000001600148d8cbed03aafe3940d06df99eff965ad47b5f35ee9d3d518000000001600148d8cbed03aafe3940d06df99eff965ad47b5f35ee9d3d518000000001600148d8cbed03aafe3940d06df99eff965ad47b5f35ee9d3d518000000001600148d8cbed03aafe3940d06df99eff965ad47b5f35ee9d3d518000000001600148d8cbed03aafe3940d06df99eff965ad47b5f35ee9d3d518000000001600148d8cbed03aafe3940d06df99eff965ad47b5f35ee9d3d518000000001600148d8cbed03aafe3940d06df99eff965ad47b5f35ee9d3d518000000001600148d8cbed03aafe3940d06df99eff965ad47b5f35ee9d3d518000000001600148d8cbed03aafe3940d06df99eff965ad47b5f35ee9d3d518000000001600148d8cbed03aafe3940d06df99eff965ad47b5f35ee9d3d518000000001600148d8cbed03aafe3940d06df99eff965ad47b5f35e4a010000000000000451024e73022220d7b6a27dee682eca6be8bc06d8d9232585ab156f86082cf707473e20b511494db321c11eba5fe7fad9d6a7676350866e2bb23662c352f96bcb8b79555ca1c2ed39e71f00000000
Broadcasted txid: 5e7542ea2b7a7802a99a53e38ce6df48853de99bb9d6380a07d257afefa4746f
Mined txid: 5e7542ea2b7a7802a99a53e38ce6df48853de99bb9d6380a07d257afefa4746f
```

![Screenshot from 2025-05-20 18-41-57](https://github.com/user-attachments/assets/fa03682e-ac29-4805-bb9f-6e0874f63bed)

The outputs all pay to the same address but just pretend they are different lol. You can see the CTV script using the `just parse-witness` script. I wrote this script because esplora doesn't parse the input witness script and I wanted to see the `OP_CTV` script for myself.

```sh
(devenv) bash-5.2$ just parse-witness 5e7542ea2b7a7802a99a53e38ce6df48853de99bb9d6380a07d257afefa4746f
cargo run -p scripts --bin parse_witness -- 5e7542ea2b7a7802a99a53e38ce6df48853de99bb9d6380a07d257afefa4746f 0
...
input[0] analysis for txid 5e7542ea2b7a7802a99a53e38ce6df48853de99bb9d6380a07d257afefa4746f:

  scriptPubKey type: p2tr
  Script-path spend (tapleaf)

  Disassembled script:
    PushBytes(0xd7b6a27dee682eca6be8bc06d8d9232585ab156f86082cf707473e20b511494d)
    Op(OP_CTV)
```

## 🌲 Layered Payout Tree

The layered tree structure is a much more complicated proposition. I built a simple binary tree with 2 layers and 4 leaves. Each transaction carries a fixed 500 sat fee. I did not have time to iterate further but I think the next steps are to replace the fixed fee with 0-value anchor outputs, make the number of leaves, depth of the tree, and the radix (number of children per parent node) configurable. This tree structure is strictly worse for mining pool payouts than the flat structure, but it is a stepping stone to more awesomer features.

```sh
(devenv) bash-5.2$ just mine-layered-ctv-coinbase
cargo run -p scripts --bin mine_layered_ctv_coinbase
...
Mining to: bcrt1pyu95vzyhv5wzw0knt4306nzd08444q6nrfg2gqln4z8h6jpdcn4s6e87d9
Spend tx: 0300000000010135e272debe4ff3d32138c7dd248b2fbc9e7c3b69a32f2630a31acdfd9caedd500000000000fdffffff0212f6029500000000222053a995c4b1b5ee4b1a7b14fe1a8aaa69d8bad8365b1c41c804eceaf03cf334e4b312f60295000000002220ee1b7e0ce5a96400f31fd0727c0c76884a2bb451f4a454240acee9d15854c4a1b30222205133a9cf8b3fc41cab69eb81cee6ce4e5e6e54c09cf400d47cc5bc00aa7e43fdb321c025bac28b52f7c13541189b9403a4644a98a75466fdd8f319128012366f11bce000000000
Broadcast root txid: 757a278dc29d25f57043c897a3463fbd3c71e21b8e7a5c45fc92e0e0f56939a0
Left child tx: 0300000001a03969f5e0e092fc455c7a8e1be2713cbd3f46a397c84370f5259dc28d277a750000000000fdffffff020f7a814a0000000016001479ba9fe5ce2387a5bca741d88e36cf050191af860f7a814a00000000160014884a09b4b3b3fbd43334c89a3dce0f08c3c4704000000000
Right child tx: 0300000001a03969f5e0e092fc455c7a8e1be2713cbd3f46a397c84370f5259dc28d277a750100000000fdffffff020f7a814a00000000160014455e4528a8fda28fa442fde9347a7b2cbdf57eb50f7a814a00000000160014a76cd9c0a9582a1ad83ec09491b7a849551194f300000000
Mined child txids: 5ea40581e3cb0f35b746700907fe2c79250868dfe7a5f7dded4890a207609fe4, 0359b10aeb783da68f1cd6d6e745d04e66107f4ba09aa9f0b6f3a969100fb323
```

![Screenshot from 2025-05-20 18-48-02](https://github.com/user-attachments/assets/45f4f764-c8f9-4321-8f28-260c15f09f83)
![Screenshot from 2025-05-20 18-47-38](https://github.com/user-attachments/assets/84fe5a51-7924-445b-8923-70dff7b190f5)

Notice the `OP_NOP4` in the scriptPubKey in the top right of the first image. This is `OP_CTV` by anther name. Esplora isn't aware of the CTV activation code running in the bitcoin node so it prints the opcode that CTV overrides. You can also see `OP_NOP4` on the prevout script on the left side of the lower image.

## 🚀 Endgame

I think the end game is to create a tree with an n of n musig locking script at each node. Then the owners of the leaves could spend the 100 blocks after confirmation trading outputs to consolidate the tree into fewer nodes. For example, if you swap off-chain funds for the signature(s) of your leaf's sibling(s) you can collapse the subtree by one level and get a larger on-chain payout with less transactions. This use case fits very nicely with the P2Pool reboot as Kulpreet explains in this [blog post](https://blog.opdup.com/2025/02/26/trading-shares-for-bitcoin-user-story.html).

I intend to continue using this playground to explore what's possible when you control the coinbase. I think there is a whole world of use cases to discover, as I explain in [this talk](https://www.youtube.com/watch?app=desktop&v=F2p_V0svDTo&t=3h15m30s).
