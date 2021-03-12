#!/bin/sh

set -e

echo "Initializing build environment"

rustup update nightly
rustup update stable
rustup target add wasm32-unknown-unknown
