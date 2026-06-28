#!/usr/bin/env bash
# Load optional local signing config (make/local.mk). Env vars already set are kept.

speaktype_load_local_config() {
    local local_mk="${1:-make/local.mk}"
    [[ -f "$local_mk" ]] || return 0
    set -a
    # shellcheck source=/dev/null
    source "$local_mk"
    set +a
}
