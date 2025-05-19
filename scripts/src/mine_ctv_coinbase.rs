use std::env;
use std::path::Path;

use bitcoincore_rpc::{Auth, Client, RpcApi};
use bitcoin::{
    Address, Amount, Network, Transaction, TxOut, TxIn, OutPoint,
    consensus::{Encodable, encode::{serialize, serialize_hex}},
    hashes::{sha256, Hash},
    key::{Keypair, Secp256k1},
    script::{Builder, ScriptBuf},
    taproot::{TaprootBuilder, TaprootSpendInfo, LeafVersion},
    Opcode, XOnlyPublicKey, Sequence,
};

use bitcoin::opcodes::all::{OP_NOP4, OP_PUSHNUM_1};

const OP_CTV: Opcode = OP_NOP4;
const ANCHOR_VALUE: u64 = 330;
const ANCHOR_PUSHBYTES: [u8; 2] = [0x4e, 0x73];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output_count: usize = env::args().nth(1).unwrap_or("50".to_string()).parse()?;

    let rpc = Client::new(
        "http://127.0.0.1:18443",
        Auth::CookieFile(Path::new("./data/regtest/.cookie").to_path_buf()),
    )?;

    ensure_wallet(&rpc, "devwallet")?;

    let rpc = Client::new(
        "http://127.0.0.1:18443/wallet/devwallet",
        Auth::CookieFile(Path::new("./data/regtest/.cookie").to_path_buf()),
    )?;

    let secp = Secp256k1::new();
    let keypair = Keypair::new(&secp, &mut rand::thread_rng());
    let (xonly_pubkey, _) = XOnlyPublicKey::from_keypair(&keypair);

    let fee_rate = 1;
    let include_anchor = true;

    let ctv_spend_address = rpc.get_new_address(None, None)?.require_network(Network::Regtest)?;

    // mine a dummy block to get the actual coinbase value
    let dummy_address = rpc.get_new_address(None, None)?.require_network(Network::Regtest)?;
    let dummy_block = rpc.generate_to_address(1, &dummy_address)?[0];
    let dummy_txid = rpc.get_block(&dummy_block)?.txdata[0].txid();
    let dummy_coinbase_tx: Transaction = rpc.get_raw_transaction(&dummy_txid, None)?;
    let actual_coinbase_value = dummy_coinbase_tx.output[0].value.to_sat();

    // Now construct spend tx and CTV tree with real input amount
    let (taproot_info, ctv_address, mut spend_tx, ctv_script) = build_ctv_contract(
        &secp,
        xonly_pubkey,
        actual_coinbase_value,
        fee_rate,
        &ctv_spend_address,
        include_anchor,
        output_count,
        &rpc,
    )?;

    println!("Mining to CTV contract address: {}", ctv_address);
    let coinbase_block = rpc.generate_to_address(1, &ctv_address)?[0];
    let coinbase_txid = rpc.get_block(&coinbase_block)?.txdata[0].txid();

    // Mature the coinbase
    rpc.generate_to_address(100, &ctv_spend_address)?;

    // Fill in prevout
    spend_tx.input[0].previous_output = OutPoint {
        txid: coinbase_txid,
        vout: 0,
    };

    // Finalize witness
    let ctrl_block = taproot_info
        .control_block(&(ctv_script.clone(), LeafVersion::TapScript))
        .unwrap();

    spend_tx.input[0].witness.push(ctv_script.into_bytes());
    spend_tx.input[0].witness.push(ctrl_block.serialize());

    // Broadcast
    let tx_hex = serialize_hex(&spend_tx);
    println!("Spending tx: {tx_hex}");
    let txid = rpc.send_raw_transaction(tx_hex)?;
    println!("Broadcasted txid: {txid}");

    // mine it
    rpc.generate_to_address(1, &ctv_spend_address)?;
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

fn anchor_output() -> TxOut {
    TxOut {
        value: Amount::from_sat(ANCHOR_VALUE),
        script_pubkey: Builder::new()
            .push_opcode(OP_PUSHNUM_1)
            .push_slice(&ANCHOR_PUSHBYTES)
            .into_script(),
    }
}

fn build_ctv_contract(
    secp: &Secp256k1<bitcoin::secp256k1::All>,
    xonly: XOnlyPublicKey,
    input_value_sat: u64,
    fee_rate: u64,
    ctv_spend_address: &Address,
    include_anchor: bool,
    output_count: usize,
    rpc: &Client,
) -> Result<(TaprootSpendInfo, Address, Transaction, ScriptBuf), Box<dyn std::error::Error>> {
    let fee = calculate_fee_with_anchor(secp, xonly, fee_rate, output_count, ctv_spend_address, include_anchor, rpc)?;
    let reserved = if include_anchor { ANCHOR_VALUE } else { 0 };
    let per_output_value = (input_value_sat - fee - reserved) / output_count as u64;

    let mut outputs = vec![];

    for _ in 0..output_count {
        outputs.push(TxOut {
            value: Amount::from_sat(per_output_value),
            script_pubkey: ctv_spend_address.script_pubkey(),
        });
    }

    if include_anchor {
        outputs.push(anchor_output());
    }

    let ctv_hash = calc_ctv_hash(&outputs, None);
    let ctv_script = Builder::new()
        .push_slice(&ctv_hash)
        .push_opcode(OP_CTV)
        .into_script();

    let taproot_info = TaprootBuilder::new()
        .add_leaf(0, ctv_script.clone())
        .unwrap()
        .finalize(secp, xonly)
        .unwrap();

    let ctv_address = Address::p2tr_tweaked(taproot_info.output_key(), Network::Regtest);

    let spend_tx = Transaction {
        version: bitcoin::transaction::Version(3),
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint::null(),
            sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness: bitcoin::Witness::default(),
            script_sig: bitcoin::ScriptBuf::new(),
        }],
        output: outputs,
    };

    Ok((taproot_info, ctv_address, spend_tx, ctv_script))
}

fn calculate_fee_with_anchor(
    secp: &Secp256k1<bitcoin::secp256k1::All>,
    xonly: XOnlyPublicKey,
    fee_rate: u64,
    output_count: usize,
    ctv_spend_address: &Address,
    include_anchor: bool,
    rpc: &Client,
) -> Result<u64, Box<dyn std::error::Error>> {
    let mut dummy_outputs = vec![TxOut {
        value: Amount::from_sat(0),
        script_pubkey: ctv_spend_address.script_pubkey(),
    }; output_count];

    if include_anchor {
        dummy_outputs.push(anchor_output());
    }

    let dummy_ctv_hash = calc_ctv_hash(&dummy_outputs, None);
    let dummy_ctv_script = Builder::new()
        .push_slice(&dummy_ctv_hash)
        .push_opcode(OP_CTV)
        .into_script();

    let dummy_taproot_info = TaprootBuilder::new()
        .add_leaf(0, dummy_ctv_script.clone())
        .unwrap()
        .finalize(secp, xonly)
        .unwrap();

    let mut dummy_tx = Transaction {
        version: bitcoin::transaction::Version(3),
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint::null(),
            sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness: bitcoin::Witness::default(),
            script_sig: bitcoin::ScriptBuf::new(),
        }],
        output: dummy_outputs,
    };

    dummy_tx.input[0].witness.push(dummy_ctv_script.to_bytes());
    dummy_tx.input[0].witness.push(
        dummy_taproot_info
            .control_block(&(dummy_ctv_script.clone(), LeafVersion::TapScript))
            .ok_or("failed to get control block")?
            .serialize(),
    );

    let raw_tx = serialize(&dummy_tx);
    let vsize = rpc.test_mempool_accept(&[&dummy_tx])?
        .into_iter()
        .next()
        .and_then(|res| res.vsize)
        .unwrap_or(bitcoin::consensus::serialize(&dummy_tx).len() as u64);

    Ok(vsize * fee_rate)
}

fn calc_ctv_hash(outputs: &[TxOut], timeout: Option<u32>) -> [u8; 32] {
    let mut buffer = Vec::new();
    buffer.extend(3_i32.to_le_bytes()); // version
    buffer.extend(0_i32.to_le_bytes()); // locktime
    buffer.extend(1_u32.to_le_bytes()); // input count

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

    buffer.extend(0_u32.to_le_bytes()); // input index

    sha256::Hash::hash(&buffer).to_byte_array()
}
