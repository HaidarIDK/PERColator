//! Interactive CLI with menu-driven interface for testing Percolator

use anyhow::{Context, Result};
use colored::Colorize;
use console::Term;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use solana_sdk::pubkey::Pubkey;
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
        let term = Term::stdout();
        term.clear_screen()?;
        
        show_header(config);

        let choices = &[
            "1. Setup & Deployment",
            "2. Slab Operations (Create, Manage)",
            "3. Trading (Place Orders, View Orderbook)",
            "4. Margin & Portfolio",
            "5. AMM Operations",
            "6. Liquidity Operations",
            "7. Status & Info",
            "8. Run Tests",
            "9. About",
            "Exit",
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Main Menu")
            .default(0)
            .items(&choices[..])
            .interact()?;

        match selection {
            0 => setup_workflow(config).await?,
            1 => slab_workflow(config).await?,
            2 => trading_workflow(config).await?,
            3 => margin_workflow(config).await?,
            4 => amm_workflow(config).await?,
            5 => liquidity_workflow(config).await?,
            6 => status_workflow(config).await?,
            7 => test_workflow(config).await?,
            8 => about_workflow(config).await?,
            9 => {
                println!("\n{}", "Goodbye! 👋".bright_green());
                break;
            }
            _ => {}
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

    if balance < MIN_BALANCE_LAMPORTS && (config.network == "devnet" || config.network == "localnet") {
        println!("\n{}", "⚠️  Low balance detected!".bright_yellow().bold());
        println!("{}", format!("You need at least 2 SOL for testing on {}.", config.network).bright_yellow());
        
        if Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Do you want to request an airdrop?")
            .default(true)
            .interact()?
        {
            println!("\n{}", "Run the following command in another terminal:".bright_cyan());
            println!("  {}", format!("solana airdrop 2 {}", pubkey).bright_green());
            
            pause();
        }
    } else {
        println!("\n{}", "✓ Balance sufficient".bright_green());
        // Brief pause to see the balance
        std::thread::sleep(std::time::Duration::from_millis(1000));
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

/// Setup and deployment workflow
async fn setup_workflow(config: &NetworkConfig) -> Result<()> {
    loop {
        println!();
        let choices = &[
            "Initialize Exchange (Router Registry)",
            "Initialize Portfolio (User Account)",
            "Deploy Programs",
            "Check Balance",
            "Back to Main Menu",
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Setup & Deployment")
            .default(0)
            .items(&choices[..])
            .interact()?;

        match selection {
            0 => {
                let name: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Exchange name")
                    .default("test".into())
                    .interact_text()?;
                
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
            1 => {
                println!("\n{}", "Initializing Portfolio...".bright_green());
                margin::initialize_portfolio(config).await?;
                pause();
            }
            2 => {
                println!("\n{}", "Deploy Programs...".bright_green());
                println!("{}", "Note: Deployment requires built programs.".yellow());
                println!("{}", "Run 'cargo build-sbf' first.".yellow());
                pause();
            }
            3 => {
                check_balance_and_prompt(config).await?;
                pause();
            }
            4 => break,
            _ => {}
        }
    }
    Ok(())
}

/// Slab operations workflow
async fn slab_workflow(config: &NetworkConfig) -> Result<()> {
    loop {
        println!();
        let choices = &[
            "Create New Slab",
            "Register Slab in Router",
            "View Slab Info",
            "View Orderbook",
            "Place Order on Slab",
            "Cancel Order",
            "Update Funding Rate",
            "Halt Trading",
            "Resume Trading",
            "Back to Main Menu",
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Slab Operations")
            .default(0)
            .items(&choices[..])
            .interact()?;

        match selection {
            0 => {
                let exchange: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Exchange address (or 'default')")
                    .default("default".into())
                    .interact_text()?;

                let exchange = if exchange == "default" {
                     let payer = config.pubkey();
                     Pubkey::create_with_seed(&payer, "registry", &config.router_program_id)?.to_string()
                } else {
                    exchange
                };
                
                let symbol: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Symbol")
                    .default("BTC-USD".into())
                    .interact_text()?;
                
                let tick_size: u64 = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Tick size")
                    .default(1_000_000)
                    .interact_text()?;
                
                let lot_size: u64 = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Lot size")
                    .default(1_000_000)
                    .interact_text()?;
                
                matcher::create_matcher(config, exchange, symbol, tick_size, lot_size).await?;
                pause();
            }
            1 => {
                let registry: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Registry address")
                    .interact_text()?;
                let slab_id: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Slab ID")
                    .interact_text()?;
                let oracle_id: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Oracle ID")
                    .default(config.oracle_program_id.to_string())
                    .interact_text()?;
                
                matcher::register_slab(
                    config,
                    registry,
                    slab_id,
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
            2 => {
                let slab_id: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Slab ID")
                    .interact_text()?;
                matcher::show_matcher_info(config, slab_id).await?;
                pause();
            }
            3 => {
                let slab_id: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Slab ID")
                    .interact_text()?;
                matcher::get_orderbook(config, slab_id).await?;
                pause();
            }
            4 => {
                let slab_id: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Slab ID")
                    .interact_text()?;
                
                let side_idx = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("Side")
                    .items(&["buy", "sell"])
                    .default(0)
                    .interact()?;
                let side = if side_idx == 0 { "buy" } else { "sell" };

                let price_float: f64 = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Price")
                    .interact_text()?;
                let price = (price_float * 1_000_000.0) as i64;

                let qty_float: f64 = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Quantity")
                    .interact_text()?;
                let qty = (qty_float * 1_000_000.0) as i64;
                
                let post_only = Confirm::with_theme(&ColorfulTheme::default())
                    .with_prompt("Post-only?")
                    .default(false)
                    .interact()?;
                
                let reduce_only = Confirm::with_theme(&ColorfulTheme::default())
                    .with_prompt("Reduce-only?")
                    .default(false)
                    .interact()?;
                
                matcher::place_order(
                    config,
                    slab_id,
                    side.to_string(),
                    price,
                    qty,
                    post_only,
                    reduce_only,
                ).await?;
                pause();
            }
            5 => {
                let slab_id: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Slab ID")
                    .interact_text()?;
                let order_id: u64 = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Order ID")
                    .interact_text()?;
                matcher::cancel_order(config, slab_id, order_id).await?;
                pause();
            }
            6 => {
                let slab_id: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Slab ID")
                    .interact_text()?;
                let price_float: f64 = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Oracle price")
                    .interact_text()?;
                let oracle_price = (price_float * 1_000_000.0) as i64;
                matcher::update_funding(config, slab_id, oracle_price, None).await?;
                pause();
            }
            7 => {
                let slab_id: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Slab ID")
                    .interact_text()?;
                matcher::halt_trading(config, slab_id).await?;
                pause();
            }
            8 => {
                let slab_id: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Slab ID")
                    .interact_text()?;
                matcher::resume_trading(config, slab_id).await?;
                pause();
            }
            9 => break,
            _ => {}
        }
    }
    Ok(())
}

/// Trading workflow
async fn trading_workflow(config: &NetworkConfig) -> Result<()> {
    loop {
        println!();
        let choices = &[
            "Place Limit Order (Router)",
            "Place Market Order (Router)",
            "Place Slab Order (Resting)",
            "Cancel Slab Order",
            "Modify Slab Order",
            "View Orderbook",
            "List Open Orders",
            "Back to Main Menu",
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Trading Operations")
            .default(0)
            .items(&choices[..])
            .interact()?;

        match selection {
            0 => {
                let slab: String = Input::with_theme(&ColorfulTheme::default()).with_prompt("Slab ID").interact_text()?;
                let side_idx = Select::with_theme(&ColorfulTheme::default()).with_prompt("Side").items(&["buy", "sell"]).default(0).interact()?;
                let side = if side_idx == 0 { "buy" } else { "sell" };
                let price_f: f64 = Input::with_theme(&ColorfulTheme::default()).with_prompt("Price").interact_text()?;
                let size_f: f64 = Input::with_theme(&ColorfulTheme::default()).with_prompt("Size").interact_text()?;
                let size = (size_f * 1_000_000.0) as u64;
                
                trading::place_limit_order(config, slab, side.to_string(), price_f, size, false).await?;
                pause();
            }
            1 => {
                let slab: String = Input::with_theme(&ColorfulTheme::default()).with_prompt("Slab ID").interact_text()?;
                let side_idx = Select::with_theme(&ColorfulTheme::default()).with_prompt("Side").items(&["buy", "sell"]).default(0).interact()?;
                let side = if side_idx == 0 { "buy" } else { "sell" };
                let size_f: f64 = Input::with_theme(&ColorfulTheme::default()).with_prompt("Size").interact_text()?;
                let size = (size_f * 1_000_000.0) as u64;
                
                trading::place_market_order(config, slab, side.to_string(), size).await?;
                pause();
            }
            2 => {
                let slab: String = Input::with_theme(&ColorfulTheme::default()).with_prompt("Slab ID").interact_text()?;
                let side_idx = Select::with_theme(&ColorfulTheme::default()).with_prompt("Side").items(&["buy", "sell"]).default(0).interact()?;
                let side = if side_idx == 0 { "buy" } else { "sell" };
                let price_f: f64 = Input::with_theme(&ColorfulTheme::default()).with_prompt("Price").interact_text()?;
                let size_f: f64 = Input::with_theme(&ColorfulTheme::default()).with_prompt("Size").interact_text()?;
                let size = (size_f * 1_000_000.0) as u64;
                
                trading::place_slab_order(config, slab, side.to_string(), price_f, size).await?;
                pause();
            }
            3 => {
                let slab: String = Input::with_theme(&ColorfulTheme::default()).with_prompt("Slab ID").interact_text()?;
                let order_id: u64 = Input::with_theme(&ColorfulTheme::default()).with_prompt("Order ID").interact_text()?;
                trading::cancel_slab_order(config, slab, order_id).await?;
                pause();
            }
            4 => {
                let slab: String = Input::with_theme(&ColorfulTheme::default()).with_prompt("Slab ID").interact_text()?;
                let order_id: u64 = Input::with_theme(&ColorfulTheme::default()).with_prompt("Order ID").interact_text()?;
                let price_f: f64 = Input::with_theme(&ColorfulTheme::default()).with_prompt("New Price").interact_text()?;
                let size_f: f64 = Input::with_theme(&ColorfulTheme::default()).with_prompt("New Size").interact_text()?;
                let size = (size_f * 1_000_000.0) as u64;

                trading::modify_slab_order(config, slab, order_id, price_f, size).await?;
                pause();
            }
            5 => {
                let slab: String = Input::with_theme(&ColorfulTheme::default()).with_prompt("Slab ID").interact_text()?;
                let depth: usize = Input::with_theme(&ColorfulTheme::default()).with_prompt("Depth").default(10).interact_text()?;
                trading::show_order_book(config, slab, depth).await?;
                pause();
            }
            6 => {
                let user: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("User address (optional)")
                    .allow_empty(true)
                    .interact_text()?;
                let user_opt = if user.is_empty() { None } else { Some(user) };
                trading::list_orders(config, user_opt).await?;
                pause();
            }
            7 => break,
            _ => {}
        }
    }
    Ok(())
}

/// Margin workflow
async fn margin_workflow(config: &NetworkConfig) -> Result<()> {
    loop {
        println!();
        let choices = &[
            "Initialize Portfolio",
            "Deposit Collateral",
            "Withdraw Collateral",
            "View Portfolio",
            "View Margin Requirements",
            "Back to Main Menu",
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Margin & Portfolio")
            .default(0)
            .items(&choices[..])
            .interact()?;

        match selection {
            0 => {
                margin::initialize_portfolio(config).await?;
                pause();
            }
            1 => {
                let amount_f: f64 = Input::with_theme(&ColorfulTheme::default()).with_prompt("Amount in SOL").interact_text()?;
                let amount = (amount_f * 1_000_000_000.0) as u64;
                margin::deposit_collateral(config, amount, None).await?;
                pause();
            }
            2 => {
                let amount_f: f64 = Input::with_theme(&ColorfulTheme::default()).with_prompt("Amount in SOL").interact_text()?;
                let amount = (amount_f * 1_000_000_000.0) as u64;
                margin::withdraw_collateral(config, amount, None).await?;
                pause();
            }
            3 => {
                let user: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("User address (optional)")
                    .allow_empty(true)
                    .interact_text()?;
                let user_opt = if user.is_empty() { None } else { Some(user) };
                margin::show_margin_account(config, user_opt).await?;
                pause();
            }
            4 => {
                let user: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("User address (optional)")
                    .allow_empty(true)
                    .interact_text()?;
                let user = if user.is_empty() { config.pubkey().to_string() } else { user };
                margin::show_margin_requirements(config, user).await?;
                pause();
            }
            5 => break,
            _ => {}
        }
    }
    Ok(())
}

/// AMM workflow
async fn amm_workflow(config: &NetworkConfig) -> Result<()> {
    loop {
        println!();
        let choices = &[
            "Create AMM Pool",
            "List AMM Pools",
            "Back to Main Menu",
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("AMM Operations")
            .default(0)
            .items(&choices[..])
            .interact()?;

        match selection {
            0 => {
                let registry: String = Input::with_theme(&ColorfulTheme::default()).with_prompt("Registry address").interact_text()?;
                let symbol: String = Input::with_theme(&ColorfulTheme::default()).with_prompt("Symbol").interact_text()?;
                let x_reserve: u64 = Input::with_theme(&ColorfulTheme::default()).with_prompt("X reserve").default(1_000_000).interact_text()?;
                let y_reserve: u64 = Input::with_theme(&ColorfulTheme::default()).with_prompt("Y reserve").default(1_000_000).interact_text()?;
                
                amm::create_amm(config, registry, symbol, x_reserve, y_reserve).await?;
                pause();
            }
            1 => {
                amm::list_amms(config).await?;
                pause();
            }
            2 => break,
            _ => {}
        }
    }
    Ok(())
}

/// Liquidity workflow
async fn liquidity_workflow(config: &NetworkConfig) -> Result<()> {
    loop {
        println!();
        let choices = &[
            "Add Liquidity",
            "Remove Liquidity",
            "Show Positions",
            "Back to Main Menu",
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Liquidity Operations")
            .default(0)
            .items(&choices[..])
            .interact()?;

        match selection {
            0 => {
                let matcher: String = Input::with_theme(&ColorfulTheme::default()).with_prompt("Matcher address").interact_text()?;
                let amount: u64 = Input::with_theme(&ColorfulTheme::default()).with_prompt("Amount").interact_text()?;
                
                let mode_idx = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("Mode")
                    .items(&["amm", "orderbook"])
                    .default(0)
                    .interact()?;
                let mode = if mode_idx == 0 { "amm".to_string() } else { "orderbook".to_string() };
                
                liquidity::add_liquidity(
                    config,
                    matcher,
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
            1 => {
                let matcher: String = Input::with_theme(&ColorfulTheme::default()).with_prompt("Matcher address").interact_text()?;
                let amount: u64 = Input::with_theme(&ColorfulTheme::default()).with_prompt("Amount").interact_text()?;
                liquidity::remove_liquidity(config, matcher, amount).await?;
                pause();
            }
            2 => {
                let user: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("User address (optional)")
                    .allow_empty(true)
                    .interact_text()?;
                let user_opt = if user.is_empty() { None } else { Some(user) };
                liquidity::show_positions(config, user_opt).await?;
                pause();
            }
            3 => break,
            _ => {}
        }
    }
    Ok(())
}

/// Status workflow
async fn status_workflow(config: &NetworkConfig) -> Result<()> {
    loop {
        println!();
        let choices = &[
            "View Registry Status",
            "View Portfolio",
            "Check Balance",
            "Back to Main Menu",
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Status & Info")
            .default(0)
            .items(&choices[..])
            .interact()?;

        match selection {
            0 => {
                let registry: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Registry address (or 'default')")
                    .default("default".into())
                    .interact_text()?;

                let registry = if registry == "default" {
                     let payer = config.pubkey();
                     Pubkey::create_with_seed(&payer, "registry", &config.router_program_id)?.to_string()
                } else {
                    registry
                };
                exchange::query_registry_status(config, registry, true).await?;
                pause();
            }
            1 => {
                let user: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("User address (optional)")
                    .allow_empty(true)
                    .interact_text()?;
                let user_opt = if user.is_empty() { None } else { Some(user) };
                margin::show_margin_account(config, user_opt).await?;
                pause();
            }
            2 => {
                check_balance_and_prompt(config).await?;
                pause();
            }
            3 => break,
            _ => {}
        }
    }
    Ok(())
}

/// About workflow
async fn about_workflow(_config: &NetworkConfig) -> Result<()> {
    let term = Term::stdout();
    term.clear_screen()?;
    println!("{}", "=== About Percolator ===".bright_green().bold());
    println!();
    println!("Percolator is a high-performance decentralized exchange (DEX) protocol");
    println!("built on Solana, featuring:");
    println!();
    println!("  • {}", "Hybrid Liquidity".bright_cyan());
    println!("    Combines AMM and Orderbook liquidity in a single pool.");
    println!();
    println!("  • {}", "Cross-Slab Routing".bright_cyan());
    println!("    Smart router splits orders across multiple liquidity sources.");
    println!();
    println!("  • {}", "Risk Management".bright_cyan());
    println!("    Advanced margin system with real-time liquidation engine.");
    println!();
    println!("  • {}", "Developer Tools".bright_cyan());
    println!("    Comprehensive CLI and testing suite for easy integration.");
    println!();
    println!("Version: {}", env!("CARGO_PKG_VERSION").bright_yellow());
    println!();
    pause();
    Ok(())
}

/// Test workflow
async fn test_workflow(_config: &NetworkConfig) -> Result<()> {
    println!("\n{}", "=== Run Tests ===".bright_green().bold());
    println!();
    println!("{}", "Note: Tests are run via the 'test' command.".yellow());
    println!("{}", "Example: percolator -n devnet test --all".bright_cyan());
    println!();
    pause();
    Ok(())
}

fn pause() {
    println!("\n{}", "Press Enter to continue...".dimmed());
    let _ = std::io::stdin().read_line(&mut String::new());
}
