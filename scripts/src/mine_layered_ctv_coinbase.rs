use std::path::Path;

use bitcoincore_rpc::{Auth, Client, RpcApi};
use bitcoin::{
    Address, Amount, Network, Transaction, TxOut, TxIn, OutPoint,
    consensus::{Encodable, encode::serialize_hex},
    hashes::{sha256, Hash},
    key::{Keypair, Secp256k1},
    script::{Builder, ScriptBuf},
    taproot::{TaprootBuilder, LeafVersion},
    Opcode, XOnlyPublicKey, Sequence, Witness,
};

use bitcoin::opcodes::all::OP_NOP4;

const OP_CTV: Opcode = OP_NOP4;
const CHILD_FEE: u64 = 500;
const ROOT_FEE: u64 = 500;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rpc = Client::new(
        "http://127.0.0.1:18443",
        Auth::CookieFile(Path::new("./data/regtest/.cookie").to_path_buf()),
    )?;

    ensure_wallet(&rpc, "devwallet")?;

    let secp = Secp256k1::new();
    let keypair = Keypair::new(&secp, &mut rand::thread_rng());
    let (xonly, _) = XOnlyPublicKey::from_keypair(&keypair);

    let dummy_addr = rpc.get_new_address(None, None)?.require_network(Network::Regtest)?;
    let cb_block = rpc.generate_to_address(1, &dummy_addr)?[0];
    let cb_txid = rpc.get_block(&cb_block)?.txdata[0].txid();
    let cb_tx: Transaction = rpc.get_raw_transaction(&cb_txid, None)?;
    let cb_value = cb_tx.output[0].value.to_sat();

    let spendable = cb_value - ROOT_FEE - 2 * CHILD_FEE;
    let child_value = spendable / 2;
    let leaf_outputs = build_leaf_outputs(&rpc, child_value);

    let left_script = build_ctv_script(&leaf_outputs[0..2]);
    let right_script = build_ctv_script(&leaf_outputs[2..4]);

    let left_out = TxOut {
        value: Amount::from_sat(child_value),
        script_pubkey: left_script.clone(),
    };
    let right_out = TxOut {
        value: Amount::from_sat(child_value),
        script_pubkey: right_script.clone(),
    };

    let root_tx = Transaction {
        version: bitcoin::transaction::Version(3),
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![TxIn::default()],
        output: vec![left_out.clone(), right_out.clone()],
    };
    let root_script = build_ctv_script(&root_tx.output);

    let taproot = TaprootBuilder::new()
        .add_leaf(0, root_script.clone())?
        .finalize(&secp, xonly)
        .map_err(|e| format!("taproot finalize failed: {e:?}"))?;
    let tap_addr = Address::p2tr_tweaked(taproot.output_key(), Network::Regtest);

    println!("Mining to: {}", tap_addr);
    let final_block = rpc.generate_to_address(1, &tap_addr)?[0];
    let final_txid = rpc.get_block(&final_block)?.txdata[0].txid();
    rpc.generate_to_address(100, &dummy_addr)?;

    let spend_tx = Transaction {
        version: bitcoin::transaction::Version(3),
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint { txid: final_txid, vout: 0 },
            script_sig: ScriptBuf::new(),
            sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness: Witness::default(),
        }],
        output: vec![left_out.clone(), right_out.clone()],
    };

    let mut spend_tx = spend_tx;
    let ctrl_block = taproot
        .control_block(&(root_script.clone(), LeafVersion::TapScript))
        .ok_or("missing control block")?;

    spend_tx.input[0].witness.push(root_script.to_bytes());
    spend_tx.input[0].witness.push(ctrl_block.serialize());

    let tx_hex = serialize_hex(&spend_tx);
    println!("Spend tx: {}", tx_hex);

    let root_spend_txid = rpc.send_raw_transaction(tx_hex)?;
    println!("Broadcast root txid: {root_spend_txid}");
    rpc.generate_to_address(1, &dummy_addr)?;

    // Broadcast left child tx
    let left_child = Transaction {
        version: bitcoin::transaction::Version(3),
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint { txid: root_spend_txid, vout: 0 },
            script_sig: ScriptBuf::new(),
            sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness: Witness::default(),
        }],
        output: leaf_outputs[0..2].to_vec(),
    };
    let left_hex = serialize_hex(&left_child);
    println!("Left child tx: {}", left_hex);
    let left_txid = rpc.send_raw_transaction(left_hex)?;

    // Broadcast right child tx
    let right_child = Transaction {
        version: bitcoin::transaction::Version(3),
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint { txid: root_spend_txid, vout: 1 },
            script_sig: ScriptBuf::new(),
            sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness: Witness::default(),
        }],
        output: leaf_outputs[2..4].to_vec(),
    };
    let right_hex = serialize_hex(&right_child);
    println!("Right child tx: {}", right_hex);
    let right_txid = rpc.send_raw_transaction(right_hex)?;

    rpc.generate_to_address(1, &dummy_addr)?;
    println!("Mined child txids: {}, {}", left_txid, right_txid);

    Ok(())
}

fn build_leaf_outputs(rpc: &Client, total_value: u64) -> Vec<TxOut> {
    let half = (total_value - CHILD_FEE) / 2;
    (0..4)
        .map(|_| {
            let addr = rpc.get_new_address(None, None).unwrap().require_network(Network::Regtest).unwrap();
            TxOut {
                value: Amount::from_sat(half),
                script_pubkey: addr.script_pubkey(),
            }
        })
        .collect()
}

fn build_ctv_script(outputs: &[TxOut]) -> ScriptBuf {
    let hash = calc_ctv_hash(outputs);
    Builder::new().push_slice(&hash).push_opcode(OP_CTV).into_script()
}

fn calc_ctv_hash(outputs: &[TxOut]) -> [u8; 32] {
    let mut buf = Vec::new();
    buf.extend(3_i32.to_le_bytes());
    buf.extend(0_i32.to_le_bytes());
    buf.extend(1_u32.to_le_bytes());
    buf.extend(sha256::Hash::hash(&Sequence::ENABLE_RBF_NO_LOCKTIME.0.to_le_bytes()).to_byte_array());
    buf.extend((outputs.len() as u32).to_le_bytes());

    let mut out_buf = Vec::new();
    for o in outputs {
        o.consensus_encode(&mut out_buf).unwrap();
    }
    buf.extend(sha256::Hash::hash(&out_buf).to_byte_array());
    buf.extend(0_u32.to_le_bytes());

    sha256::Hash::hash(&buf).to_byte_array()
}

fn ensure_wallet(rpc: &Client, name: &str) -> Result<(), bitcoincore_rpc::Error> {
    let _ = rpc.create_wallet(name, None, None, None, None);
    if !rpc.list_wallets()?.contains(&name.to_string()) {
        rpc.load_wallet(name)?;
    }
    Ok(())
}
