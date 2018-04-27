#!/bin/sh
rm -rf target/
RUSTC_BOOTSTRAP=1 RUSTFLAGS='-C opt-level=2 --emit asm' cargo build --release
find target | grep '\.s$' | xargs less
