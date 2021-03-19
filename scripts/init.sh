#!/bin/sh

set -e

echo "Initializing build environment"

rustup update nightly-2021-03-03
rustup target add wasm32-unknown-unknown
