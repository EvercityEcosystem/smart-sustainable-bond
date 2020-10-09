# Evercity Substrate Node

Blockchain node of Evercity project, based on Parity Substrate with evercity-pallet module, implementing business logic of Evercity green bond project

### Build

```bash
cargo build --release
```

## Run

### Single Node Development Chain

Purge any existing dev chain state:

```bash
./target/release/evercity-node purge-chain --dev
```

Start a dev chain:

```bash
./target/release/evercity-node --dev
```

Or, start a dev chain with detailed logging:

```bash
RUST_LOG=debug RUST_BACKTRACE=1 ./target/release/evercity-node -lruntime=debug --dev
```

## [TODO] to be continued...
