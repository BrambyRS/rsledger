#!/bin/bash
set -e

# Build and test the project
cargo test --release
# If tests pass, install the binary to /opt/rsledger
mv target/release/rsledger /opt/rsledger
