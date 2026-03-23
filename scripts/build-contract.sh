#!/bin/bash
set -e

cargo build --target riscv32emac-unknown-none-polkavm --release "$@"
