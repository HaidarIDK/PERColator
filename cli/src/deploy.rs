//! Program deployment logic

use anyhow::{Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    program_pack::Pack,
    pubkey::Pubkey,
    signature::Signer,
    system_program,
};
use std::path::PathBuf;
use std::process::Command;

use crate::config::NetworkConfig;

#[derive(Serialize, Deserialize)]
pub struct DeploymentResult {
    pub success: bool,
    pub deployed_programs: Vec<ProgramDeployment>,
    pub network: String,
    pub deployer: String,
}

#[derive(Serialize, Deserialize)]
pub struct ProgramDeployment {
    pub name: String,
    pub program_id: String,
    pub size_bytes: usize,
}

const ROUTER_SO: &str = "target/deploy/percolator_router.so";
const SLAB_SO: &str = "target/deploy/percolator_slab.so";
const AMM_SO: &str = "target/deploy/percolator_amm.so";
const ORACLE_SO: &str = "target/deploy/percolator_oracle.so";

pub async fn deploy_programs(
    config: &NetworkConfig,
    router: bool,
    slab: bool,
    amm: bool,
    oracle: bool,
    all: bool,
    program_keypair: Option<PathBuf>,
) -> Result<()> {
    let mut deployed_programs = Vec::new();

    if !config.json_output {
        println!("{}", "=== Program Deployment ===".bright_green().bold());
        println!("{} {}", "Network:".bright_cyan(), config.network);
        println!("{} {}\n", "Deployer:".bright_cyan(), config.pubkey());
    }

    // Build programs first
    if !config.json_output {
        build_programs()?;
    } else {
        build_programs_quiet()?;
    }

    if all || router {
        if !config.json_output {
            println!("\n{}", "Deploying Router Program...".bright_yellow());
        }
        let deployment = deploy_program(config, ROUTER_SO, "Router", program_keypair.as_deref()).await?;
        deployed_programs.push(deployment);
    }

    if all || slab {
        if !config.json_output {
            println!("\n{}", "Deploying Slab (Matcher) Program...".bright_yellow());
        }
        let deployment = deploy_program(config, SLAB_SO, "Slab", program_keypair.as_deref()).await?;
        deployed_programs.push(deployment);
    }

    if all || amm {
        if !config.json_output {
            println!("\n{}", "Deploying AMM Program...".bright_yellow());
        }
        let deployment = deploy_program(config, AMM_SO, "AMM", program_keypair.as_deref()).await?;
        deployed_programs.push(deployment);
    }

    if all || oracle {
        if !config.json_output {
            println!("\n{}", "Deploying Oracle Program...".bright_yellow());
        }
        let deployment = deploy_program(config, ORACLE_SO, "Oracle", program_keypair.as_deref()).await?;
        deployed_programs.push(deployment);
    }

    if config.json_output {
        let result = DeploymentResult {
            success: true,
            deployed_programs,
            network: config.network.clone(),
            deployer: config.pubkey().to_string(),
        };
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("\n{}", "=== Deployment Complete ===".bright_green().bold());
    }

    Ok(())
}

fn build_programs() -> Result<()> {
    println!("{}", "Building Solana programs with cargo build-sbf...".dimmed());

    let output = Command::new("cargo")
        .arg("build-sbf")
        .output()
        .context("Failed to execute cargo build-sbf. Is Solana CLI installed?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Build failed:\n{}", stderr);
    }

    println!("{}", "Build successful!".bright_green());

    Ok(())
}

fn build_programs_quiet() -> Result<()> {
    let output = Command::new("cargo")
        .arg("build-sbf")
        .output()
        .context("Failed to execute cargo build-sbf. Is Solana CLI installed?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Build failed:\n{}", stderr);
    }

    Ok(())
}

async fn deploy_program(
    config: &NetworkConfig,
    program_path: &str,
    name: &str,
    _program_keypair: Option<&std::path::Path>,
) -> Result<ProgramDeployment> {
    use std::fs;

    // Check if program file exists
    if !std::path::Path::new(program_path).exists() {
        anyhow::bail!(
            "Program file not found: {}\nRun 'cargo build-sbf' first",
            program_path
        );
    }

    let program_data = fs::read(program_path)
        .with_context(|| format!("Failed to read program file: {}", program_path))?;

    let size_bytes = program_data.len();

    if !config.json_output {
        println!("{} Program size: {} bytes", "  ├─".dimmed(), size_bytes);
    }

    // Use solana program deploy command for now
    // In a production tool, you'd use solana_program_test or similar
    let output = Command::new("solana")
        .arg("program")
        .arg("deploy")
        .arg(program_path)
        .arg("--url")
        .arg(&config.rpc_url)
        .arg("--keypair")
        .arg(&config.keypair_path)
        .output()
        .context("Failed to execute solana program deploy")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Deployment failed:\n{}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Extract program ID from output
    let program_id = if let Some(line) = stdout.lines().find(|l| l.contains("Program Id:")) {
        if !config.json_output {
            println!("{} {}", "  └─".dimmed(), line.bright_green());
        }
        // Extract the pubkey from "Program Id: <pubkey>"
        line.split_whitespace()
            .last()
            .unwrap_or("unknown")
            .to_string()
    } else {
        if !config.json_output {
            println!("{} {}", "  └─".dimmed(), "Deployed successfully".bright_green());
        }
        "unknown".to_string()
    };

    Ok(ProgramDeployment {
        name: name.to_string(),
        program_id,
        size_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_program_paths() {
        // Just verify constants are defined
        assert!(!ROUTER_SO.is_empty());
        assert!(!SLAB_SO.is_empty());
    }
}
