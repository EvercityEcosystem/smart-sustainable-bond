# Evercity Substrate Node

Blockchain node of Evercity project, based on Parity Substrate with evercity-pallet module, implementing business logic of Evercity green bond project

### Build

```bash
git clone https://github.com/EvercityEcosystem/evercity-substrate.git
cd evercity-substrate
cargo build --release
```

## Run

### Single Node Development Chain

Purge any existing dev chain state:

```bash
./target/release/evercity-node purge-chain --dev
```

#### Remove all chains with all data

| [WARNING] - all chains data is usually located in ```$HOME/.local/share/evercity-node/chains/*```.  |
| --- |
Removing of all chains: "dev", "local-testnet", and any others to launch all chains from block "0" can be made by:
```
rm -rf $HOME/.local/share/evercity-node/chains/*
```

#### Start a dev chain:

```bash
./target/release/evercity-node --dev
```

Or, start a dev chain with detailed logging:

```bash
RUST_LOG=debug RUST_BACKTRACE=1 ./target/release/evercity-node -lruntime=debug --dev
```

## [TODO] to be continued...
