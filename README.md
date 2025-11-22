# Minimalist binary crate for ASPEED

> Based on the  template for building applications for ARM Cortex-M microcontrollers


## Dependencies

To build embedded programs using this template you'll need:

- Rust  toolchain. 

- `rust-std` components (pre-compiled `core` crate) for the ARM Cortex-M
  targets. Run:

``` console
$ rustup target add thumbv7em-none-eabihf
```

## Building this app

$ cargo build --release

## Using this app

1. **Start the JLinkGDBServer**:
    ```sh
    JLinkGDBServer -device cortex-m4 -if swd
    ```

2. **Run the program with GDB**:
    ```sh
    gdb-multiarch target/thumbv7em-none-eabihf/release/aspeed-ddk
    
    ```

3. **Enable semihosting in GDB**:
    ```gdb
    target remote :2331
    monitor semihosting IOClient 2
    load
    continue
    ```

## Generate Image for AST1060

- Generate Image for Programming

   ```sh
   cargo build;cargo objcopy -- -O binary ast10x0.bin
   ```

- Generate Image for Boot from UART
   ```sh
   cargo build;cargo objcopy -- -O binary ast10x0.bin
   scripts/gen_uart_booting_image.py ast10x0.bin uart_ast10x0.bin
   ```

## Runing the app on QEMU

### Build QEMU
1. git clone https://github.com/qemu/qemu
2. Run the following commands to build qemu
   ```sh
   mkdir build
   cd build
   ../qemu/configure --target-list=arm-linux-user,arm-softmmu,aarch64-softmmu,aarch64-linux-user,riscv32-softmmu --enable-docs --enable-slirp --enable-gcrypt
   make -j 4
   ```

### Run
1. Run the image in QEMU using `ast1030-evb` machine
   ```sh
   qemu-system-arm -M ast1030-evb -nographic -kernel ~/work/rot/aspeed/aspeed-rust/target/thumbv7em-none-eabihf/debug/aspeed-ddk
   Hello, world!
   aspeed_ddk!
   ```

## Running the app on Hardware

### Host Platform

The recommended host platform is a Raspberry Pi, per ASpeed. Connecting two GPIO from the Pi to SRST pin 1 and FWSPICK pin 2 will allow the upload script to manage UART boot state and device ready. Check the upload script for the correct pins.

### Test Execution

Using the UART boot image created above, the uart-test-exec.py script will load the binary on the device and monitor all output.

$ python3 ./scripts/uart-test-exec.py <path to UART> uart_ast10x0.bin

The script will toggle the GPIOs in the necessary order to reset the device into UART boot mode, then once "U" is observed from the device, it will upload the binary and begin observing the output. All test output will be printed to the screen and logged for later use.

Running

$ pythone3 ./scripts/uart-test-exec.py --help

will provide details on multiple additional options including manual toggling of GPIOs, altering the GPIO number, and other configuration options (UART baud rate, timeouts, quiet operation, etc.)

