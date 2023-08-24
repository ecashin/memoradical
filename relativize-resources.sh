#! /bin/sh
# This convenience script changes the absolute URLs to relative ones in trunk output.

set -xe

d=`dirname "$0"`
test -d "$d/dist"
cd "$d"
# (in case last time was "trunk serve" ...)
RUSTFLAGS=--cfg=web_sys_unstable_apis trunk build
ed dist/index.html <<EOF
g/\/memoradical-/s//.&/g
w
q
EOF
