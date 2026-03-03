// Licensed under the Apache-2.0 license

use anyhow::{Context, Result};
use std::process::Command;

/// Run cargo bloat analysis and generate size report
pub fn analyze_bloat(release: bool, target: &str, format: BloatFormat) -> Result<()> {
    println!("Running binary size analysis...");

    let mut cmd = Command::new("cargo");
    cmd.arg("bloat");

    if release {
        cmd.arg("--release");
    }

    cmd.args(["--target", target]);

    match format {
        BloatFormat::Table => {
            // Default table format - most readable for humans
        }
        BloatFormat::Json => {
            cmd.arg("--message-format=json");
        }
        BloatFormat::Csv => {
            cmd.arg("--format=csv");
        }
    }

    let output = cmd.output().context(
        "Failed to run cargo bloat - make sure it's installed with 'cargo install cargo-bloat'",
    )?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("cargo bloat failed: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("{}", stdout);

    Ok(())
}

/// Generate detailed bloat report with multiple views
pub fn generate_report(release: bool, target: &str, output_dir: &str) -> Result<()> {
    println!("Generating comprehensive binary size report...");

    // Create output directory
    std::fs::create_dir_all(output_dir)
        .with_context(|| format!("Failed to create output directory: {}", output_dir))?;

    // Generate different report formats
    let reports = [
        ("functions", "--crates"),
        ("crates", "--crates"),
        ("time", "--time"),
    ];

    for (name, flag) in &reports {
        let output_file = format!("{}/bloat_{}.txt", output_dir, name);

        let mut cmd = Command::new("cargo");
        cmd.arg("bloat");

        if release {
            cmd.arg("--release");
        }

        cmd.args(["--target", target]);
        cmd.arg(flag);

        let output = cmd
            .output()
            .with_context(|| format!("Failed to generate {} report", name))?;

        if output.status.success() {
            std::fs::write(&output_file, &output.stdout)
                .with_context(|| format!("Failed to write report to {}", output_file))?;

            println!("📊 Generated {}", output_file);
        }
    }

    // Generate size comparison (if previous reports exist)
    generate_size_comparison(output_dir)?;

    println!("✅ Binary size report generated in {}", output_dir);
    Ok(())
}

/// Compare current size with previous builds
fn generate_size_comparison(output_dir: &str) -> Result<()> {
    // This is a placeholder for size comparison logic
    // In a real implementation, you'd:
    // 1. Store historical size data
    // 2. Compare with baseline
    // 3. Detect regressions

    let comparison_file = format!("{}/size_comparison.md", output_dir);
    let comparison_content = r#"# Binary Size Comparison

## Current Build Analysis
- Total binary size: [PLACEHOLDER]
- Largest functions: [PLACEHOLDER] 
- Largest crates: [PLACEHOLDER]

## Size Regression Detection
- Compared to main branch: [PLACEHOLDER]
- Size change: [PLACEHOLDER]

## Recommendations
- Consider `#[inline(never)]` for large functions
- Review generic monomorphization 
- Check for unexpected std library usage
"#;

    std::fs::write(&comparison_file, comparison_content)
        .with_context(|| format!("Failed to write comparison to {}", comparison_file))?;

    Ok(())
}

#[derive(Clone)]
#[allow(dead_code)] // Json and Csv variants reserved for future use
pub enum BloatFormat {
    Table,
    Json,
    Csv,
}
