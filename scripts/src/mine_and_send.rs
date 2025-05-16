use std::path::Path;
use bitcoincore_rpc::{Auth, Client, RpcApi};
use bitcoincore_rpc::bitcoin::{Address, Amount, Network};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rpc = Client::new(
        "http://127.0.0.1:18443",
        Auth::CookieFile(Path::new("./data/regtest/.cookie").to_path_buf()),
    )?;
    
    let wallet_name = "devwallet";

    // create and load wallet
    if let Err(e) = rpc.create_wallet(wallet_name, None, None, None, None) {
        if !e.to_string().contains("already exists") {
            return Err(e.into());
        }
    }

    let loaded_wallets = rpc.list_wallets()?;
    if !loaded_wallets.iter().any(|w| w == wallet_name) {
        if let Err(e) = rpc.load_wallet(wallet_name) {
            return Err(e.into());
        }
    }

    // Check spendable balance
    let balance = rpc.get_balance(None, None)?;
    if balance < Amount::from_btc(1.0)? {
        // Get mining address
        let mining_addr_unchecked = rpc.get_new_address(None, None)?;
        let mining_addr: Address = mining_addr_unchecked.require_network(Network::Regtest)?;

        // Mine 101 blocks to mature coinbase
        let blocks = rpc.generate_to_address(101, &mining_addr)?;
        println!("Generated {} blocks to reach spendable balance", blocks.len());
    }

    // Confirm balance
    let balance = rpc.get_balance(None, None)?;
    println!("Confirmed balance: {} BTC", balance.to_btc());

    // Get a recipient address
    let dest_addr_unchecked = rpc.get_new_address(None, None)?;
    let dest_addr: Address = dest_addr_unchecked.require_network(Network::Regtest)?;
    println!("Sending to address: {}", dest_addr);

    // Send funds
    let amount = Amount::from_btc(1.0)?;
    let txid = rpc.send_to_address(
        &dest_addr,
        amount,
        None, None, None, None, None, None,
    )?;
    println!("Sent 1.0 BTC, txid: {}", txid);

    // Mine a block to confirm
    let mining_addr_unchecked = rpc.get_new_address(None, None)?;
    let mining_addr: Address = mining_addr_unchecked.require_network(Network::Regtest)?;
    rpc.generate_to_address(1, &mining_addr)?;
    println!("Confirmed transaction in new block.");

    Ok(())
}
