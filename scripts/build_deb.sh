#!/bin/bash

set -e
DIR=$(dirname $0)
. "$DIR/init.sh"

echo "Build debian package"
if [ ! -e ./debian/usr/local/bin/evercity-node ]; then
if [ ! -e ./target/release/evercity-node ]; then
    cargo build --release
fi
mv ./target/release/evercity-node ./debian/usr/local/bin
fi

dpkg-deb -b ./debian  evercity_node-2.1.0-rc1_amd64.deb
