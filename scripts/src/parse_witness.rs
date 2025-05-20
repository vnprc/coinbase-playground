use bitcoincore_rpc::{Auth, Client, RpcApi};
use bitcoin::{
    Txid, TxIn, ScriptBuf, Address,
    opcodes::all::OP_NOP4,
    blockdata::script::Instruction,
    Opcode,
};
use std::{env, path::Path};
use hex;

const OP_CTV: Opcode = OP_NOP4;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let txid_str = env::args().nth(1).expect("txid required");
    let txid: Txid = txid_str.parse()?;

    let maybe_index = env::args().nth(2);

    let rpc = Client::new(
        "http://127.0.0.1:18443/wallet/devwallet",
        Auth::CookieFile(Path::new("./data/regtest/.cookie").to_path_buf()),
    )?;

    let tx = rpc.get_raw_transaction(&txid, None)?;

    match maybe_index {
        Some(index_str) => {
            let index: usize = index_str.parse()?;
            print_input_analysis(&rpc, &txid, index, &tx.input[index])?;
        }
        None => {
            for (i, input) in tx.input.iter().enumerate() {
                print_input_analysis(&rpc, &txid, i, input)?;
            }
        }
    }

    Ok(())
}

fn print_input_analysis(
    rpc: &Client,
    txid: &Txid,
    index: usize,
    input: &TxIn,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("input[{index}] analysis for txid {txid}:\n");

    if input.previous_output.is_null() {
        println!("  Coinbase input (no prevout)\n");
        return Ok(());
    }

    let prev_txid = input.previous_output.txid;
    let vout = input.previous_output.vout;

    let prev_tx = rpc.get_raw_transaction(&prev_txid, None)?;
    let spent_output = &prev_tx.output[vout as usize];
    let spk = &spent_output.script_pubkey;

    let spend_type = classify_spk(spk);
    println!("  scriptPubKey type: {spend_type}");

    let contains_ctv = spk.instructions().any(|i| {
        matches!(i, Ok(Instruction::Op(op)) if op == OP_CTV)
    });

    if contains_ctv {
        println!("  💡 This input spends an OP_CTV contract (CTV spend). Look for OP_NOP4 in Esplora!");
    }

    match spend_type.as_str() {
        "p2tr" => {
            if input.witness.len() == 1 {
                println!("  Key-path spend (schnorr sig only)\n");
            } else if input.witness.len() >= 2 {
                println!("  Script-path spend (tapleaf)\n");
                parse_script_witness(&input.witness[0])?;
            } else {
                println!("  Unexpected witness layout\n");
            }
        }
        "p2wpkh" | "p2sh-p2wpkh" => {
            println!("  SegWit key spend (P2WPKH or P2SH-P2WPKH), no script\n");
        }
        "p2wsh" => {
            println!("  Witness script spend (P2WSH)\n");
            parse_script_witness(&input.witness.last().unwrap())?;
        }
        _ => {
            println!("  Unknown or non-segwit input type\n");
        }
    }

    println!();
    Ok(())
}

fn parse_script_witness(witness_bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let script = ScriptBuf::from_bytes(witness_bytes.to_vec());

    println!("  Disassembled script:");

    for instr in script.instructions() {
        match instr {
            Ok(Instruction::Op(op)) if op == OP_CTV => {
                println!("    Op(OP_CTV)");
            }
            Ok(Instruction::Op(op)) => {
                println!("    Op({:?})", op);
            }
            Ok(Instruction::PushBytes(bytes)) => {
                println!("    PushBytes(0x{})", hex::encode(bytes.as_bytes()));
            }
            Err(e) => {
                println!("    Error parsing instruction: {:?}", e);
            }
        }
    }

    Ok(())
}

fn classify_spk(spk: &ScriptBuf) -> String {
    if let Ok(addr) = Address::from_script(spk, bitcoin::Network::Regtest) {
        match addr.payload() {
            bitcoin::address::Payload::WitnessProgram(witprog) => {
                match (witprog.version(), witprog.program().len()) {
                    (bitcoin::WitnessVersion::V0, 20) => "p2wpkh",
                    (bitcoin::WitnessVersion::V0, 32) => "p2wsh",
                    (bitcoin::WitnessVersion::V1, 32) => "p2tr",
                    _ => "unknown-witness",
                }                
            }
            bitcoin::address::Payload::ScriptHash(_) => "p2sh",
            bitcoin::address::Payload::PubkeyHash(_) => "p2pkh",
            _ => "other",
        }
    } else {
        "nonstandard"
    }.to_string()
}
