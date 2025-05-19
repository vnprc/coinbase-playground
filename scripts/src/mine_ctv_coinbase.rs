use std::path::Path;
use std::str::FromStr;

use bitcoincore_rpc::{Auth, Client, RpcApi};
use bitcoin::{
    Address, Amount, Network, Transaction, TxOut,
    consensus::Encodable,
    hashes::{sha256, Hash},
    key::{Keypair, Secp256k1},
    opcodes::all::{OP_RETURN, OP_NOP4},
    script::Builder,
    taproot::{TaprootBuilder, TaprootSpendInfo},
    Opcode, XOnlyPublicKey, Sequence,
};

const OP_CTV: Opcode = OP_NOP4;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rpc = Client::new(
        "http://127.0.0.1:18443",
        Auth::CookieFile(Path::new("./data/regtest/.cookie").to_path_buf()),
    )?;

    // 👇 ensure wallet is created and loaded
    let wallet_name = "devwallet"; // 👈 changed
    let _ = rpc.create_wallet(wallet_name, None, None, None, None); // 👈 changed

    let loaded_wallets = rpc.list_wallets()?; // 👈 changed
    if !loaded_wallets.iter().any(|w| w == wallet_name) { // 👈 changed
        rpc.load_wallet(wallet_name)?; // 👈 changed
    }

    // 👇 reconnect to wallet RPC endpoint
    let rpc = Client::new( // 👈 changed
        &format!("http://127.0.0.1:18443/wallet/{}", wallet_name), // 👈 changed
        Auth::CookieFile(Path::new("./data/regtest/.cookie").to_path_buf()), // 👈 changed
    )?;

    let ctv_spend_address = rpc.get_new_address(None, None)?
        .require_network(Network::Regtest)?;

    let anchor_address = Address::from_str("bcrt1pfeesnyr2tx")?
        .require_network(Network::Regtest)?;

    let outputs = [
        TxOut {
            value: Amount::from_sat(1577 - 240),
            script_pubkey: ctv_spend_address.script_pubkey(),
        },
        TxOut {
            value: Amount::from_sat(240),
            script_pubkey: anchor_address.script_pubkey(),
        },
        TxOut {
            value: Amount::from_sat(0),
            script_pubkey: Builder::new()
                .push_opcode(OP_RETURN)
                .push_slice(b"\xf0\x9f\xa5\xaa \xe2\x9a\x93 \xf0\x9f\xa5\xaa")
                .into_script(),
        },
    ];

    let ctv_hash = calc_ctv_hash(&outputs, None);
    let taproot_info = create_ctv_address(ctv_hash);
    let ctv_address = Address::p2tr_tweaked(taproot_info.output_key(), Network::Regtest);

    println!("Mining to CTV contract address: {}", ctv_address);

    let block_hashes = rpc.generate_to_address(1, &ctv_address)?;
    let block_hash = block_hashes[0];

    let block = rpc.get_block(&block_hash)?;
    let coinbase_txid = block.txdata[0].txid();

    println!("Coinbase txid: {}", coinbase_txid);

    let coinbase_tx: Transaction = rpc.get_raw_transaction(&coinbase_txid, None)?;
    let ctv_output_index = coinbase_tx
        .output
        .iter()
        .position(|o| o.script_pubkey == ctv_address.script_pubkey())
        .expect("CTV output not found in coinbase");

    println!("CTV output index: {}", ctv_output_index);
    println!("Output value: {} sats", coinbase_tx.output[ctv_output_index].value);

    Ok(())
}

fn calc_ctv_hash(outputs: &[TxOut], timeout: Option<u32>) -> [u8; 32] {
    let mut buffer = Vec::new();
    buffer.extend(3_i32.to_le_bytes()); // version
    buffer.extend(0_i32.to_le_bytes()); // locktime
    buffer.extend(1_u32.to_le_bytes()); // inputs len

    let seq = if let Some(timeout_value) = timeout {
        sha256::Hash::hash(&Sequence(timeout_value).0.to_le_bytes())
    } else {
        sha256::Hash::hash(&Sequence::ENABLE_RBF_NO_LOCKTIME.0.to_le_bytes())
    };
    buffer.extend(seq.to_byte_array()); // sequences

    buffer.extend((outputs.len() as u32).to_le_bytes());

    let mut output_bytes = Vec::new();
    for o in outputs {
        o.consensus_encode(&mut output_bytes).unwrap();
    }
    buffer.extend(sha256::Hash::hash(&output_bytes).to_byte_array()); // outputs hash

    buffer.extend(0_u32.to_le_bytes()); // inputs index

    sha256::Hash::hash(&buffer).to_byte_array()
}

fn create_ctv_address(ctv_hash: [u8; 32]) -> TaprootSpendInfo {
    let secp = Secp256k1::new();
    let keypair = Keypair::new(&secp, &mut rand::thread_rng());
    let (xonly_pubkey, _parity) = XOnlyPublicKey::from_keypair(&keypair);

    let script = Builder::new()
        .push_slice(&ctv_hash)
        .push_opcode(OP_CTV)
        .into_script();

    let builder = TaprootBuilder::new().add_leaf(0, script).unwrap();
    builder.finalize(&secp, xonly_pubkey).unwrap()
}
