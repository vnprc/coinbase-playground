use std::path::Path;

use bitcoincore_rpc::{Auth, Client, RpcApi};
use bitcoin::{
    Address, Amount, Network, Transaction, TxOut, TxIn, OutPoint,
    consensus::{Encodable, encode::{serialize_hex}},
    hashes::{sha256, Hash},
    key::{Keypair, Secp256k1},
    script::{Builder, ScriptBuf},
    taproot::{TaprootBuilder, LeafVersion},
    Opcode, XOnlyPublicKey, Sequence,
};

use bitcoin::opcodes::all::OP_NOP4;

const OP_CTV: Opcode = OP_NOP4;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rpc = Client::new(
        "http://127.0.0.1:18443",
        Auth::CookieFile(Path::new("./data/regtest/.cookie").to_path_buf()),
    )?;

    ensure_wallet(&rpc, "devwallet")?;

    let secp = Secp256k1::new();
    let keypair = Keypair::new(&secp, &mut rand::thread_rng());
    let (xonly_pubkey, _) = XOnlyPublicKey::from_keypair(&keypair);

    let dummy_addr = rpc.get_new_address(None, None)?.require_network(Network::Regtest)?;
    let dummy_block = rpc.generate_to_address(1, &dummy_addr)?[0];
    let dummy_txid = rpc.get_block(&dummy_block)?.txdata[0].txid();
    let dummy_coinbase_tx: Transaction = rpc.get_raw_transaction(&dummy_txid, None)?;
    let coinbase_value = dummy_coinbase_tx.output[0].value.to_sat();

    let fee = calculate_layered_fee(&secp, xonly_pubkey, &rpc)?; // 🟠 use vbytes-based fee
    let output_value = (coinbase_value - fee) / 4;

    let leaf_outputs = build_outputs(output_value * 4, &rpc);
    let (root_script, root_outputs) = build_recursive_ctv_tree(&leaf_outputs)?;

    let taproot_info = TaprootBuilder::new()
        .add_leaf(0, root_script.clone())?
        .finalize(&secp, xonly_pubkey)
        .map_err(|e| format!("taproot finalize: {e:?}"))?;
    let tap_address = Address::p2tr_tweaked(taproot_info.output_key(), Network::Regtest);

    println!("Mining coinbase to Taproot CTV address: {}", tap_address);
    let coinbase_block = rpc.generate_to_address(1, &tap_address)?[0];
    let coinbase_txid = rpc.get_block(&coinbase_block)?.txdata[0].txid();
    rpc.generate_to_address(100, &dummy_addr)?;

    let spend_tx = Transaction {
        version: bitcoin::transaction::Version(3),
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint { txid: coinbase_txid, vout: 0 },
            script_sig: ScriptBuf::new(),
            sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness: bitcoin::Witness::default(),
        }],
        output: root_outputs.clone(), // 🟠 only committed outputs, no change
    };

    let ctrl_block = taproot_info
        .control_block(&(root_script.clone(), LeafVersion::TapScript))
        .ok_or("control block missing")?;
    let mut spend_tx = spend_tx;
    spend_tx.input[0].witness.push(root_script.into_bytes());
    spend_tx.input[0].witness.push(ctrl_block.serialize());

    let tx_hex = serialize_hex(&spend_tx);
    println!("Spending tx: {tx_hex}");
    let txid = rpc.send_raw_transaction(tx_hex)?;
    println!("Broadcasted txid: {txid}");
    rpc.generate_to_address(1, &dummy_addr)?;
    println!("Mined txid: {txid}");

    Ok(())
}

fn ensure_wallet(rpc: &Client, wallet_name: &str) -> Result<(), bitcoincore_rpc::Error> {
    let _ = rpc.create_wallet(wallet_name, None, None, None, None);
    if !rpc.list_wallets()?.contains(&wallet_name.to_string()) {
        rpc.load_wallet(wallet_name)?;
    }
    Ok(())
}

fn build_outputs(total_value: u64, rpc: &Client) -> Vec<TxOut> {
    let mut outputs = Vec::with_capacity(4);
    for _ in 0..4 {
        let addr = rpc
            .get_new_address(None, None)
            .expect("get address")
            .require_network(Network::Regtest)
            .expect("network check");
        outputs.push(TxOut {
            value: Amount::from_sat(total_value / 4),
            script_pubkey: addr.script_pubkey(),
        });
    }
    outputs
}

fn calc_ctv_hash(outputs: &[TxOut], timeout: Option<u32>) -> [u8; 32] {
    let mut buffer = Vec::new();
    buffer.extend(3_i32.to_le_bytes());
    buffer.extend(0_i32.to_le_bytes());
    buffer.extend(1_u32.to_le_bytes());

    let seq = if let Some(timeout_value) = timeout {
        sha256::Hash::hash(&Sequence(timeout_value).0.to_le_bytes())
    } else {
        sha256::Hash::hash(&Sequence::ENABLE_RBF_NO_LOCKTIME.0.to_le_bytes())
    };
    buffer.extend(seq.to_byte_array());

    buffer.extend((outputs.len() as u32).to_le_bytes());

    let mut output_bytes = Vec::new();
    for o in outputs {
        o.consensus_encode(&mut output_bytes).unwrap();
    }
    buffer.extend(sha256::Hash::hash(&output_bytes).to_byte_array());

    buffer.extend(0_u32.to_le_bytes());

    sha256::Hash::hash(&buffer).to_byte_array()
}

fn build_ctv_script(outputs: &[TxOut]) -> ScriptBuf {
    let hash = calc_ctv_hash(outputs, None);
    Builder::new().push_slice(&hash).push_opcode(OP_CTV).into_script()
}

fn build_recursive_ctv_tree(outputs: &[TxOut]) -> Result<(ScriptBuf, Vec<TxOut>), Box<dyn std::error::Error>> {
    assert_eq!(outputs.len(), 4);

    let left_tx = Transaction {
        version: bitcoin::transaction::Version(3),
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint::null(),
            script_sig: ScriptBuf::new(),
            sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness: bitcoin::Witness::default(),
        }],
        output: outputs[0..2].to_vec(),
    };
    let left_script = build_ctv_script(&left_tx.output);

    let right_tx = Transaction {
        version: bitcoin::transaction::Version(3),
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint::null(),
            script_sig: ScriptBuf::new(),
            sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness: bitcoin::Witness::default(),
        }],
        output: outputs[2..4].to_vec(),
    };
    let right_script = build_ctv_script(&right_tx.output);

    let left_output = TxOut {
        value: outputs[0].value + outputs[1].value,
        script_pubkey: left_script.clone(),
    };
    let right_output = TxOut {
        value: outputs[2].value + outputs[3].value,
        script_pubkey: right_script.clone(),
    };

    let root_tx = Transaction {
        version: bitcoin::transaction::Version(3),
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint::null(),
            script_sig: ScriptBuf::new(),
            sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness: bitcoin::Witness::default(),
        }],
        output: vec![left_output.clone(), right_output.clone()],
    };

    let root_script = build_ctv_script(&root_tx.output);

    Ok((root_script, vec![left_output, right_output]))
}

fn calculate_layered_fee(
    secp: &Secp256k1<bitcoin::secp256k1::All>,
    xonly: XOnlyPublicKey,
    rpc: &Client,
) -> Result<u64, Box<dyn std::error::Error>> {
    let dummy_address = rpc.get_new_address(None, None)?.require_network(Network::Regtest)?;
    let dummy_outputs: Vec<TxOut> = (0..2)
        .map(|_| TxOut {
            value: Amount::from_sat(0),
            script_pubkey: dummy_address.script_pubkey(),
        })
        .collect();
    let leaf_script = build_ctv_script(&dummy_outputs);

    let parent_outputs = vec![
        TxOut { value: Amount::from_sat(0), script_pubkey: leaf_script.clone() },
        TxOut { value: Amount::from_sat(0), script_pubkey: leaf_script.clone() },
    ];

    let root_script = build_ctv_script(&parent_outputs);

    let taproot_info = TaprootBuilder::new()
        .add_leaf(0, root_script.clone())?
        .finalize(secp, xonly)
        .map_err(|e| format!("taproot finalize: {e:?}"))?;

    let mut dummy_tx = Transaction {
        version: bitcoin::transaction::Version(3),
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint::null(),
            script_sig: ScriptBuf::new(),
            sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness: bitcoin::Witness::default(),
        }],
        output: parent_outputs,
    };

    dummy_tx.input[0].witness.push(root_script.to_bytes());
    dummy_tx.input[0].witness.push(
        taproot_info.control_block(&(root_script.clone(), LeafVersion::TapScript)).unwrap().serialize(),
    );

    let vsize = bitcoin::consensus::encode::serialize(&dummy_tx).len();
    Ok((vsize as u64 * 1).max(500)) // 🟠 1 sat/vB or 500 sats minimum for regtest
}