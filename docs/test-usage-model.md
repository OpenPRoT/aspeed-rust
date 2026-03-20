# I2C Slave API — Test Usage Model

Date: 2026-03-20

## Background

`pre_arm_tx()` and `rearm_rx()` were added to `Ast1060I2c` to eliminate the raw
PAC writes that `backend-aspeed` needed for the slave read-response path.  The
existing functional tests in `i2c_master_slave_test.rs` do not exercise this path
in a way that matches how the consumer actually calls the DDK.  This document
describes the mismatch, the corrected real-world call sequence, and the
implementation that was applied to the test file.

## Terminology

**Handle** (`Ast1060I2c<'_>`) — a lightweight, stack-allocated struct that holds
references to the controller's MMIO register blocks.  It carries no hardware
state of its own: creating or dropping a handle does not read or write any
register.  All hardware state lives in the peripheral itself.  Multiple handles
can be created in sequence from the same register pointers (`from_initialized()`)
without disturbing the hardware, which is why the per-phase handle split is safe.

**Pre-arm** — loading the hardware I2C slave TX buffer *and* writing the slave
command register (`i2cs28`) to set `TX_BUFF_EN` in a single register write, before
the master issues a read transaction.  Because the AST1060 begins clocking SCL
immediately when it detects a read address match, the TX buffer must already
contain the correct data at that moment; there is no opportunity to load it after
`ReadRequest` fires.  Pre-arming is the only safe mechanism for a slave to respond
to master reads.

**Rearm RX** — after a `DataSent` event the hardware clears `RX_BUFF_EN`,
preventing the slave from receiving further writes.  Rearming writes `i2cs28` with
`RX_BUFF_EN` set (without `TX_BUFF_EN`) to restore the receive-ready state before
the master's next write transaction arrives.

## Structural Mismatch with the Old Test

`slave_event_loop()` used a **reactive** pattern — it called `slave_write()` inside
the `DataSent` handler.  `DataSent` fires *after* the master has already clocked
out the data, so this write had no effect.  The consumer (`backend-aspeed`) uses a
**proactive** pattern — it loads TX data *before* the master starts clocking, then
relies on the hardware to respond automatically.

| Aspect | Old DDK test | Consumer (`backend-aspeed`) |
|---|---|---|
| Handle lifetime | Single `Ast1060I2c::new()` held for whole loop | Fresh `from_initialized()` per operation boundary |
| TX loading | `slave_write()` called in `DataSent` — too late | `pre_arm_tx()` called after `DataReceived`, before read |
| RX re-arm | Not done | `rearm_rx()` called after `DataSent` before returning |
| `ReadRequest` handler | Prints a message | `continue` — hardware already has the data |
| Response data | Static `TEST_PATTERN_READ` pre-armed at boot | Computed from received bytes after each write |

The old test therefore exercised a usage path that does not work and silently
missed the path that does.

## Real-World Call Sequence

The MCTP transport layer (`services/mctp/transport-i2c`) drives the I2C slave
through _separate_ IPC calls for `SlaveSetResponse` and `SlaveWaitEvent`.  The
master **always writes first** (MCTP request), after which the device computes a
response, arms TX, then the master reads back (MCTP response delivery).  There is
no pre-arm at startup.

```
[startup — no pre_arm_tx; master always writes first in MCTP]

loop {
    Phase A: slave_wait_event() → DataReceived     (master write = MCTP request)
    Phase B: MCTP stack computes response from received bytes
    Phase C: slave_set_response(response)          ← pre_arm_tx() happens here
    Phase D: slave_wait_event() → DataSent         (master read = response delivery)
    Phase E: rearm_rx()                            ← back to Phase A
}
```

Each `slave_wait_event` and `slave_set_response` call creates a fresh
`from_initialized()` handle, because `AspeedI2cBackend` is called once per IPC
dispatch.

### Timing window between `rearm_rx()` and `pre_arm_tx()`

After `rearm_rx()` only RX is armed; TX is not pre-armed again until the next
`slave_set_response()`.  In MCTP this is safe because DSP0236 mandates write-before-
read ordering — the master will not issue a read until after its write has been
acknowledged by the device.  Any test that skips Phase A and pre-arms TX at startup
cannot detect a regression in this window.

## Correct Test Structure

### Phase A — wait for `DataReceived` (fresh `from_initialized` handle)

```rust
let mut h = Ast1060I2c::from_initialized(&controller, config);
// ... poll handle_slave_interrupt() until DataReceived, then slave_read()
```

### Phase B — compute response from received data

```rust
// Derive response from what was received so stale-data bugs are detectable.
for (i, b) in rx_buf[..rx_len].iter().enumerate() {
    response[i] = b.wrapping_add(1);
}
```

### Phase C — `pre_arm_tx()` (fresh `from_initialized` handle)

```rust
let mut h = Ast1060I2c::from_initialized(&controller, config);
h.pre_arm_tx(&response[..rx_len])?;
// drop h — hardware TX buffer and i2cs28 state persists
```

### Phase D — wait for `DataSent` (fresh `from_initialized` handle)

```rust
let mut h = Ast1060I2c::from_initialized(&controller, config);
// ... poll until ReadRequest (continue) or DataSent (break)
```

### Phase E — `rearm_rx()` (same handle)

```rust
h.rearm_rx();
// drop h, loop back to Phase A
```

### Why `from_initialized()` per phase matters

`Ast1060I2c` carries no slave runtime state — it is a pure register-pointer
wrapper.  A handle created via `from_initialized()` after `pre_arm_tx()` was
called on a different handle will still see the hardware state left by
`pre_arm_tx()` (TX buffer loaded, `i2cs28` written).  The split-handle pattern
therefore works correctly at the hardware level.

However, if the test used a single long-lived `new()` handle, it could not detect a
regression where `from_initialized()` inadvertently clears `i2cs28` or resets
buffer state.

## Implementation Applied

The following changes were made to
`aspeed-rust/src/tests/functional/i2c_master_slave_test.rs`:

### Fix: removed `slave_write()` from `DataSent`

The broken call `slave.slave_write(&TEST_PATTERN_READ)` was removed from the
`DataSent` arm of `slave_event_loop()`.  A comment was added explaining why
`DataSent` is the wrong moment to write TX data.

### New: `run_slave_request_response_test()`

Added a new public test function that:

- Calls `Ast1040I2c::new()` + `configure_slave()` + `enable_slave()` once, then
  drops the init handle.
- Does **not** call `pre_arm_tx()` at startup.
- Runs `TRANSACTION_CYCLES` (= 5) iterations of the Phase A–E loop.
- Creates a fresh `from_initialized()` handle for each phase.
- Computes the response as `received_byte + 1` for each byte so that the external
  master can verify it received the correct data.
- Calls `pre_arm_tx()` and `rearm_rx()` exclusively via DDK methods — no raw PAC
  writes.

### Update: `run_master_slave_tests()` help text

Added a description of the new test to the info/help function.

## External Master Requirements for `run_slave_request_response_test`

The external master (AST2600, another EVB, or bus master) must:

1. Write N bytes (1–32) to slave address `0x50`.
2. Read N bytes back and verify each byte equals the written byte + 1.
3. Repeat 5 times.


Date: 2026-03-20

## Background

`pre_arm_tx()` and `rearm_rx()` were added to `Ast1060I2c` to eliminate the raw
PAC writes that `backend-aspeed` needed for the slave read-response path.  The
existing functional tests in `i2c_master_slave_test.rs` do not exercise this path
in a way that matches how the consumer actually calls the DDK.  This document
describes the mismatch and what a representative test should look like.

## Structural Mismatch with the Current Test

`slave_event_loop()` uses a **reactive** pattern — it calls `slave_write()` inside
the `DataSent` handler.  `DataSent` fires *after* the master has already clocked
out the data, so this write has no effect.  The consumer (`backend-aspeed`) uses a
**proactive** pattern — it loads TX data *before* the master starts clocking, then
relies on the hardware to respond automatically.

| Aspect | Current DDK test | Consumer (`backend-aspeed`) |
|---|---|---|
| Handle lifetime | Single `Ast1060I2c::new()` held for whole loop | Fresh `from_initialized()` per operation boundary |
| TX loading | `slave_write()` called in `DataSent` — too late | `pre_arm_tx()` called before entering the event loop |
| RX re-arm | Not done | `rearm_rx()` called after `DataSent` before returning |
| `ReadRequest` handler | Prints a message | `continue` — hardware already has the data |

The current test therefore exercises a usage path that does not work and silently
misses the path that does.

## Correct Call Sequence

The three-phase cycle the consumer uses, which the test must replicate:

### Phase 1 — Pre-arm (before polling)

```rust
// After configure_slave() + enable_slave(), load TX data once.
// This is the only window where the hardware accepts pre-loaded TX data —
// before the master begins clocking SCL.
slave.pre_arm_tx(&response_data)?;
```

### Phase 2 — Event loop

```rust
// Create a fresh handle via from_initialized() per logical operation,
// matching the consumer's per-call handle model.
let mut i2c = Ast1060I2c::from_initialized(&controller, config);

const POLL_BUDGET: usize = 10_000;
for _ in 0..POLL_BUDGET {
    match i2c.handle_slave_interrupt() {
        Some(SlaveEvent::ReadRequest) => {
            // TX was pre-armed — hardware responds on the next clock edge.
            // Do nothing here.
        }
        Some(SlaveEvent::DataSent { len }) => {
            // Master has received the data. Re-arm RX so it can write next.
            i2c.rearm_rx();
            // Return to the outer loop; caller will pre_arm_tx() again.
            break;
        }
        Some(SlaveEvent::DataReceived { len: _ }) => {
            let n = i2c.slave_read(&mut rx_buf)?;
            // process received data …
            break;
        }
        Some(SlaveEvent::Stop) | None => {}
        _ => {}
    }
}
```

### Phase 3 — Loop back

The outer loop calls `pre_arm_tx()` again with the next response payload before
re-entering Phase 2.  This mirrors `slave_set_response()` → `slave_wait_event()`
back-to-back in the server dispatch loop.

## Why `from_initialized()` in the Test Matters

`Ast1060I2c` carries no slave runtime state — it is a pure register-pointer
wrapper.  A handle created via `from_initialized()` *after* `pre_arm_tx()` was
called on a different handle will still observe the hardware state left by
`pre_arm_tx()` (TX buffer loaded, `i2cs28` written).  The split-handle pattern
therefore works correctly at the hardware level.

However, the test must exercise this split to verify it.  If the test uses a single
long-lived `new()` handle throughout, it cannot detect a regression where
`from_initialized()` inadvertently clears `i2cs28` or resets buffer state — which
would silently break the consumer without failing the test.

## Changes Needed in the Existing Test File

1. **Remove** the `slave_write()` call from the `DataSent` arm of
   `slave_event_loop()` — it tests a path that is both broken and unused by any
   current consumer.

2. **Add** a new test function (e.g. `run_slave_pre_arm_test()`) that:
   - Calls `Ast1060I2c::new()` + `configure_slave()` + `enable_slave()` once.
   - Calls `pre_arm_tx(TEST_PATTERN_READ)` before entering the poll loop.
   - Creates a new `from_initialized()` handle at the top of each iteration.
   - On `ReadRequest`: continues without touching TX.
   - On `DataSent`: calls `rearm_rx()`, breaks the inner loop, then calls
     `pre_arm_tx()` again at the top of the outer iteration.
   - Verifies that the external master receives `TEST_PATTERN_READ` on each read
     transaction.
