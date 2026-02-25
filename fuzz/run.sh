#!/usr/bin/env bash

set -euo pipefail

FUZZ_TARGETS=(
    "example"
)

cargo afl build

for FUZZ_TARGET in "${FUZZ_TARGETS[@]}"; do
    BIN="target/debug/$FUZZ_TARGET"
    OUT_DIR="$FUZZ_TARGET/out"
    CRASH_DIR="$OUT_DIR/default/crashes"

    rm -rf "$OUT_DIR"
    mkdir -p "$OUT_DIR"

    cargo afl fuzz -i "$FUZZ_TARGET/in" -o "$OUT_DIR" -V 30 "$BIN"

    if [ -d "$CRASH_DIR" ]; then
        mapfile -t CRASH_FILES < <(
            find "$CRASH_DIR" -type f ! -name README.txt ! -name "minified-*"
        )

        if [ "${#CRASH_FILES[@]}" -gt 0 ]; then
            for file in "${CRASH_FILES[@]}"; do
                base="$(basename "$file")"
                minified="$CRASH_DIR/minified-${base}"

                cargo afl tmin -i "$file" -o "$minified" "$BIN"

                "$BIN" < "$minified" || true

                cat "$minified" || true
            done

            exit 1
        fi
    fi
done
