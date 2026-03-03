// Licensed under the Apache-2.0 license

use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod bloat;
mod build;
mod clippy;
mod docs;
mod format;
mod header;
mod test;

#[derive(Parser)]
#[command(version, about = "ASPEED DDK development automation tasks", long_about = None)]
struct Xtask {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build the project
    Build {
        /// Build for release
        #[arg(long)]
        release: bool,

        /// Target architecture
        #[arg(long, default_value = "thumbv7em-none-eabihf")]
        target: String,

        /// Additional features to enable
        #[arg(long)]
        features: Vec<String>,
    },

    /// Run clippy on all targets
    Clippy,

    /// Build documentation
    Docs {
        /// Open documentation after building
        #[arg(long)]
        open: bool,
    },

    /// Check code formatting
    Format {
        /// Fix formatting issues
        #[arg(long)]
        fix: bool,
    },

    /// Check license headers
    HeaderCheck,

    /// Add license headers to files
    HeaderFix,

    /// Run tests
    Test {
        /// Run only unit tests
        #[arg(long)]
        unit: bool,

        /// Run only integration tests
        #[arg(long)]
        integration: bool,
    },

    /// Run all pre-commit checks
    Precommit,

    /// Generate booting images
    GenBootImage {
        /// Input binary file
        #[arg(short, long)]
        input: PathBuf,

        /// Output image file
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Run functional tests on hardware
    HardwareTest {
        /// UART device path (e.g., /dev/ttyUSB0)
        #[arg(long)]
        uart: Option<String>,

        /// Test suite to run
        #[arg(long)]
        suite: Option<String>,
    },

    /// Analyze binary size with cargo bloat
    Bloat {
        /// Build for release
        #[arg(long)]
        release: bool,

        /// Target architecture
        #[arg(long, default_value = "thumbv7em-none-eabihf")]
        target: String,

        /// Generate detailed report
        #[arg(long)]
        report: bool,

        /// Output directory for reports
        #[arg(long, default_value = "target/bloat-reports")]
        output_dir: String,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Xtask::parse();

    match cli.command {
        Commands::Build {
            release,
            target,
            features,
        } => build::build(release, &target, &features),
        Commands::Clippy => clippy::clippy(),
        Commands::Docs { open } => docs::docs(open),
        Commands::Format { fix } => format::format(fix),
        Commands::HeaderCheck => header::check(),
        Commands::HeaderFix => header::fix(),
        Commands::Test { unit, integration } => test::test(unit, integration),
        Commands::Precommit => precommit(),
        Commands::GenBootImage { input, output } => build::gen_boot_image(&input, &output),
        Commands::HardwareTest { uart, suite } => {
            test::hardware_test(uart.as_deref(), suite.as_deref())
        }
        Commands::Bloat {
            release,
            target,
            report,
            output_dir,
        } => {
            if report {
                bloat::generate_report(release, &target, &output_dir)
            } else {
                bloat::analyze_bloat(release, &target, bloat::BloatFormat::Table)
            }
        }
    }
}

fn precommit() -> anyhow::Result<()> {
    println!("Running pre-commit checks...");

    // Check formatting
    format::format(false)?;

    // Check clippy
    clippy::clippy()?;

    // Check license headers
    header::check()?;

    // Run tests
    test::test(false, false)?;

    // Generate size analysis report (optional - skip if cargo-bloat not available)
    println!("Generating binary size report...");
    match bloat::generate_report(true, "thumbv7em-none-eabihf", "target/bloat-reports") {
        Ok(_) => println!("📊 Size analysis report generated"),
        Err(e) => {
            println!("⚠️  Size analysis skipped: {}", e);
            println!("   Install cargo-bloat with: cargo install cargo-bloat");
        }
    }

    println!("✅ All pre-commit checks passed!");
    Ok(())
}
