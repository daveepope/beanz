#!/bin/bash -eu

cd "$SRC/beanz"
cargo fuzz build -O
HOST_TRIPLE="$(rustc -vV | awk '/^host: / { print $2 }')"
FUZZ_TARGET_OUTPUT_DIR="fuzz/target/${HOST_TRIPLE}/release"
for f in fuzz/fuzz_targets/*.rs; do
  FUZZ_TARGET_NAME="$(basename "${f%.*}")"
  cp "${FUZZ_TARGET_OUTPUT_DIR}/${FUZZ_TARGET_NAME}" "$OUT/"
done
