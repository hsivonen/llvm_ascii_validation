#!/bin/sh
RUSTC_BOOTSTRAP=1 RUSTFLAGS='-C opt-level=2' cargo test
