# Initial configuration of evercity node
  you need:
   - Linux server with 4 CPU Intel Xeon or AMD EPYC
   - 16 Gb RAM
   - 400 Gb free space on SSD

# Build from GitHub on clean Ubuntu 18.04

```bash
# (!) In this document we work as user "evercity" with $HOME=/home/evercity and SUDO privilege
```

## Install required packages
```bash
# renew list of packages
sudo apt update

# install cmake, libssl, developer libraries, etc...
sudo apt install -y cmake pkg-config libssl-dev git build-essential clang libclang-dev curl libz-dev
```

## Install Rust compiler and components for building node
```bash
# Download and install Rust
sudo curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh # proceed with (Default) installation

# "turn on" Rust environment in current shell
source $HOME/.cargo/env

# Update to lastest version of Rust
rustup update

## Install Rust nightly version(select needed "nightly" release

```bash
# set wanted nightly version of Rust
# (!) I set version to 'nightly-2020-10-01' because with newer version (f.e. 'nightly-2021-02-13')
# there is a compilation error
NIGHTLY_RELEASE=nightly-2020-10-01

# install and make default nightly Rust
rustup install $NIGHTLY_RELEASE
rustup default $NIGHTLY_RELEASE

# Add WASM compilation target
rustup target add wasm32-unknown-unknown --toolchain $NIGHTLY_RELEASE
```

## Clone and build node
```bash
# clone needed branch of "evercity-substrate" ("master" by default)
git clone https://github.com/EvercityEcosystem/evercity-substrate.git
cd evercity-substrate

# build node using toolchain $NIGHTLY_RELEASE with target "wasm32-unknown-unknown" installed
WASM_BUILD_TOOLCHAIN=$NIGHTLY_RELEASE cargo build --release
```

## Running a node manually
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

## Work with development node

Development node is a blockchain with 1 validator(itself), running by default on ws://127.0.0.1:9944 endpoint.
Examples:
```
# run "dev" node on ws://127.0.0.1:9944
./evercity-node --dev 
# fully delete current "dev" chain, next launch will be from block #0
./evercity-node purge-chain --dev # предлагает удалить всю цепочку
```


