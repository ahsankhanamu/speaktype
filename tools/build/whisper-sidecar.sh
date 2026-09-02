#!/usr/bin/env bash

ensure_whisper_src() {
    local whisper_src="$1"
    if [ ! -f "$whisper_src/CMakeLists.txt" ]; then
        echo "→ Cloning whisper.cpp..."
        rm -rf "$whisper_src"
        mkdir -p "$(dirname "$whisper_src")"
        git clone --depth 1 https://github.com/ggerganov/whisper.cpp.git "$whisper_src"
    fi
}

whisper_cmake_args() {
    local -n _out=$1
    _out=(-DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=OFF -DGGML_METAL=ON)

    # GGML_NATIVE: compile/run CPU feature probes for this machine.
    # ON  — best CPU tuning; fresh cmake on Apple Silicon can hang on SVE probes
    #       (safe if whisper.cpp/build/ is already configured, e.g. your M5 Mini).
    # OFF — skip runtime probes; compile-only checks. Safe for fresh clones.
    # Set via export or make/local.mk: WHISPER_GGML_NATIVE=ON|OFF
    local native="${WHISPER_GGML_NATIVE:-}"
    if [ -z "$native" ] && [ "$(uname -s)" = "Darwin" ]; then
        native=OFF
    fi
    case "$native" in
        ON|on|1|true|TRUE)  _out+=(-DGGML_NATIVE=ON) ;;
        OFF|off|0|false|FALSE) _out+=(-DGGML_NATIVE=OFF) ;;
        "" ) ;;
        * )
            echo "ERROR: WHISPER_GGML_NATIVE must be ON or OFF (got: $native)" >&2
            return 1
            ;;
    esac
}
