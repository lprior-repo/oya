#!/usr/bin/env sh
set -eu

npm ci --prefer-offline --no-audit
node_modules/.bin/tailwindcss -i ./assets/tailwind.input.css -o ./assets/tailwind.css
export DX_HOME="${DX_HOME:-$PWD/target/dx-home}"
sh scripts/prepare-dx-tools.sh
env -u RUSTC_WRAPPER dx build --platform web --release
mkdir -p target/dx/oya-frontend/release/web/public/assets
cp assets/tailwind.css target/dx/oya-frontend/release/web/public/assets/tailwind.css
