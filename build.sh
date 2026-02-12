#!/bin/bash
# Build for MIPS MT7688AN (OpenWrt)

cross +nightly build --target mipsel-unknown-linux-musl --release
