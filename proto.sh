#!/usr/bin/env bash

set -e

if [ ! -d "$1" ]
then
    echo "$0 [protobuf repository]"
    exit 1
fi
PROTO="$(realpath "$1")"

make -C "${PROTO}"
rm -rf src/proto
cp -r "${PROTO}/result/rust" src/proto
