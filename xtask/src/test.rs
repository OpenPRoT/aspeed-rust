// Licensed under the Apache-2.0 license

use anyhow::{bail, Result};
use once_cell::sync::Lazy;
use std::path::PathBuf;
use std::process::Command;

static PROJECT_ROOT: Lazy<PathBuf> = Lazy::new(|| {
    std::path::Path::new(&env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
});

pub fn test(unit_only: bool, integration_only: bool) -> Result<()> {
    if unit_only && integration_only {
        bail!("Cannot specify both --unit and --integration flags");
    }

    if unit_only {
        run_unit_tests()?;
    } else if integration_only {
        run_integration_tests()?;
    } else {
        // Run all tests
        run_unit_tests()?;
        run_integration_tests()?;
    }

    println!("✅ All tests passed");
    Ok(())
}

fn run_unit_tests() -> Result<()> {
    println!("Running unit tests...");

    let status = Command::new("cargo")
        .current_dir(&*PROJECT_ROOT)
        .args([
            "test",
            "--workspace",
            "--exclude",
            "aspeed-ddk",
            "--exclude",
            "aspeed-ddk-tests-hw",
            "--target",
            "x86_64-unknown-linux-gnu",
        ])
        .status()?;

    if !status.success() {
        bail!("Unit tests failed");
    }

    println!("✅ Unit tests passed");
    Ok(())
}

fn run_integration_tests() -> Result<()> {
    println!("Running integration tests...");

    let status = Command::new("cargo")
        .current_dir(&*PROJECT_ROOT)
        .args([
            "test",
            "--workspace",
            "--exclude",
            "aspeed-ddk",
            "--exclude",
            "aspeed-ddk-tests-hw",
            "--target",
            "x86_64-unknown-linux-gnu",
        ])
        .status()?;

    if !status.success() {
        bail!("Integration tests failed");
    }

    println!("✅ Integration tests passed");
    Ok(())
}

pub fn hardware_test(uart_device: Option<&str>, test_suite: Option<&str>) -> Result<()> {
    println!("Running hardware tests...");

    let uart = uart_device.unwrap_or("/dev/ttyUSB0");
    println!("Using UART device: {}", uart);

    // Check if device exists
    if !std::path::Path::new(uart).exists() {
        bail!("UART device {} not found", uart);
    }

    if let Some(suite) = test_suite {
        println!("Running test suite: {}", suite);
        run_specific_hardware_test(uart, suite)?;
    } else {
        println!("Running all hardware test suites...");
        run_all_hardware_tests(uart)?;
    }

    println!("✅ Hardware tests completed");
    Ok(())
}

fn run_specific_hardware_test(uart: &str, suite: &str) -> Result<()> {
    match suite {
        "rsa" => {
            println!("Running RSA hardware tests...");
            run_hardware_test_suite(uart, "rsa")?;
        }
        "ecdsa" => {
            println!("Running ECDSA hardware tests...");
            run_hardware_test_suite(uart, "ecdsa")?;
        }
        "hash" => {
            println!("Running Hash hardware tests...");
            run_hardware_test_suite(uart, "hash")?;
        }
        "hmac" => {
            println!("Running HMAC hardware tests...");
            run_hardware_test_suite(uart, "hmac")?;
        }
        _ => {
            bail!(
                "Unknown test suite: {}. Available suites: rsa, ecdsa, hash, hmac",
                suite
            );
        }
    }
    Ok(())
}

fn run_all_hardware_tests(uart: &str) -> Result<()> {
    let suites = ["rsa", "ecdsa", "hash", "hmac"];

    for suite in &suites {
        println!("Running {} tests...", suite.to_uppercase());
        run_hardware_test_suite(uart, suite)?;
    }

    Ok(())
}

fn run_hardware_test_suite(uart: &str, suite: &str) -> Result<()> {
    // Build the test binary with the specific feature enabled
    let feature = format!("test-{}", suite);

    let status = Command::new("cargo")
        .current_dir(&*PROJECT_ROOT)
        .args([
            "build",
            "-p",
            "aspeed-ddk-tests-hw",
            "--release",
            "--target",
            "thumbv7em-none-eabihf",
            "--features",
            &feature,
        ])
        .status()?;

    if !status.success() {
        bail!("Failed to build {} test binary", suite);
    }

    // Generate UART boot image
    let elf_path = PROJECT_ROOT.join("target/thumbv7em-none-eabihf/release/aspeed-ddk-tests-hw");
    assert!(elf_path.exists(), "ELF file not found: {:?}", elf_path);
    println!("ELF binary path: {:?}", elf_path);
    let bin_path = PROJECT_ROOT.join(format!("target/{}-test-{}.bin", suite, "release"));
    println!("Binary path: {:?}", bin_path);

    let _status = Command::new("cargo")
        .current_dir(&*PROJECT_ROOT)
        .args([
            "objcopy",
            "-p",
            "aspeed-ddk-tests-hw",
            "--target",
            "thumbv7em-none-eabihf",
            "--release",
            "--",
            "-O",
            "binary",
            bin_path.to_str().unwrap(),
        ])
        .status()?;

    let boot_image_path = PROJECT_ROOT.join(format!("target/{}-test-boot.img", suite));
    crate::build::gen_boot_image(&bin_path, &boot_image_path)?;

    // TODO: Add actual hardware test execution here
    // This would involve sending the boot image to the hardware via UART
    // and collecting/analyzing the test results

    println!("Hardware test for {} suite would run here", suite);
    println!("Boot image: {:?}", boot_image_path);
    println!("UART device: {}", uart);

    Ok(())
}
