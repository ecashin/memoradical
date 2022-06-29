#! /bin/sh
# This convenience script changes the absolute URLs to relative ones in trunk output.

set -xe

d=`dirname "$0"`
test -d "$d/dist"
cd "$d"
trunk build  # (in case last time was "trunk serve")
ed dist/index.html <<EOF
g/\/index-/s//.&/g
w
q
EOF
