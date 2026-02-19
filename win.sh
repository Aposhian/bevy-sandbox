#!/bin/sh
CARGO_FEATURES_PURE=1 cargo build --example $1 --target x86_64-pc-windows-gnu &&
cp ./target/x86_64-pc-windows-gnu/debug/examples/$1.exe .
exec ./$1.exe 
