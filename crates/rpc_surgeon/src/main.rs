use anyhow::{Context, Result};
use clap::Parser;
use rpc_surgeon::{derive_mapping_slot, get_storage_at};

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    #[arg(short, long)]
    contract: String,

    #[arg(short, long)]
    owner: String,

    #[arg(short, long)]
    slot: u64,

    #[arg(short, long)]
    rpc: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    dotenvy::dotenv().ok();
    let rpc_url = args
        .rpc
        .or_else(|| std::env::var("RPC_URL").ok())
        .context("RPC_URL must be provided via --rpc or .env file")?;

    // Exemple:
    // eth_contract = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
    // binance_holder = "0xF977814e90dA44bFA03b6295A0616a897441aceC";
    // mapping_slot = 3;

    let target_slot = derive_mapping_slot(&args.owner, args.slot)?;

    println!("--- SURGERY RESULT ---");
    println!("Target Slot: {}", target_slot);

    let raw_balance = get_storage_at(&rpc_url, &args.contract, &target_slot).await?;
    println!("Raw value: {}", raw_balance);

    let clean = raw_balance.trim_start_matches("0x");
    if !clean.is_empty() && clean != "0" {
        let val = u128::from_str_radix(clean, 16).unwrap_or(0);
        println!("Decimal value: {}", val);
    }
    Ok(())
}
