#!/bin/sh
CARGO_FEATURES_PURE=1 cargo build --target x86_64-pc-windows-gnu "$@" &&
cp ./target/x86_64-pc-windows-gnu/debug/bevy_sandbox.exe . &&
exec ./bevy_sandbox.exe
