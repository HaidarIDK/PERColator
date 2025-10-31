//! Percolator CLI - Comprehensive testing and deployment tool
//!
//! This CLI provides end-to-end testing and deployment of the Percolator
//! perpetual exchange protocol on Solana networks (localnet, devnet, mainnet).

use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;

mod config;
mod client;
mod deploy;
mod exchange;
mod matcher;
mod liquidity;
mod trading;
mod margin;
mod liquidation;
mod insurance;
mod amm;
mod crisis;
mod keeper;
mod tests;
mod tests_funding;

use config::NetworkConfig;

#[derive(Parser)]
#[command(name = "percolator")]
#[command(about = "Percolator Protocol CLI - Deploy and test perpetual exchange", long_about = None)]
#[command(version)]
struct Cli {
    /// Network to connect to (localnet, devnet, mainnet-beta)
    #[arg(short, long, default_value = "localnet")]
    network: String,

    /// RPC URL (overrides network default)
    #[arg(short, long)]
    url: Option<String>,

    /// Path to keypair file
    #[arg(short, long)]
    keypair: Option<PathBuf>,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Deploy programs to the network
    Deploy {
        /// Deploy router program
        #[arg(long)]
        router: bool,

        /// Deploy slab (matcher) program
        #[arg(long)]
        slab: bool,

        /// Deploy AMM program
        #[arg(long)]
        amm: bool,

        /// Deploy oracle program
        #[arg(long)]
        oracle: bool,

        /// Deploy all programs
        #[arg(long)]
        all: bool,

        /// Program keypair file (for upgradeable deploys)
        #[arg(long)]
        program_keypair: Option<PathBuf>,
    },

    /// Initialize a new perp exchange
    Init {
        /// Exchange name/identifier
        #[arg(short, long)]
        name: String,

        /// Insurance fund initial balance (lamports)
        #[arg(short, long, default_value = "1000000000")]
        insurance_fund: u64,

        /// Maintenance margin ratio (basis points)
        #[arg(short, long, default_value = "500")]
        maintenance_margin: u16,

        /// Initial margin ratio (basis points)
        #[arg(long, default_value = "1000")]
        initial_margin: u16,
    },

    /// Matcher/slab operations
    Matcher {
        #[command(subcommand)]
        command: MatcherCommands,
    },

    /// Liquidity operations
    Liquidity {
        #[command(subcommand)]
        command: LiquidityCommands,
    },

    /// AMM operations
    Amm {
        #[command(subcommand)]
        command: AmmCommands,
    },

    /// Trading operations
    Trade {
        #[command(subcommand)]
        command: TradeCommands,
    },

    /// Margin operations
    Margin {
        #[command(subcommand)]
        command: MarginCommands,
    },

    /// Liquidation operations
    Liquidation {
        #[command(subcommand)]
        command: LiquidationCommands,
    },

    /// Insurance fund operations
    Insurance {
        #[command(subcommand)]
        command: InsuranceCommands,
    },

    /// Crisis simulation and testing
    Crisis {
        #[command(subcommand)]
        command: CrisisCommands,
    },

    /// Keeper operations
    Keeper {
        #[command(subcommand)]
        command: KeeperCommands,
    },

    /// Run end-to-end test suite
    Test {
        /// Run quick smoke tests only
        #[arg(long)]
        quick: bool,

        /// Run margin system tests
        #[arg(long)]
        margin: bool,

        /// Run order management tests
        #[arg(long)]
        orders: bool,

        /// Run trade matching tests
        #[arg(long)]
        matching: bool,

        /// Run liquidation tests
        #[arg(long)]
        liquidations: bool,

        /// Run multi-slab routing tests
        #[arg(long)]
        routing: bool,

        /// Run capital efficiency tests
        #[arg(long)]
        capital_efficiency: bool,

        /// Run crisis haircut tests
        #[arg(long)]
        crisis: bool,

        /// Run LP insolvency tests
        #[arg(long)]
        lp_insolvency: bool,

        /// Run funding mechanics tests
        #[arg(long)]
        funding: bool,

        /// Run all tests
        #[arg(long)]
        all: bool,
    },

    /// Show protocol status and statistics
    Status {
        /// Exchange address
        exchange: String,

        /// Show detailed statistics
        #[arg(short, long)]
        detailed: bool,
    },
}

#[derive(Subcommand)]
enum MatcherCommands {
    /// Create a new matcher/slab
    Create {
        /// Exchange address
        exchange: String,

        /// Market symbol (e.g., BTC-USD)
        symbol: String,

        /// Tick size (price increment)
        #[arg(long)]
        tick_size: u64,

        /// Lot size (quantity increment)
        #[arg(long)]
        lot_size: u64,
    },

    /// List all matchers for an exchange
    List {
        /// Exchange address
        exchange: String,
    },

    /// Show matcher details
    Info {
        /// Matcher address
        matcher: String,
    },

    /// Register a slab in the router registry
    RegisterSlab {
        /// Registry address
        registry: String,

        /// Slab program address
        slab_id: String,

        /// Oracle address
        oracle_id: String,

        /// Initial margin ratio in basis points (e.g., 500 = 5%)
        #[arg(long, default_value = "500")]
        imr_bps: u64,

        /// Maintenance margin ratio in basis points
        #[arg(long, default_value = "300")]
        mmr_bps: u64,

        /// Maker fee cap in basis points
        #[arg(long, default_value = "10")]
        maker_fee_bps: u64,

        /// Taker fee cap in basis points
        #[arg(long, default_value = "20")]
        taker_fee_bps: u64,

        /// Latency SLA in milliseconds
        #[arg(long, default_value = "100")]
        latency_sla_ms: u64,

        /// Maximum position exposure
        #[arg(long, default_value = "1000000000000")]
        max_exposure: u128,
    },

    /// Update funding rate for a slab
    UpdateFunding {
        /// Slab address
        slab: String,

        /// Oracle price (scaled by 1e6, e.g., 100_000_000 for price 100)
        #[arg(long)]
        oracle_price: i64,

        /// Time to wait (simulates time passage for funding accrual, in seconds)
        #[arg(long)]
        wait_time: Option<u64>,
    },

    /// Place a limit order on the order book
    PlaceOrder {
        /// Slab address
        slab: String,

        /// Side (buy or sell)
        #[arg(long)]
        side: String,

        /// Price (scaled by 1e6, e.g., 100_000_000 for price 100)
        #[arg(long)]
        price: i64,

        /// Quantity (scaled by 1e6, e.g., 1_000_000 for quantity 1.0)
        #[arg(long)]
        qty: i64,

        /// Post-only (reject if order would cross immediately)
        #[arg(long)]
        post_only: bool,

        /// Reduce-only (order can only reduce existing position)
        #[arg(long)]
        reduce_only: bool,
    },

    /// Cancel an order by ID
    CancelOrder {
        /// Slab address
        slab: String,

        /// Order ID to cancel
        #[arg(long)]
        order_id: u64,
    },

    /// Get order book state
    GetOrderbook {
        /// Slab address
        slab: String,
    },

    /// Match an incoming order against the order book (CommitFill)
    MatchOrder {
        /// Slab address
        slab: String,

        /// Side (buy or sell)
        #[arg(long)]
        side: String,

        /// Quantity (scaled by 1e6)
        #[arg(long)]
        qty: i64,

        /// Limit price (scaled by 1e6)
        #[arg(long)]
        limit_price: i64,

        /// Time-in-force (GTC, IOC, FOK)
        #[arg(long, default_value = "GTC")]
        time_in_force: String,

        /// Self-trade prevention (None, CancelNewest, CancelOldest, DecrementAndCancel)
        #[arg(long, default_value = "None")]
        self_trade_prevention: String,
    },

    /// Halt trading on a slab (LP owner only)
    HaltTrading {
        /// Slab address
        slab: String,
    },

    /// Resume trading on a slab (LP owner only)
    ResumeTrading {
        /// Slab address
        slab: String,
    },
}

#[derive(Subcommand)]
enum LiquidityCommands {
    /// Add liquidity to a matcher
    Add {
        /// Matcher address
        matcher: String,

        /// Amount to add (in base currency)
        amount: u64,

        /// Price for the liquidity (required for orderbook mode)
        #[arg(long)]
        price: Option<f64>,

        /// LP mode: amm (default) or orderbook
        #[arg(long, default_value = "amm")]
        mode: String,

        /// Order side for orderbook mode (buy/sell)
        #[arg(long)]
        side: Option<String>,

        /// Post-only flag for orderbook mode
        #[arg(long)]
        post_only: bool,

        /// Reduce-only flag for orderbook mode
        #[arg(long)]
        reduce_only: bool,

        /// Lower price for AMM range (AMM mode only)
        #[arg(long)]
        lower_price: Option<f64>,

        /// Upper price for AMM range (AMM mode only)
        #[arg(long)]
        upper_price: Option<f64>,
    },

    /// Remove liquidity from a matcher
    Remove {
        /// Matcher address
        matcher: String,

        /// Amount to remove
        amount: u64,
    },

    /// Show liquidity provider positions
    Show {
        /// Optional user address (defaults to CLI keypair)
        user: Option<String>,
    },
}

#[derive(Subcommand)]
enum AmmCommands {
    /// Create a new AMM pool
    Create {
        /// Registry address
        registry: String,

        /// Trading pair symbol (e.g., "BTC-USD")
        symbol: String,

        /// Initial X reserve (base)
        #[arg(long)]
        x_reserve: u64,

        /// Initial Y reserve (quote)
        #[arg(long)]
        y_reserve: u64,
    },

    /// List AMM pools (placeholder)
    List,
}

#[derive(Subcommand)]
enum TradeCommands {
    /// Place a limit order (router cross-slab, fill-or-kill)
    Limit {
        /// Matcher address
        matcher: String,

        /// Buy or sell
        side: String,

        /// Order price
        price: f64,

        /// Order size
        size: u64,

        /// Post-only order
        #[arg(long)]
        post_only: bool,
    },

    /// Place a market order
    Market {
        /// Matcher address
        matcher: String,

        /// Buy or sell
        side: String,

        /// Order size
        size: u64,
    },

    /// Cancel an order (receipt-based)
    Cancel {
        /// Order ID
        order_id: String,
    },

    /// Place a resting order directly on slab (maker flow)
    SlabOrder {
        /// Slab address
        slab: String,

        /// Buy or sell
        side: String,

        /// Order price
        price: f64,

        /// Order size
        size: u64,
    },

    /// Cancel a slab order by order ID
    SlabCancel {
        /// Slab address
        slab: String,

        /// Order ID
        order_id: u64,
    },

    /// Modify a slab order (change price and/or size)
    SlabModify {
        /// Slab address
        slab: String,

        /// Order ID
        order_id: u64,

        /// New order price
        price: f64,

        /// New order size
        size: u64,
    },

    /// List open orders
    Orders {
        /// Optional user address (defaults to CLI keypair)
        user: Option<String>,
    },

    /// Show order book
    Book {
        /// Matcher address
        matcher: String,

        /// Number of levels to show
        #[arg(short, long, default_value = "10")]
        depth: usize,
    },
}

#[derive(Subcommand)]
enum MarginCommands {
    /// Initialize portfolio account for the user
    Init,

    /// Deposit collateral
    Deposit {
        /// Amount in lamports
        amount: u64,

        /// Token mint address
        #[arg(long)]
        token: Option<String>,
    },

    /// Withdraw collateral
    Withdraw {
        /// Amount in lamports
        amount: u64,

        /// Token mint address
        #[arg(long)]
        token: Option<String>,
    },

    /// Show margin account
    Show {
        /// Optional user address (defaults to CLI keypair)
        user: Option<String>,
    },

    /// Calculate margin requirements
    Requirements {
        /// User address
        user: String,
    },
}

#[derive(Subcommand)]
enum LiquidationCommands {
    /// Trigger liquidation for an account
    Execute {
        /// User address to liquidate
        user: String,

        /// Max position size to liquidate
        #[arg(long)]
        max_size: Option<u64>,
    },

    /// List liquidatable accounts
    List {
        /// Exchange address
        exchange: String,
    },

    /// Show liquidation history
    History {
        /// Number of recent liquidations to show
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },
}

#[derive(Subcommand)]
enum InsuranceCommands {
    /// Add funds to insurance fund
    Fund {
        /// Exchange address
        exchange: String,

        /// Amount in lamports
        amount: u64,
    },

    /// Show insurance fund balance
    Balance {
        /// Exchange address
        exchange: String,
    },

    /// Show insurance fund usage history
    History {
        /// Exchange address
        exchange: String,
    },
}

#[derive(Subcommand)]
enum CrisisCommands {
    /// Simulate crisis scenario
    Simulate {
        /// Exchange address
        exchange: String,

        /// Deficit amount to simulate
        deficit: u64,

        /// Dry run (don't execute)
        #[arg(long)]
        dry_run: bool,
    },

    /// Show crisis history
    History {
        /// Exchange address
        exchange: String,
    },

    /// Test haircut calculation
    TestHaircut {
        /// Total deficit
        deficit: u64,

        /// Warming PnL balance
        warming_pnl: u64,

        /// Insurance fund balance
        insurance: u64,

        /// Total equity
        equity: u64,
    },
}

#[derive(Subcommand)]
enum KeeperCommands {
    /// Start keeper bot
    Run {
        /// Exchange address to monitor
        exchange: String,

        /// Check interval in seconds
        #[arg(short, long, default_value = "5")]
        interval: u64,

        /// Only monitor, don't execute
        #[arg(long)]
        monitor_only: bool,
    },

    /// Show keeper statistics
    Stats {
        /// Exchange address
        exchange: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    // Initialize network configuration
    let config = NetworkConfig::new(
        &cli.network,
        cli.url.clone(),
        cli.keypair.clone(),
    )?;

    if cli.verbose {
        println!("{} {}", "Network:".bright_cyan(), config.network);
        println!("{} {}", "RPC URL:".bright_cyan(), config.rpc_url);
        println!("{} {}", "Keypair:".bright_cyan(), config.keypair_path.display());
    }

    // Execute command
    match cli.command {
        Commands::Deploy { router, slab, amm, oracle, all, program_keypair } => {
            deploy::deploy_programs(&config, router, slab, amm, oracle, all, program_keypair).await?;
        }
        Commands::Init { name, insurance_fund, maintenance_margin, initial_margin } => {
            exchange::initialize_exchange(&config, name, insurance_fund, maintenance_margin, initial_margin).await?;
        }
        Commands::Matcher { command } => {
            match command {
                MatcherCommands::Create { exchange, symbol, tick_size, lot_size } => {
                    matcher::create_matcher(&config, exchange, symbol, tick_size, lot_size).await?;
                }
                MatcherCommands::List { exchange } => {
                    matcher::list_matchers(&config, exchange).await?;
                }
                MatcherCommands::Info { matcher } => {
                    matcher::show_matcher_info(&config, matcher).await?;
                }
                MatcherCommands::RegisterSlab {
                    registry,
                    slab_id,
                    oracle_id,
                    imr_bps,
                    mmr_bps,
                    maker_fee_bps,
                    taker_fee_bps,
                    latency_sla_ms,
                    max_exposure
                } => {
                    matcher::register_slab(
                        &config,
                        registry,
                        slab_id,
                        oracle_id,
                        imr_bps,
                        mmr_bps,
                        maker_fee_bps,
                        taker_fee_bps,
                        latency_sla_ms,
                        max_exposure
                    ).await?;
                }
                MatcherCommands::UpdateFunding { slab, oracle_price, wait_time } => {
                    matcher::update_funding(&config, slab, oracle_price, wait_time).await?;
                }
                MatcherCommands::PlaceOrder { slab, side, price, qty, post_only, reduce_only } => {
                    matcher::place_order(&config, slab, side, price, qty, post_only, reduce_only).await?;
                }
                MatcherCommands::CancelOrder { slab, order_id } => {
                    matcher::cancel_order(&config, slab, order_id).await?;
                }
                MatcherCommands::GetOrderbook { slab } => {
                    matcher::get_orderbook(&config, slab).await?;
                }
                MatcherCommands::MatchOrder { slab, side, qty, limit_price, time_in_force, self_trade_prevention } => {
                    matcher::match_order(&config, slab, side, qty, limit_price, time_in_force, self_trade_prevention).await?;
                }
                MatcherCommands::HaltTrading { slab } => {
                    matcher::halt_trading(&config, slab).await?;
                }
                MatcherCommands::ResumeTrading { slab } => {
                    matcher::resume_trading(&config, slab).await?;
                }
            }
        }
        Commands::Liquidity { command } => {
            match command {
                LiquidityCommands::Add {
                    matcher,
                    amount,
                    price,
                    mode,
                    side,
                    post_only,
                    reduce_only,
                    lower_price,
                    upper_price,
                } => {
                    liquidity::add_liquidity(
                        &config,
                        matcher,
                        amount,
                        price,
                        mode,
                        side,
                        post_only,
                        reduce_only,
                        lower_price,
                        upper_price,
                    ).await?;
                }
                LiquidityCommands::Remove { matcher, amount } => {
                    liquidity::remove_liquidity(&config, matcher, amount).await?;
                }
                LiquidityCommands::Show { user } => {
                    liquidity::show_positions(&config, user).await?;
                }
            }
        }
        Commands::Amm { command } => {
            match command {
                AmmCommands::Create { registry, symbol, x_reserve, y_reserve } => {
                    amm::create_amm(&config, registry, symbol, x_reserve, y_reserve).await?;
                }
                AmmCommands::List => {
                    amm::list_amms(&config).await?;
                }
            }
        }
        Commands::Trade { command } => {
            match command {
                TradeCommands::Limit { matcher, side, price, size, post_only } => {
                    trading::place_limit_order(&config, matcher, side, price, size, post_only).await?;
                }
                TradeCommands::Market { matcher, side, size } => {
                    trading::place_market_order(&config, matcher, side, size).await?;
                }
                TradeCommands::Cancel { order_id } => {
                    trading::cancel_order(&config, order_id).await?;
                }
                TradeCommands::SlabOrder { slab, side, price, size } => {
                    trading::place_slab_order(&config, slab, side, price, size).await?;
                }
                TradeCommands::SlabCancel { slab, order_id } => {
                    trading::cancel_slab_order(&config, slab, order_id).await?;
                }
                TradeCommands::SlabModify { slab, order_id, price, size } => {
                    trading::modify_slab_order(&config, slab, order_id, price, size).await?;
                }
                TradeCommands::Orders { user } => {
                    trading::list_orders(&config, user).await?;
                }
                TradeCommands::Book { matcher, depth } => {
                    trading::show_order_book(&config, matcher, depth).await?;
                }
            }
        }
        Commands::Margin { command } => {
            match command {
                MarginCommands::Init => {
                    margin::initialize_portfolio(&config).await?;
                }
                MarginCommands::Deposit { amount, token } => {
                    margin::deposit_collateral(&config, amount, token).await?;
                }
                MarginCommands::Withdraw { amount, token } => {
                    margin::withdraw_collateral(&config, amount, token).await?;
                }
                MarginCommands::Show { user } => {
                    margin::show_margin_account(&config, user).await?;
                }
                MarginCommands::Requirements { user } => {
                    margin::show_margin_requirements(&config, user).await?;
                }
            }
        }
        Commands::Liquidation { command } => {
            match command {
                LiquidationCommands::Execute { user, max_size } => {
                    liquidation::execute_liquidation(&config, user, max_size).await?;
                }
                LiquidationCommands::List { exchange } => {
                    liquidation::list_liquidatable(&config, exchange).await?;
                }
                LiquidationCommands::History { limit } => {
                    liquidation::show_history(&config, limit).await?;
                }
            }
        }
        Commands::Insurance { command } => {
            match command {
                InsuranceCommands::Fund { exchange, amount } => {
                    insurance::add_funds(&config, exchange, amount).await?;
                }
                InsuranceCommands::Balance { exchange } => {
                    insurance::show_balance(&config, exchange).await?;
                }
                InsuranceCommands::History { exchange } => {
                    insurance::show_history(&config, exchange).await?;
                }
            }
        }
        Commands::Crisis { command } => {
            match command {
                CrisisCommands::Simulate { exchange, deficit, dry_run } => {
                    crisis::simulate_crisis(&config, exchange, deficit, dry_run).await?;
                }
                CrisisCommands::History { exchange } => {
                    crisis::show_history(&config, exchange).await?;
                }
                CrisisCommands::TestHaircut { deficit, warming_pnl, insurance, equity } => {
                    crisis::test_haircut_calculation(deficit, warming_pnl, insurance, equity)?;
                }
            }
        }
        Commands::Keeper { command } => {
            match command {
                KeeperCommands::Run { exchange, interval, monitor_only } => {
                    keeper::run_keeper(&config, exchange, interval, monitor_only).await?;
                }
                KeeperCommands::Stats { exchange } => {
                    keeper::show_stats(&config, exchange).await?;
                }
            }
        }
        Commands::Test {
            quick,
            margin,
            orders,
            matching,
            liquidations,
            routing,
            capital_efficiency,
            crisis: test_crisis,
            lp_insolvency,
            funding,
            all,
        } => {
            println!("{}", "Running test suite...".bright_green().bold());

            if all || quick {
                tests::run_smoke_tests(&config).await?;
            }
            if all || margin {
                tests::run_margin_tests(&config).await?;
            }
            if all || orders {
                tests::run_order_tests(&config).await?;
            }
            if all || matching {
                tests::run_trade_matching_tests(&config).await?;
            }
            if all || liquidations {
                tests::run_liquidation_tests(&config).await?;
            }
            if all || routing {
                tests::run_routing_tests(&config).await?;
            }
            if all || capital_efficiency {
                tests::run_capital_efficiency_tests(&config).await?;
            }
            if all || test_crisis {
                tests::run_crisis_tests(&config).await?;
            }
            if all || lp_insolvency {
                tests::run_lp_insolvency_tests(&config).await?;
            }
            if all || funding {
                tests_funding::run_funding_tests().await?;
            }
        }
        Commands::Status { exchange, detailed } => {
            exchange::query_registry_status(&config, exchange, detailed).await?;
        }
    }

    Ok(())
}
