#!/bin/sh
CARGO_FEATURES_PURE=1 cargo build --example tiled --target x86_64-pc-windows-gnu --features physics_debug &&
cp ./target/x86_64-pc-windows-gnu/debug/examples/tiled.exe .
exec ./tiled.exe
