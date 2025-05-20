use bitcoincore_rpc::{Auth, Client, RpcApi};
use bitcoin::{
    Txid, TxIn, script::ScriptBuf,
    opcodes::all::OP_NOP4,
    Opcode,
};
use std::{env, path::Path};

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
            print_witness(&txid, index, &tx.input)?;
        }
        None => {
            for (i, _) in tx.input.iter().enumerate() {
                print_witness(&txid, i, &tx.input)?;
            }
        }
    }

    Ok(())
}

fn print_witness(
    txid: &Txid,
    input_index: usize,
    inputs: &[TxIn],
) -> Result<(), Box<dyn std::error::Error>> {
    let input = &inputs[input_index];

    println!("input[{input_index}] witness script for txid {txid}:\n");

    match input.witness.len() {
        0 => {
            println!("  No witness data\n");
        }
        1 => {
            println!("  Taproot key-path spend (Schnorr sig only). No script present.\n");
        }
        _ => {
            let witness_script = &input.witness[0];
            let script = ScriptBuf::from_bytes(witness_script.to_vec());

            for instr in script.instructions() {
                match instr {
                    Ok(bitcoin::blockdata::script::Instruction::Op(op)) if op == OP_CTV => {
                        println!("  Op(OP_CTV)");
                    }
                    Ok(bitcoin::blockdata::script::Instruction::Op(op)) => {
                        println!("  Op({:?})", op);
                    }
                    Ok(bitcoin::blockdata::script::Instruction::PushBytes(bytes)) => {
                        println!("  PushBytes(0x{})", hex::encode(bytes.as_bytes()));
                    }
                    Err(e) => {
                        println!("  Error parsing instruction: {:?}", e);
                    }
                }
            }

            println!();
        }
    }

    Ok(())
}

