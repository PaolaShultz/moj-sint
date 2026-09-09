#!/bin/sh
# Focused native sanitizer checks, outside Cargo's Rust allocator instrumentation.
set -eu
cd "$(dirname "$0")/.."
open303_check_dir=$(mktemp -d)
trap 'rm -rf "$open303_check_dir"' EXIT HUP INT TERM
${CXX:-c++} -std=c++17 -O1 -g -fno-omit-frame-pointer \
  -fsanitize=address,undefined -fno-sanitize-recover=all \
  -Wall -Wextra -Werror -Wno-unused-parameter \
  -Ivendor/open303/src -Inative/open303 \
  tests/open303_native.cpp vendor/open303/src/*.cpp \
  -o "$open303_check_dir/native-test"
"$open303_check_dir/native-test" bounds
"$open303_check_dir/native-test" contracts
if [ "${1:-}" = "--spectral" ]; then
  "$open303_check_dir/native-test" spectrum
fi
