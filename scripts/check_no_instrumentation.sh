#!/usr/bin/env bash
# Verifies the testing-build isolation requirement: default builds
# contain no security-test instrumentation.
#
# 1. Builds the library with default features.
# 2. Asserts the produced rlib contains no `security_test` symbols.
# 3. Asserts no security-test binaries were built.
set -euo pipefail

cd "$(dirname "$0")/.."

# Build in an isolated target dir so stale artifacts from a
# feature-enabled build cannot mask (or fake) instrumentation.
TARGET_DIR="target/no-instrumentation"
export CARGO_TARGET_DIR="$TARGET_DIR"

cargo build --quiet

RLIB="$TARGET_DIR/debug/libwasmtiny.rlib"

if [[ ! -f "$RLIB" ]]; then
    echo "FAIL: expected library artifact at $RLIB (run cargo build first)"
    exit 1
fi

# rlibs are archives; nm lists member object symbols. Any
# security-test instrumentation would appear as symbols containing
# `security_test`.
if nm -A "$RLIB" 2>/dev/null | grep -q "security_test"; then
    echo "FAIL: default (no-feature) build contains security_test instrumentation:"
    nm -A "$RLIB" 2>/dev/null | grep "security_test" | head -5
    exit 1
fi

for BIN in wasmtiny-corpus-runner wasmtiny-fuzz; do
    if [[ -x "$TARGET_DIR/debug/$BIN" ]]; then
        echo "FAIL: security-test binary '$BIN' present in default build output"
        exit 1
    fi
done

echo "OK: default build contains no security-test instrumentation"
