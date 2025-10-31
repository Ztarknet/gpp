use clap::{Parser, Subcommand, ValueEnum};
use log::info;
use std::path::PathBuf;

mod cmd_utils;
mod generate_pie;
mod proof_utils;
mod stwo_run_and_prove;

use proof_utils::load_and_print_proof;
use stwo_run_and_prove::stwo_run_and_prove;

use crate::generate_pie::{format_block_numbers, generate_pie};

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
                strk_fee_token:
                    "0x04718f5a0fc34cc1af16a1cdee98ffb20c31f5cd61d6ab07201858f4287c938d",
                eth_fee_token: "0x049d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7",
            },
            Network::Mainnet => NetworkConfig {
                rpc_url: "https://pathfinder-mainnet.d.karnot.xyz",
                strk_fee_token:
                    "0x04718f5a0fc34cc1af16a1cdee98ffb20c31f5cd61d6ab07201858f4287c938d",
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
#[command(
    about = "Generate PIE and Proof - A CLI utility for generating PIE using snos and creating proofs using stwo_run_and_prove"
)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate PIE and create proof (full pipeline)
    Generate {
        /// Block number or range (e.g., "123" or "100-110")
        #[arg(short, long)]
        block_numbers: String,

        /// Output directory for generated files
        #[arg(short, long, default_value = "./output")]
        output_dir: PathBuf,

        /// Path to bootloader program JSON file
        #[arg(
            long,
            default_value = "bootloaders/simple_bootloader_compiled.json"
        )]
        program: PathBuf,

        /// Path to prover parameters JSON file
        #[arg(long, default_value = "prover_params.json")]
        prover_params: PathBuf,

        /// Keep intermediate files after completion
        #[arg(long)]
        keep_intermediate: bool,

        /// Network (sepolia or mainnet)
        #[arg(short, long, default_value = "sepolia")]
        network: Network,
    },
    /// Load a proof file and print its output
    PrintOutput {
        /// Path to the proof file (either .json or .bz format)
        #[arg(short, long)]
        proof_file: PathBuf,
    },
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

    match cli.command {
        Commands::Generate {
            block_numbers,
            output_dir,
            program,
            prover_params,
            keep_intermediate,
            network,
        } => {
            // Get network configuration
            let network_config = network.config();
            let network_name = network.as_str();

            info!("Starting PIE generation and proof creation");
            info!("Network: {}", network_name);
            info!("Block numbers: {}", block_numbers);
            info!("Output directory: {}", output_dir.display());
            info!("RPC URL: {}", network_config.rpc_url);

            // Create output directory if it doesn't exist
            std::fs::create_dir_all(&output_dir)?;

            // Step 1: Generate PIE
            info!("=== Step 1: Generating PIE ===");

            // Format block numbers to comma-separated format
            let block_range = format_block_numbers(&block_numbers)?;

            // Create output path for PIE file
            let pie_filename =
                format!("pie_{}_{}.zip", network_name, block_numbers);
            let pie_path = output_dir.join(pie_filename);

            // Call generate-pie binary
            generate_pie(
                &block_range,
                &pie_path,
                network_config.rpc_url,
                network_name,
                network_config.strk_fee_token,
                network_config.eth_fee_token,
                cli.verbose,
            )
            .await?;

            info!("PIE generated successfully: {}", pie_path.display());

            // Step 2: Create proof
            info!("=== Step 2: Creating proof ===");
            let proof_result = stwo_run_and_prove(
                &program,
                &pie_path,
                &prover_params,
                &output_dir,
                cli.verbose,
            )
            .await?;
            info!("Proof created successfully: {}", proof_result.display());

            info!("=== Step 3: Print proof output ===");
            load_and_print_proof(&proof_result)?;

            // Clean up intermediate files if requested
            if !keep_intermediate {
                info!("Cleaning up intermediate files...");
                // Add cleanup logic here if needed
            }

            info!("Pipeline completed successfully!");
        }
        Commands::PrintOutput { proof_file } => {
            info!("Loading proof from: {}", proof_file.display());
            load_and_print_proof(&proof_file)?;
        }
    }

    Ok(())
}
