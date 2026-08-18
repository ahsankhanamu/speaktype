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

speaktype_load_rust_env() {
    if [ -f "${HOME}/.cargo/env" ]; then
        # shellcheck source=/dev/null
        source "${HOME}/.cargo/env"
    fi
}

# Tauri bundler runs `xattr -cr`; Homebrew's python-xattr package shadows
# /usr/bin/xattr and does not support -r, which breaks app bundling.
speaktype_prefer_system_xattr() {
    if [ -x /usr/bin/xattr ] && /usr/bin/xattr -h 2>&1 | grep -q ' \[-r\]'; then
        PATH="/usr/bin:/bin:/usr/sbin:/sbin:${PATH}"
        export PATH
    fi
}
