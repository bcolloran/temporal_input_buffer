# Temporal Input Buffer

Temporal Input Buffer is a small Rust library for synchronizing player inputs in
multiplayer games.  It provides data structures for collecting per-player input
states, finalizing them on a host, and sharing finalized slices with peers.

The crate is written with deterministic lockstep style games in mind.  Each
input implements the `SimInput` trait which allows conversion to and from a
fixed-size byte representation.  Input buffers keep track of finalized and
non-finalized ticks and can predict missing inputs using a simple
last-observation carried forward strategy.

Key modules:

- `input_trait` – defines the `SimInput` trait used by inputs.
- `input_buffer` – handles a single player's input history.
- `multiplayer_input_buffer` – collections of buffers for all players.
- `multiplayer_input_manager` – common logic shared by host and guest managers.
- `multiplayer_input_manager_host` / `multiplayer_input_manager_guest` – manage
  communication of input slices and acknowledgements between peers.
- `input_messages` – serializable message types used over the network.
- `button_state` and `ewma` – helper utilities used by the managers.

The repository also contains extensive unit tests demonstrating usage with a
simple `PlayerInput` structure.

To run the tests:

```bash
cargo test
```

## Coverage

Install the coverage helper once:

```bash
cargo install cargo-llvm-cov
```

Then collect coverage for the full workspace:

```bash
cargo coverage
```

The alias expands to `cargo llvm-cov --workspace --all-targets --lcov
--output-path target/llvm-cov/lcov.info --html`, producing an HTML report under
`target/llvm-cov/html/` and an LCOV file at `target/llvm-cov/lcov.info`. Open
the HTML report with `open target/llvm-cov/html/index.html` (or your
platform's equivalent) to explore the results in a browser.

This project requires the nightly Rust toolchain because it uses the
`duration_millis_float` feature for timing utilities.  The required toolchain is
specified in `rust-toolchain.toml` and will be downloaded automatically by
`rustup` when building.
