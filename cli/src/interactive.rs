//! Interactive CLI with menu-driven interface for testing Percolator

use anyhow::{Context, Result};
use colored::Colorize;
use solana_sdk::pubkey::Pubkey;
use std::io::{self, Write};
use std::str::FromStr;

use crate::config::NetworkConfig;
use crate::{amm, client, exchange, liquidity, margin, matcher, trading};

/// Minimum SOL balance required (in lamports) - 2 SOL for testing
const MIN_BALANCE_LAMPORTS: u64 = 2_000_000_000;

/// Run interactive CLI menu
pub async fn run_interactive(config: &NetworkConfig) -> Result<()> {
    // Check SOL balance first
    check_balance_and_prompt(config).await?;

    loop {
        clear_screen();
        show_header(config);
        show_main_menu();

        let choice = read_input("Enter your choice: ")?;

        match choice.trim() {
            "1" => {
                setup_workflow(config).await?;
            }
            "2" => {
                slab_workflow(config).await?;
            }
            "3" => {
                trading_workflow(config).await?;
            }
            "4" => {
                margin_workflow(config).await?;
            }
            "5" => {
                amm_workflow(config).await?;
            }
            "6" => {
                liquidity_workflow(config).await?;
            }
            "7" => {
                status_workflow(config).await?;
            }
            "8" => {
                test_workflow(config).await?;
            }
            "0" | "q" | "quit" | "exit" => {
                println!("\n{}", "Goodbye! 👋".bright_green());
                break;
            }
            _ => {
                println!("\n{} Invalid choice. Press Enter to continue...", "⚠".yellow());
                let _ = read_input("");
            }
        }
    }

    Ok(())
}

/// Check SOL balance and prompt for airdrop if needed
async fn check_balance_and_prompt(config: &NetworkConfig) -> Result<()> {
    let rpc_client = client::create_rpc_client(config);
    let pubkey = config.pubkey();
    
    let balance = rpc_client
        .get_balance(&pubkey)
        .context("Failed to get balance")?;

    let balance_sol = balance as f64 / 1_000_000_000.0;

    println!("\n{}", "=== Balance Check ===".bright_cyan().bold());
    println!("{} {}", "Address:".bright_cyan(), pubkey);
    println!("{} {:.4} SOL", "Balance:".bright_cyan(), balance_sol);

    if balance < MIN_BALANCE_LAMPORTS && config.network == "devnet" {
        println!("\n{}", "⚠️  Low balance detected!".bright_yellow().bold());
        println!("{}", "You need at least 2 SOL for testing on devnet.".bright_yellow());
        println!("\n{}", "To get devnet SOL:".bright_cyan());
        println!("  {}", "solana airdrop 2".bright_green());
        println!("  {}", format!("solana airdrop 2 {}", pubkey).bright_green());
        println!("\n{}", "Press Enter after airdropping to continue...".dimmed());
        let _ = read_input("");
    } else if balance < MIN_BALANCE_LAMPORTS && config.network == "localnet" {
        println!("\n{}", "⚠️  Low balance detected!".bright_yellow().bold());
        println!("{}", "You need at least 2 SOL for testing on localnet.".bright_yellow());
        println!("\n{}", "To get localnet SOL:".bright_cyan());
        println!("  {}", "solana airdrop 2".bright_green());
        println!("  {}", format!("solana airdrop 2 {}", pubkey).bright_green());
        println!("\n{}", "Press Enter after airdropping to continue...".dimmed());
        let _ = read_input("");
    } else {
        println!("\n{}", "✓ Balance sufficient".bright_green());
    }

    Ok(())
}

/// Show header with network info
fn show_header(config: &NetworkConfig) {
    println!("{}", "╔═══════════════════════════════════════════════════════╗".bright_cyan());
    println!("{}", "║       Percolator Protocol - Interactive CLI          ║".bright_cyan().bold());
    println!("{}", "╚═══════════════════════════════════════════════════════╝".bright_cyan());
    println!();
    println!("{} {}", "Network:".bright_cyan(), config.network.bright_yellow());
    println!("{} {}", "Address:".bright_cyan(), config.pubkey().to_string().bright_yellow());
    println!();
}

/// Show main menu
fn show_main_menu() {
    println!("{}", "Main Menu:".bright_yellow().bold());
    println!();
    println!("  {}  Setup & Deployment", "1.".bright_cyan());
    println!("  {}  Slab Operations (Create, Manage)", "2.".bright_cyan());
    println!("  {}  Trading (Place Orders, View Orderbook)", "3.".bright_cyan());
    println!("  {}  Margin & Portfolio", "4.".bright_cyan());
    println!("  {}  AMM Operations", "5.".bright_cyan());
    println!("  {}  Liquidity Operations", "6.".bright_cyan());
    println!("  {}  Status & Info", "7.".bright_cyan());
    println!("  {}  Run Tests", "8.".bright_cyan());
    println!();
    println!("  {}  Quit", "0.".bright_cyan());
    println!();
}

/// Setup and deployment workflow
async fn setup_workflow(config: &NetworkConfig) -> Result<()> {
    loop {
        clear_screen();
        println!("{}", "=== Setup & Deployment ===".bright_green().bold());
        println!();
        println!("  {}  Initialize Exchange (Router Registry)", "1.".bright_cyan());
        println!("  {}  Initialize Portfolio (User Account)", "2.".bright_cyan());
        println!("  {}  Deploy Programs", "3.".bright_cyan());
        println!("  {}  Check Balance", "4.".bright_cyan());
        println!("  {}  Back to Main Menu", "0.".bright_cyan());
        println!();

        let choice = read_input("Enter your choice: ")?;

        match choice.trim() {
            "1" => {
                println!("\n{}", "Initializing Exchange...".bright_green());
                let name = read_input("Exchange name (default: 'test'): ")?;
                let name = if name.trim().is_empty() {
                    "test".to_string()
                } else {
                    name.trim().to_string()
                };
                
                exchange::initialize_exchange(
                    config,
                    name,
                    1_000_000_000, // 1 SOL insurance fund
                    500,           // 5% maintenance margin
                    1000,          // 10% initial margin
                    None,          // Default insurance authority
                ).await?;
                
                pause();
            }
            "2" => {
                println!("\n{}", "Initializing Portfolio...".bright_green());
                margin::initialize_portfolio(config).await?;
                pause();
            }
            "3" => {
                println!("\n{}", "Deploy Programs...".bright_green());
                println!("{}", "Note: Deployment requires built programs.".yellow());
                println!("{}", "Run 'cargo build-sbf' first.".yellow());
                pause();
            }
            "4" => {
                check_balance_and_prompt(config).await?;
                pause();
            }
            "0" => break,
            _ => {
                println!("\n{} Invalid choice.", "⚠".yellow());
                pause();
            }
        }
    }
    Ok(())
}

/// Slab operations workflow
async fn slab_workflow(config: &NetworkConfig) -> Result<()> {
    loop {
        clear_screen();
        println!("{}", "=== Slab Operations ===".bright_green().bold());
        println!();
        println!("  {}  Create New Slab", "1.".bright_cyan());
        println!("  {}  Register Slab in Router", "2.".bright_cyan());
        println!("  {}  View Slab Info", "3.".bright_cyan());
        println!("  {}  View Orderbook", "4.".bright_cyan());
        println!("  {}  Place Order on Slab", "5.".bright_cyan());
        println!("  {}  Cancel Order", "6.".bright_cyan());
        println!("  {}  Update Funding Rate", "7.".bright_cyan());
        println!("  {}  Halt Trading", "8.".bright_cyan());
        println!("  {}  Resume Trading", "9.".bright_cyan());
        println!("  {}  Back to Main Menu", "0.".bright_cyan());
        println!();

        let choice = read_input("Enter your choice: ")?;

        match choice.trim() {
            "1" => {
                println!("\n{}", "Create New Slab".bright_green());
                let exchange = read_input("Exchange address (or 'default' for test): ")?;
                let exchange = if exchange.trim() == "default" || exchange.trim().is_empty() {
                    // Derive default registry
                    let payer = config.pubkey();
                    Pubkey::create_with_seed(
                        &payer,
                        "registry",
                        &config.router_program_id,
                    )?.to_string()
                } else {
                    exchange.trim().to_string()
                };
                
                let symbol = read_input("Symbol (e.g., BTC-USD): ")?;
                let symbol = if symbol.trim().is_empty() {
                    "BTC-USD".to_string()
                } else {
                    symbol.trim().to_string()
                };
                
                let tick_size_str = read_input("Tick size (default: 1000000 = $1): ")?;
                let tick_size = if tick_size_str.trim().is_empty() {
                    1_000_000
                } else {
                    tick_size_str.trim().parse().context("Invalid tick size")?
                };
                
                let lot_size_str = read_input("Lot size (default: 1000000 = 1.0): ")?;
                let lot_size = if lot_size_str.trim().is_empty() {
                    1_000_000
                } else {
                    lot_size_str.trim().parse().context("Invalid lot size")?
                };
                
                matcher::create_matcher(config, exchange, symbol, tick_size, lot_size).await?;
                pause();
            }
            "2" => {
                println!("\n{}", "Register Slab in Router".bright_green());
                let registry = read_input("Registry address: ")?;
                let slab_id = read_input("Slab ID: ")?;
                let oracle_id = read_input("Oracle ID (or 'default'): ")?;
                let oracle_id = if oracle_id.trim() == "default" || oracle_id.trim().is_empty() {
                    config.oracle_program_id.to_string()
                } else {
                    oracle_id.trim().to_string()
                };
                
                matcher::register_slab(
                    config,
                    registry.trim().to_string(),
                    slab_id.trim().to_string(),
                    oracle_id,
                    500,  // 5% IMR
                    300,  // 3% MMR
                    10,   // 10 bps maker fee
                    20,   // 20 bps taker fee
                    100,  // 100ms latency SLA
                    1_000_000_000_000, // Max exposure
                ).await?;
                pause();
            }
            "3" => {
                println!("\n{}", "View Slab Info".bright_green());
                let slab_id = read_input("Slab ID: ")?;
                matcher::show_matcher_info(config, slab_id.trim().to_string()).await?;
                pause();
            }
            "4" => {
                println!("\n{}", "View Orderbook".bright_green());
                let slab_id = read_input("Slab ID: ")?;
                matcher::get_orderbook(config, slab_id.trim().to_string()).await?;
                pause();
            }
            "5" => {
                println!("\n{}", "Place Order on Slab".bright_green());
                let slab_id = read_input("Slab ID: ")?;
                let side = read_input("Side (buy/sell): ")?;
                let price_str = read_input("Price (e.g., 100.0): ")?;
                let price = (price_str.trim().parse::<f64>().context("Invalid price")? * 1_000_000.0) as i64;
                let qty_str = read_input("Quantity (e.g., 1.0): ")?;
                let qty = (qty_str.trim().parse::<f64>().context("Invalid quantity")? * 1_000_000.0) as i64;
                
                let post_only = read_input("Post-only? (y/n): ")?;
                let post_only = post_only.trim().to_lowercase() == "y";
                
                let reduce_only = read_input("Reduce-only? (y/n): ")?;
                let reduce_only = reduce_only.trim().to_lowercase() == "y";
                
                matcher::place_order(
                    config,
                    slab_id.trim().to_string(),
                    side.trim().to_string(),
                    price,
                    qty,
                    post_only,
                    reduce_only,
                ).await?;
                pause();
            }
            "6" => {
                println!("\n{}", "Cancel Order".bright_green());
                let slab_id = read_input("Slab ID: ")?;
                let order_id_str = read_input("Order ID: ")?;
                let order_id = order_id_str.trim().parse().context("Invalid order ID")?;
                matcher::cancel_order(config, slab_id.trim().to_string(), order_id).await?;
                pause();
            }
            "7" => {
                println!("\n{}", "Update Funding Rate".bright_green());
                let slab_id = read_input("Slab ID: ")?;
                let price_str = read_input("Oracle price (e.g., 100.0): ")?;
                let oracle_price = (price_str.trim().parse::<f64>().context("Invalid price")? * 1_000_000.0) as i64;
                matcher::update_funding(config, slab_id.trim().to_string(), oracle_price, None).await?;
                pause();
            }
            "8" => {
                println!("\n{}", "Halt Trading".bright_red());
                let slab_id = read_input("Slab ID: ")?;
                matcher::halt_trading(config, slab_id.trim().to_string()).await?;
                pause();
            }
            "9" => {
                println!("\n{}", "Resume Trading".bright_green());
                let slab_id = read_input("Slab ID: ")?;
                matcher::resume_trading(config, slab_id.trim().to_string()).await?;
                pause();
            }
            "0" => break,
            _ => {
                println!("\n{} Invalid choice.", "⚠".yellow());
                pause();
            }
        }
    }
    Ok(())
}

/// Trading workflow
async fn trading_workflow(config: &NetworkConfig) -> Result<()> {
    loop {
        clear_screen();
        println!("{}", "=== Trading Operations ===".bright_green().bold());
        println!();
        println!("  {}  Place Limit Order (Router)", "1.".bright_cyan());
        println!("  {}  Place Market Order (Router)", "2.".bright_cyan());
        println!("  {}  Place Slab Order (Resting)", "3.".bright_cyan());
        println!("  {}  Cancel Slab Order", "4.".bright_cyan());
        println!("  {}  Modify Slab Order", "5.".bright_cyan());
        println!("  {}  View Orderbook", "6.".bright_cyan());
        println!("  {}  List Open Orders", "7.".bright_cyan());
        println!("  {}  Back to Main Menu", "0.".bright_cyan());
        println!();

        let choice = read_input("Enter your choice: ")?;

        match choice.trim() {
            "1" => {
                println!("\n{}", "Place Limit Order (Router)".bright_green());
                let slab = read_input("Slab ID: ")?;
                let side = read_input("Side (buy/sell): ")?;
                let price_str = read_input("Price (e.g., 100.0): ")?;
                let price = price_str.trim().parse().context("Invalid price")?;
                let size_str = read_input("Size (e.g., 1.0): ")?;
                let size = (size_str.trim().parse::<f64>().context("Invalid size")? * 1_000_000.0) as u64;
                
                trading::place_limit_order(config, slab.trim().to_string(), side.trim().to_string(), price, size, false).await?;
                pause();
            }
            "2" => {
                println!("\n{}", "Place Market Order (Router)".bright_green());
                let slab = read_input("Slab ID: ")?;
                let side = read_input("Side (buy/sell): ")?;
                let size_str = read_input("Size (e.g., 1.0): ")?;
                let size = (size_str.trim().parse::<f64>().context("Invalid size")? * 1_000_000.0) as u64;
                
                trading::place_market_order(config, slab.trim().to_string(), side.trim().to_string(), size).await?;
                pause();
            }
            "3" => {
                println!("\n{}", "Place Slab Order (Resting)".bright_green());
                let slab = read_input("Slab ID: ")?;
                let side = read_input("Side (buy/sell): ")?;
                let price_str = read_input("Price (e.g., 100.0): ")?;
                let price = price_str.trim().parse().context("Invalid price")?;
                let size_str = read_input("Size (e.g., 1.0): ")?;
                let size = (size_str.trim().parse::<f64>().context("Invalid size")? * 1_000_000.0) as u64;
                
                trading::place_slab_order(config, slab.trim().to_string(), side.trim().to_string(), price, size).await?;
                pause();
            }
            "4" => {
                println!("\n{}", "Cancel Slab Order".bright_green());
                let slab = read_input("Slab ID: ")?;
                let order_id_str = read_input("Order ID: ")?;
                let order_id = order_id_str.trim().parse().context("Invalid order ID")?;
                trading::cancel_slab_order(config, slab.trim().to_string(), order_id).await?;
                pause();
            }
            "5" => {
                println!("\n{}", "Modify Slab Order".bright_green());
                let slab = read_input("Slab ID: ")?;
                let order_id_str = read_input("Order ID: ")?;
                let order_id = order_id_str.trim().parse().context("Invalid order ID")?;
                let price_str = read_input("New price (e.g., 100.0): ")?;
                let price = price_str.trim().parse().context("Invalid price")?;
                let size_str = read_input("New size (e.g., 1.0): ")?;
                let size = (size_str.trim().parse::<f64>().context("Invalid size")? * 1_000_000.0) as u64;
                
                trading::modify_slab_order(config, slab.trim().to_string(), order_id, price, size).await?;
                pause();
            }
            "6" => {
                println!("\n{}", "View Orderbook".bright_green());
                let slab = read_input("Slab ID: ")?;
                let depth_str = read_input("Depth (default: 10): ")?;
                let depth = if depth_str.trim().is_empty() {
                    10
                } else {
                    depth_str.trim().parse().context("Invalid depth")?
                };
                trading::show_order_book(config, slab.trim().to_string(), depth).await?;
                pause();
            }
            "7" => {
                println!("\n{}", "List Open Orders".bright_green());
                let user = read_input("User address (or press Enter for self): ")?;
                let user = if user.trim().is_empty() {
                    None
                } else {
                    Some(user.trim().to_string())
                };
                trading::list_orders(config, user).await?;
                pause();
            }
            "0" => break,
            _ => {
                println!("\n{} Invalid choice.", "⚠".yellow());
                pause();
            }
        }
    }
    Ok(())
}

/// Margin workflow
async fn margin_workflow(config: &NetworkConfig) -> Result<()> {
    loop {
        clear_screen();
        println!("{}", "=== Margin & Portfolio ===".bright_green().bold());
        println!();
        println!("  {}  Initialize Portfolio", "1.".bright_cyan());
        println!("  {}  Deposit Collateral", "2.".bright_cyan());
        println!("  {}  Withdraw Collateral", "3.".bright_cyan());
        println!("  {}  View Portfolio", "4.".bright_cyan());
        println!("  {}  View Margin Requirements", "5.".bright_cyan());
        println!("  {}  Back to Main Menu", "0.".bright_cyan());
        println!();

        let choice = read_input("Enter your choice: ")?;

        match choice.trim() {
            "1" => {
                margin::initialize_portfolio(config).await?;
                pause();
            }
            "2" => {
                println!("\n{}", "Deposit Collateral".bright_green());
                let amount_str = read_input("Amount in SOL (e.g., 1.0): ")?;
                let amount = (amount_str.trim().parse::<f64>().context("Invalid amount")? * 1_000_000_000.0) as u64;
                margin::deposit_collateral(config, amount, None).await?;
                pause();
            }
            "3" => {
                println!("\n{}", "Withdraw Collateral".bright_green());
                let amount_str = read_input("Amount in SOL (e.g., 0.5): ")?;
                let amount = (amount_str.trim().parse::<f64>().context("Invalid amount")? * 1_000_000_000.0) as u64;
                margin::withdraw_collateral(config, amount, None).await?;
                pause();
            }
            "4" => {
                let user = read_input("User address (or press Enter for self): ")?;
                let user = if user.trim().is_empty() {
                    None
                } else {
                    Some(user.trim().to_string())
                };
                margin::show_margin_account(config, user).await?;
                pause();
            }
            "5" => {
                let user = read_input("User address (or press Enter for self): ")?;
                let user = if user.trim().is_empty() {
                    config.pubkey().to_string()
                } else {
                    user.trim().to_string()
                };
                margin::show_margin_requirements(config, user).await?;
                pause();
            }
            "0" => break,
            _ => {
                println!("\n{} Invalid choice.", "⚠".yellow());
                pause();
            }
        }
    }
    Ok(())
}

/// AMM workflow
async fn amm_workflow(config: &NetworkConfig) -> Result<()> {
    loop {
        clear_screen();
        println!("{}", "=== AMM Operations ===".bright_green().bold());
        println!();
        println!("  {}  Create AMM Pool", "1.".bright_cyan());
        println!("  {}  List AMM Pools", "2.".bright_cyan());
        println!("  {}  Back to Main Menu", "0.".bright_cyan());
        println!();

        let choice = read_input("Enter your choice: ")?;

        match choice.trim() {
            "1" => {
                println!("\n{}", "Create AMM Pool".bright_green());
                let registry = read_input("Registry address: ")?;
                let symbol = read_input("Symbol (e.g., BTC-USD): ")?;
                let x_reserve_str = read_input("X reserve (default: 1000000): ")?;
                let x_reserve = if x_reserve_str.trim().is_empty() {
                    1_000_000
                } else {
                    x_reserve_str.trim().parse().context("Invalid reserve")?
                };
                let y_reserve_str = read_input("Y reserve (default: 1000000): ")?;
                let y_reserve = if y_reserve_str.trim().is_empty() {
                    1_000_000
                } else {
                    y_reserve_str.trim().parse().context("Invalid reserve")?
                };
                
                amm::create_amm(config, registry.trim().to_string(), symbol.trim().to_string(), x_reserve, y_reserve).await?;
                pause();
            }
            "2" => {
                amm::list_amms(config).await?;
                pause();
            }
            "0" => break,
            _ => {
                println!("\n{} Invalid choice.", "⚠".yellow());
                pause();
            }
        }
    }
    Ok(())
}

/// Liquidity workflow
async fn liquidity_workflow(config: &NetworkConfig) -> Result<()> {
    loop {
        clear_screen();
        println!("{}", "=== Liquidity Operations ===".bright_green().bold());
        println!();
        println!("  {}  Add Liquidity", "1.".bright_cyan());
        println!("  {}  Remove Liquidity", "2.".bright_cyan());
        println!("  {}  Show Positions", "3.".bright_cyan());
        println!("  {}  Back to Main Menu", "0.".bright_cyan());
        println!();

        let choice = read_input("Enter your choice: ")?;

        match choice.trim() {
            "1" => {
                println!("\n{}", "Add Liquidity".bright_green());
                let matcher = read_input("Matcher address: ")?;
                let amount_str = read_input("Amount: ")?;
                let amount = amount_str.trim().parse().context("Invalid amount")?;
                let mode = read_input("Mode (amm/orderbook, default: amm): ")?;
                let mode = if mode.trim().is_empty() {
                    "amm".to_string()
                } else {
                    mode.trim().to_string()
                };
                
                liquidity::add_liquidity(
                    config,
                    matcher.trim().to_string(),
                    amount,
                    None,
                    mode,
                    None,
                    false,
                    false,
                    None,
                    None,
                ).await?;
                pause();
            }
            "2" => {
                println!("\n{}", "Remove Liquidity".bright_green());
                let matcher = read_input("Matcher address: ")?;
                let amount_str = read_input("Amount: ")?;
                let amount = amount_str.trim().parse().context("Invalid amount")?;
                liquidity::remove_liquidity(config, matcher.trim().to_string(), amount).await?;
                pause();
            }
            "3" => {
                let user = read_input("User address (or press Enter for self): ")?;
                let user = if user.trim().is_empty() {
                    None
                } else {
                    Some(user.trim().to_string())
                };
                liquidity::show_positions(config, user).await?;
                pause();
            }
            "0" => break,
            _ => {
                println!("\n{} Invalid choice.", "⚠".yellow());
                pause();
            }
        }
    }
    Ok(())
}

/// Status workflow
async fn status_workflow(config: &NetworkConfig) -> Result<()> {
    loop {
        clear_screen();
        println!("{}", "=== Status & Info ===".bright_green().bold());
        println!();
        println!("  {}  View Registry Status", "1.".bright_cyan());
        println!("  {}  View Portfolio", "2.".bright_cyan());
        println!("  {}  Check Balance", "3.".bright_cyan());
        println!("  {}  Back to Main Menu", "0.".bright_cyan());
        println!();

        let choice = read_input("Enter your choice: ")?;

        match choice.trim() {
            "1" => {
                let registry = read_input("Registry address (or 'default'): ")?;
                let registry = if registry.trim() == "default" || registry.trim().is_empty() {
                    let payer = config.pubkey();
                    Pubkey::create_with_seed(
                        &payer,
                        "registry",
                        &config.router_program_id,
                    )?.to_string()
                } else {
                    registry.trim().to_string()
                };
                exchange::query_registry_status(config, registry, true).await?;
                pause();
            }
            "2" => {
                let user = read_input("User address (or press Enter for self): ")?;
                let user = if user.trim().is_empty() {
                    None
                } else {
                    Some(user.trim().to_string())
                };
                margin::show_margin_account(config, user).await?;
                pause();
            }
            "3" => {
                check_balance_and_prompt(config).await?;
                pause();
            }
            "0" => break,
            _ => {
                println!("\n{} Invalid choice.", "⚠".yellow());
                pause();
            }
        }
    }
    Ok(())
}

/// Test workflow
async fn test_workflow(config: &NetworkConfig) -> Result<()> {
    println!("\n{}", "=== Run Tests ===".bright_green().bold());
    println!();
    println!("{}", "Note: Tests are run via the 'test' command.".yellow());
    println!("{}", "Example: percolator -n devnet test --all".bright_cyan());
    println!();
    pause();
    Ok(())
}

/// Utility functions

fn clear_screen() {
    // Try ANSI escape codes first (works on modern Windows terminals)
    print!("\x1B[2J\x1B[1;1H");
    io::stdout().flush().ok();
    
    // On Windows, also try cls command as fallback
    #[cfg(windows)]
    {
        use std::process::Command;
        let _ = Command::new("cmd").args(["/C", "cls"]).status();
    }
}

fn read_input(prompt: &str) -> Result<String> {
    print!("{}", prompt.bright_cyan());
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input)
}

fn pause() {
    println!("\n{}", "Press Enter to continue...".dimmed());
    let _ = read_input("");
}

