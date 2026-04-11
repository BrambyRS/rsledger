#!/bin/bash
set -e

# Test the code first
cargo test
# If tests pass, build and install the binary
cargo build --release
mv target/release/rsledger /opt/rsledger
