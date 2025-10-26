//! Utility functions

use crate::{client, config::Config, Result};
use console::style;
use indicatif::ProgressBar;
use solana_client::rpc_client::RpcClient;
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_sdk::signer::Signer;

/// Check SOL balance
pub async fn check_balance(config: &Config) -> Result<()> {
    let pb = ProgressBar::new_spinner();
    pb.set_message("Checking balance...");

    let wallet = client::load_wallet(&config.wallet_path)?;
    let rpc_client = RpcClient::new(&config.rpc_url);

    let balance = rpc_client.get_balance(&wallet.pubkey())?;
    let sol = balance as f64 / LAMPORTS_PER_SOL as f64;

    pb.finish_and_clear();

    println!("\n{}", style("💰 Wallet Balance").cyan().bold());
    println!("{}", style("─".repeat(60)).dim());
    println!("Address: {}", wallet.pubkey());
    println!("Balance: {} SOL", style(format!("{:.4}", sol)).green().bold());

    if sol < 0.1 {
        println!("\n{}", style("⚠️  Low balance! Request airdrop:").yellow());
        println!("{}", style("  perc airdrop").dim());
    }

    Ok(())
}

/// Request devnet SOL airdrop
pub async fn request_airdrop(config: &Config, amount: f64) -> Result<()> {
    let pb = ProgressBar::new_spinner();
    pb.set_message("Requesting airdrop...");

    let wallet = client::load_wallet(&config.wallet_path)?;
    let rpc_client = RpcClient::new(&config.rpc_url);

    let signature = rpc_client.request_airdrop(
        &wallet.pubkey(),
        (amount * LAMPORTS_PER_SOL as f64) as u64,
    )?;

    // Wait for confirmation
    rpc_client.confirm_transaction(&signature)?;

    pb.finish_with_message(format!("{} Airdropped {} SOL", style("✅").green(), amount));
    println!("{}", style(format!("Signature: {}", signature)).dim());

    Ok(())
}

