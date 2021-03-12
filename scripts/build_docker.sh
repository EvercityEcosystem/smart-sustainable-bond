#!/bin/bash

set -e

DIR=$(dirname $0)
. "$DIR/init.sh"

echo "Build docker image"
if [ ! -e ./docker/evercity-node ]; then
if [ ! -e ./target/release/evercity-node ]; then
    cargo build --release
fi
mv ./target/release/evercity-node ./docker/
fi
cd ./docker
docker build . -t evercity/evercity_node:latest
