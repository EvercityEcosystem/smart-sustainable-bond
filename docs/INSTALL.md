# Initial configuration of evercity node
  you need:
   - Linux server with 4 CPU Intel Xeon or AMD EPYC
   - 8-16 Gb RAM
   - 400 Gb free space on SSD

# Build from GitHub on clean Ubuntu 18.04

Warning! Do not use root account for running and support blockchain node

## Install required packages

```bash
# renew list of packages
sudo apt update

# install cmake, libssl, developer libraries, etc...
sudo apt install -y cmake pkg-config libssl-dev git build-essential clang libclang-dev curl libz-dev
```

## Download and install Rust

```bash
# download and run install script. Proceed with (Default) installation
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# "turn on" Rust environment in current shell
source $HOME/.cargo/env

# update to lastest version of Rust
rustup update
```

## Clone repo, install Rust compiler and components, build

Clone last version of node
```bash
git clone https://github.com/EvercityEcosystem/smart-sustainable-bond.git
cd smart-sustainable-bond
```

Build node
```
# add target "wasm32-unknown-unknown" for current rust-toolchain
rustup target add wasm32-unknown-unknown

# version of Rust and compоnents are configured in "rust-toolchain.toml" file
cargo build --release
```

# Running and testing of the node

## Running manually

```bash
# node binary after build is usually in ./target/release/evercity-node
cd ./target/release

# simple run with all defaults
./evercity-node

# example of run command ("dev" variant of node, HTTP CORS headers (allowing WS connections from everywhere))
./evercity-node --dev --ws-port 9944 --rpc-cors all
```

## Checking, that blockchain goes forward

You should see in log records like these:

```
...
Feb 17 16:54:04.860  INFO 💤 Idle (0 peers), best: #37 (0x6d7b…51b1), finalized #35 (0x628b…d0ab), ⬇ 0.1kiB/s ⬆ 0.2kiB/s
...
Feb 17 16:55:39.869  INFO 💤 Idle (0 peers), best: #41 (0xf6cb…55ef), finalized #39 (0xe921…f720), ⬇ 0.1kiB/s ⬆ 0.2kiB/s
...
```
where "finalized" block numbers are increasing. Finalization is possible only when several validators agreed on the block (in case of one validator in "dev" mode finalization always works)

## Working with development node

Development node is a blockchain with 1 validator(itself), running by default on ws://127.0.0.1:9944 endpoint.
Examples:
```bash
# run "dev" node on ws://127.0.0.1:9944
./evercity-node --dev 

# fully delete current "dev" chain, next launch will be from block #0. Requires confirmation
./evercity-node purge-chain --dev
```

## Running testcase scenarios with local "dev" blockchain

Testcase scenarios are contained in Node.JS script, interacting with local 127.0.0.1:9944 "dev" node.
Scenarios init user balances, assigns roles to accounts, create bonds and perform all actions of real
blockchain users(invest, receive coupon yield). Scenarios use local "dev" chain by default


### 1. (Optional) Purge previously created "dev"chain (requires confirmation) and run fresh new chain
```bash
./evercity-node purge-chain --dev
./evercity-node --dev
```


### 2. Checkout and build testcase scenarios

Install correct version of Node.JS and run test scenarios

```bash
git clone https://github.com/EvercityEcosystem/ssb-testcases-generator.git
cd ssb-testcases-generator
npm install
```

### 3. Run testcases init and scenarios. Full list of possible scenarios is in "package.json" file.
Some scenarios require a lot of time to pass (can emulate several time-periods of 1-2 minutes length each).
You can modify you scenarios or output additional info by editing "index.js" file.
```bash
npm run init 		# init balances and roles
npm run scenario1 	# run scenario1
npm run scenario2 	# run scenario2 
```

### 4. Repeat
