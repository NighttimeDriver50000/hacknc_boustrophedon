#!/bin/sh
im="$(readlink -e "$1")"
cd "$(dirname "$0")"
cargo run "$im" | ./ShowCriticalPoints.py "$im"
