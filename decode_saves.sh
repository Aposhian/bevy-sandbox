#!/bin/sh

mkdir -p decoded_saves

for save in saves/*.binpb; do
    name=$(basename "$save" .binpb)
    protoc --decode=save.SaveGame --proto_path=./proto ./proto/save.proto < "$save" > "decoded_saves/${name}.txtpb"
    echo "Decoded $save -> decoded_saves/${name}.txtpb"
done
