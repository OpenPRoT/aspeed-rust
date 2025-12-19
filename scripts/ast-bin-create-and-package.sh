#!/bin/bash
# Licensed under the Apache-2.0 license
set -euo pipefail

# AST1060 Binary Creation and Packaging Script
# Converts ELF firmware to UART bootable binary format

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "OPTIONS:"
    echo "  -t, --target TARGET        Target triple (default: thumbv7em-none-eabihf)"
    echo "  -b, --binary NAME          Binary name (default: aspeed-ddk)"
    echo "  -o, --output OUTPUT        Output binary path (default: target/functional-tests.bin)"
    echo "  -s, --max-size SIZE        Maximum binary size in bytes (default: 1048576)"
    echo "  -q, --quiet                Suppress informational output"
    echo "  -h, --help                 Show this help message"
    echo ""
    echo "EXAMPLES:"
    echo "  $0                                    # Use defaults"
    echo "  $0 -b my-firmware -o my-firmware.bin # Custom binary name and output"
    echo "  $0 -s 2097152                        # Allow 2MB max size"
    echo "  $0 -q                                # Run silently"
}

# Default values
TARGET="thumbv7em-none-eabihf"
BINARY_NAME="aspeed-ddk"
OUTPUT_PATH="target/functional-tests.bin"
MAX_SIZE=1048576  # 1MB default
QUIET=false

# Helper function for conditional echo
log() {
    if [[ "$QUIET" != "true" ]]; then
        echo "$@"
    fi
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -t|--target)
            TARGET="$2"
            shift 2
            ;;
        -b|--binary)
            BINARY_NAME="$2"
            shift 2
            ;;
        -o|--output)
            OUTPUT_PATH="$2"
            shift 2
            ;;
        -s|--max-size)
            MAX_SIZE="$2"
            shift 2
            ;;
        -q|--quiet)
            QUIET=true
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "Error: Unknown option $1"
            usage
            exit 1
            ;;
    esac
done

# Validate inputs
if [[ ! "$MAX_SIZE" =~ ^[0-9]+$ ]]; then
    echo "Error: max-size must be a positive integer"
    exit 1
fi

# Change to project root
cd "$PROJECT_ROOT"

# Define paths
ELF_PATH="target/$TARGET/release/$BINARY_NAME"
RAW_BINARY_PATH="target/functional-tests-raw.bin"

log "AST1060 Binary Generation"
log "========================="
log "Target: $TARGET"
log "Binary: $BINARY_NAME"
log "Output: $OUTPUT_PATH"
log "Max size: $MAX_SIZE bytes"
log ""

# Check if ELF file exists
if [[ ! -f "$ELF_PATH" ]]; then
    echo "Error: ELF file not found at $ELF_PATH"
    echo "Please build the firmware first with:"
    echo "  cargo build --release --target $TARGET"
    exit 1
fi

# Check for required tools
if ! command -v arm-none-eabi-objcopy >/dev/null 2>&1; then
    echo "Error: arm-none-eabi-objcopy not found"
    echo "Please install gcc-arm-none-eabi package"
    exit 1
fi

log "Converting ELF to raw binary..."

# Convert ELF to raw binary
if ! arm-none-eabi-objcopy \
    -O binary \
    "$ELF_PATH" \
    "$RAW_BINARY_PATH"; then
    echo "Error: Failed to convert ELF to binary"
    exit 1
fi

# Verify raw binary was created
if [[ ! -f "$RAW_BINARY_PATH" ]]; then
    echo "Error: Raw binary was not created"
    exit 1
fi

# Get raw binary size
RAW_SIZE=$(stat -c%s "$RAW_BINARY_PATH")
log "Raw binary size: $RAW_SIZE bytes"

# Check size limits
if [[ $RAW_SIZE -gt $MAX_SIZE ]]; then
    echo "Error: Binary too large ($RAW_SIZE bytes > $MAX_SIZE bytes limit)"
    echo "Consider:"
    echo "  - Enabling more aggressive optimizations"
    echo "  - Reducing feature set"
    echo "  - Using release profile with size optimization"
    exit 1
fi

log "Wrapping with UART boot header..."

# Check if UART boot image generator exists
UART_BOOT_SCRIPT="$SCRIPT_DIR/gen_uart_booting_image.sh"
if [[ ! -f "$UART_BOOT_SCRIPT" ]]; then
    echo "Error: UART boot image generator not found at $UART_BOOT_SCRIPT"
    log "Creating simple 4-byte size prefix wrapper..."
    
    # Create simple wrapper if script doesn't exist
    # Write 4-byte little-endian size prefix followed by binary data
    python3 -c "
import struct
import sys

raw_path = '$RAW_BINARY_PATH'
output_path = '$OUTPUT_PATH'
max_size = $MAX_SIZE
quiet = '$QUIET' == 'true'

try:
    with open(raw_path, 'rb') as f:
        data = f.read()
    
    size = len(data)
    if size > max_size:
        print(f'Error: Binary size {size} exceeds limit {max_size}')
        sys.exit(1)
    
    with open(output_path, 'wb') as f:
        # Write 4-byte little-endian size header
        f.write(struct.pack('<I', size))
        # Write binary data
        f.write(data)
    
    if not quiet:
        print(f'Created UART boot image: {output_path}')
        print(f'Header size: 4 bytes')
        print(f'Payload size: {size} bytes')
        print(f'Total size: {size + 4} bytes')

except Exception as e:
    print(f'Error creating UART boot image: {e}')
    sys.exit(1)
"
else
    # Use existing script
    if ! "$UART_BOOT_SCRIPT" "$RAW_BINARY_PATH" "$OUTPUT_PATH"; then
        echo "Error: Failed to create UART boot image"
        exit 1
    fi
fi

# Verify final binary was created
if [[ ! -f "$OUTPUT_PATH" ]]; then
    echo "Error: UART boot image was not created"
    exit 1
fi

# Get final sizes
UART_SIZE=$(stat -c%s "$OUTPUT_PATH")

log ""
log "Binary generation completed successfully!"
log "========================================"
log "Raw binary: $RAW_BINARY_PATH ($RAW_SIZE bytes)"
log "UART image: $OUTPUT_PATH ($UART_SIZE bytes)"
log "Header overhead: $((UART_SIZE - RAW_SIZE)) bytes"

# Calculate utilization
UTILIZATION=$((RAW_SIZE * 100 / MAX_SIZE))
log "Flash utilization: $UTILIZATION% of ${MAX_SIZE} bytes"

if [[ $UTILIZATION -gt 90 ]]; then
    log "Warning: Flash utilization is high (>90%)"
fi

log ""
log "Ready for UART upload to AST1060!"
