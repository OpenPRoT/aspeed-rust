# CI Setup for the ASPEED AST1060 + EVB A3

## Overview

The ASPEED AST1060 EVB A3 features two AST1060 evaluation boards linked over a larger test fixture. One device is equipped with a JTAG pigtail on the underside of the board.

Control of the AST1060 for testing purposes requires both boards, but only one needs to be loaded regularly. It is recommended that the secondary board (without JTAG) have an appropriate image written to SPI.

## Resources

To set up CI for the AST1060, the following devices will be needed:

* A Raspberry Pi 4 or 5
* An ASpeed PRoT Fixture vA3.0 with two EVB cards.
* 2x USB UART adapters
  * Recommended: https://www.amazon.com/DTECH-Adapter-Compatible-Windows-Genuine/dp/B0D97VR3CY/
  * Any M USB UART adapter will work as well.
* 4x F-to-F fly leads

## Setup

### Raspberry Pi

#### OS

Install the latest Raspberry Pi OS on a uSD card. Do not enable SPI or I2C.

Recommended packages: xxd, picocom, ack, libglib2.0-dev, liblua5.2-dev, tio

### EVB

Ensure power is supplied to the board and the USB UART cables are plugged in.

### Hardware Configuration

* Connect the USB uart cables to the UART header on each EVB. Plug these into the Raspberry Pi and note their path via /dev/serial/by-id/uart_...
  * Alternatively, just plug in the DB-9
* Using an F-to-F fly lead, connect RPi pin 12 (GPIO 18) to pin 2 of J1 (FWSPICK)
* Using an F-to-F fly lead, connect RPi pin 16 (GPIO 23) to pin 1 of J3 (SRST)

## Testing

All builds and tests are executed by the Raspberry Pi using a support script within the repository.

### Build

Build is managed via Cargo

```
\$ cargo build
```

### Package

First, we dump the ELF to a binary file:

```
$ cargo objcopy -- -O binary ast10x0.bin
```

The AST1060 UART requires a 4-byte header that informs the ROM of the size of the incoming payload.

```
 ./scripts/gen_uart_booting_image.sh ast10x0.bin uart_ast10x0.bin
 ```

### Execute

A support script in the repository, uart-test-exec.py, is designed to give both high level test contorl and fine-grained device control.

```
$ python3 uart-test-exec.py --help

usage: uart-test-exec.py [-h] [--srst-pin SRST_PIN] [--fwspick-pin FWSPICK_PIN] [--manual-srst {low,high,dl,dh}]
                         [--manual-fwspick {low,high,dl,dh}] [--sequence {fwspick-mode,normal-mode}] [-b BAUDRATE]
                         [--test-timeout TEST_TIMEOUT] [--log-file LOG_FILE] [--skip-uart] [-q] [--dry-run]
                         [uart_device] [firmware]

AST1060 UART Test Execution Script

positional arguments:
  uart_device           UART device path (e.g., /dev/ttyUSB0)
  firmware              Firmware binary file path

options:
  -h, --help            show this help message and exit
  --srst-pin SRST_PIN   SRST GPIO pin number (default: 23)
  --fwspick-pin FWSPICK_PIN
                        FWSPICK GPIO pin number (default: 18)
  --manual-srst {low,high,dl,dh}
                        Manually toggle SRST pin
  --manual-fwspick {low,high,dl,dh}
                        Manually toggle FWSPICK pin
  --sequence {fwspick-mode,normal-mode}
                        Run GPIO sequence
  -b BAUDRATE, --baudrate BAUDRATE
                        UART baud rate (default: 115200)
  --test-timeout TEST_TIMEOUT
                        Test execution monitoring timeout in seconds (default: 600)
  --log-file LOG_FILE   Log file path (auto-generated if not specified)
  --skip-uart           Skip all UART operations
  -q, --quiet           Run silently (no output)
  --dry-run             Show what would be done without executing

Examples:
  # Full test sequence
  ./uart-test-exec.py /dev/ttyUSB0 firmware.bin

  # Manual GPIO control
  ./uart-test-exec.py --manual-srst low
  ./uart-test-exec.py --manual-fwspick high

  # Sequence control
  ./uart-test-exec.py --sequence fwspick-mode
  ./uart-test-exec.py --sequence normal-mode

  # Custom pin numbers and timeout
  ./uart-test-exec.py --srst-pin 25 --fwspick-pin 20 --test-timeout 300 /dev/ttyUSB0 firmware.bin
```

To execute the test:

```
$ python3 uart-test-exec.py /dev/serial/by-id/usb-... uart_ast10x0.bin
```

The script will execute the following sequence:
- Assert SRST#
- Ensure FWSPICK is asserted (J1)
- Deassert SRST#
- Read uart waiting for "U"
- Upload the firmware
- Read UART output until "COMPLETE" is seen or a timeout happens.

While running the script looks for three tokens:

"panic" - The test executable has suffered an unrecoverable fault
"PASS" - A test has completed successfully
"FAIL" - A test has failed

If "panic" or "FAIL" is seen anywhere in the test, the script will print all UART output and return non-zero. Otherwise the script will return zero.

## Automation

When using a Github runner, it is recommended that the following things be set in environment variables and passed in via the YAML:
- UART device for the EVB
- SRST GPIO pin
- FWSPICK GPIO pin

### Variables

The YAML is configured to key on 3 variables for basic test config. These variables must be set on the runner.
* TARGET_UART - Path to the UART device attached to the DUT.
* FWSPICK_PIN - A number that corresponds to the GPIO pin to be toggled to control FWSPICK
* SRST_PIN - A number that corresponds to the GPIO pin to be toggled to control SRST

If FWSPICK_PIN is not set or is zero length, the script will assume the default pin settings are acceptable.