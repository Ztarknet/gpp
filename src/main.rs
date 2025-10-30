use clap::{Parser, ValueEnum};
use log::info;
use std::path::PathBuf;

mod cmd_utils;
mod generate_pie;
mod stwo_run_and_prove;

use stwo_run_and_prove::stwo_run_and_prove;

#[derive(Debug, Clone, ValueEnum)]
enum Network {
    Sepolia,
    Mainnet,
}

struct NetworkConfig {
    rpc_url: &'static str,
    strk_fee_token: &'static str,
    eth_fee_token: &'static str,
}

impl Network {
    fn config(&self) -> NetworkConfig {
        match self {
            Network::Sepolia => NetworkConfig {
                rpc_url: "https://pathfinder-sepolia.d.karnot.xyz",
                strk_fee_token: "0x04718f5a0fc34cc1af16a1cdee98ffb20c31f5cd61d6ab07201858f4287c938d",
                eth_fee_token: "0x049d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7",
            },
            Network::Mainnet => NetworkConfig {
                rpc_url: "https://pathfinder-mainnet.d.karnot.xyz",
                strk_fee_token: "0x04718f5a0fc34cc1af16a1cdee98ffb20c31f5cd61d6ab07201858f4287c938d",
                eth_fee_token: "0x049d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7",
            },
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            Network::Sepolia => "sepolia",
            Network::Mainnet => "mainnet",
        }
    }
}

#[derive(Parser)]
#[command(name = "gpp")]
#[command(about = "Generate PIE and Proof - A CLI utility for generating PIE using snos and creating proofs using stwo_run_and_prove")]
#[command(version)]
struct Cli {
    /// Block number or range (e.g., "123" or "100-110")
    #[arg(short, long)]
    block_numbers: String,

    /// Output directory for generated files
    #[arg(short, long, default_value = "./output")]
    output_dir: PathBuf,

    /// Path to bootloader program JSON file
    #[arg(long, default_value = "bootloaders/simple_bootloader_compiled.json")]
    program: PathBuf,

    /// Path to prover parameters JSON file
    #[arg(long, default_value = "prover_params.json")]
    prover_params: PathBuf,

    /// Keep intermediate files after completion
    #[arg(long)]
    keep_intermediate: bool,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Network (sepolia or mainnet)
    #[arg(short, long, default_value = "sepolia")]
    network: Network,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    env_logger::Builder::from_default_env()
        .filter_level(if cli.verbose {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Info
        })
        .init();

    // Get network configuration
    let network_config = cli.network.config();
    let network_name = cli.network.as_str();

    info!("Starting PIE generation and proof creation");
    info!("Network: {}", network_name);
    info!("Block numbers: {}", cli.block_numbers);
    info!("Output directory: {}", cli.output_dir.display());
    info!("RPC URL: {}", network_config.rpc_url);

    // Create output directory if it doesn't exist
    std::fs::create_dir_all(&cli.output_dir)?;

    // Step 1: Generate PIE
    info!("=== Step 1: Generating PIE ===");
    
    // Format block numbers to comma-separated format
    let block_range = generate_pie::format_block_numbers(&cli.block_numbers)?;
    
    // Create output path for PIE file
    let pie_filename = format!("pie_{}_{}.zip", network_name, cli.block_numbers);
    let pie_path = cli.output_dir.join(pie_filename);
    
    // Call generate-pie binary
    generate_pie::generate_pie(
        &block_range,
        &pie_path,
        network_config.rpc_url,
        network_name,
        network_config.strk_fee_token,
        network_config.eth_fee_token,
    ).await?;
    
    info!("PIE generated successfully: {}", pie_path.display());

    // Step 2: Create proof
    info!("=== Step 2: Creating proof ===");
    let proof_result = stwo_run_and_prove(
        &cli.program,
        &pie_path,
        &cli.prover_params,
        &cli.output_dir,
    ).await?;
    info!("Proof created successfully: {}", proof_result.display());

    // Clean up intermediate files if requested
    if !cli.keep_intermediate {
        info!("Cleaning up intermediate files...");
        // Add cleanup logic here if needed
    }

    info!("Pipeline completed successfully!");
    Ok(())
}