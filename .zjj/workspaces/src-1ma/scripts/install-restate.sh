#!/bin/bash
set -e

BIN="${HOME}/.local/bin"
RESTATE_PLATFORM="x86_64-unknown-linux-musl"

mkdir -p "bin"

curl -L --remote-name-all "https://restate.gateway.scarf.sh/latest/restate-{server,cli}-${RESTATE_PLATFORM}.tar.xz"

tar -xvf "restate-server-${RESTATE_PLATFORM}.tar.xz" --strip-components=1 "restate-server-${RESTATE_PLATFORM}/restate-server"
tar -xvf "restate-cli-${RESTATE_PLATFORM}.tar.xz" --strip-components=1 "restate-cli-${RESTATE_PLATFORM}/restate"

chmod +x restate restate-server
mv restate bin/
mv restate-server bin/

rm -f "restate-server-${RESTATE_PLATFORM}.tar.xz" "restate-cli-${RESTATE_PLATFORM}.tar.xz"

echo "Restate installed to bin/"
