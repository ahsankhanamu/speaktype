# Setup SpeakType on a new Mac

End-to-end setup for developers on an **Apple Silicon Mac**. Covers the Python CLI, widget build tools, whisper.cpp sidecar, and Developer ID signing for release builds.

> **End users:** download the DMG from [Releases](https://github.com/ahsankhanamu/speaktype/releases) instead — see [macos-widget install](install/macos-widget.md).

Related docs: [Getting started](getting-started.md) · [Build widget from source](build-widget-from-source.md) · [macOS CLI](install/macos-cli.md)

---

## 0. Notes

| Topic | Detail |
|-------|--------|
| **Account role** | Creating Developer ID certificates requires **Account Holder** on the Apple Developer team (or cloud-managed Developer ID access). |
| **Signing keys** | Developer ID certificates live in the local Keychain (not iCloud). Export a `.p12` backup after setup. |
| **Certificate limit** | Apple allows up to **5** Developer ID Application certificates per team. |
| **Hardware mic** | Some Macs (e.g. Mac mini) have **no built-in microphone**. Use a USB mic, headset, or AirPods when testing recording. |
| **`tools/build/build.sh`** | Builds the whisper.cpp sidecar but does **not** clone the repo. Clone it first (step 4). |

---

## 1. Clone the repo

```bash
git clone https://github.com/ahsankhanamu/speaktype.git
cd speaktype
```

---

## 2. Homebrew tools

```bash
# CLI audio + widget sidecar build
brew install python@3.12 portaudio cmake librsvg

# Optional: only if your shell config uses direnv
brew install direnv
```

`librsvg` provides `rsvg-convert`, required when `make build` renders the DMG background.

### Python PATH note

If another tool (e.g. pyenv) shadows Homebrew’s `python3.12`, call it explicitly:

```bash
/opt/homebrew/bin/python3.12 --version   # expect 3.12.x
```

---

## 3. Python CLI

From the repo root:

```bash
/opt/homebrew/bin/python3.12 -m venv .venv
.venv/bin/pip install --upgrade pip
.venv/bin/pip install -e ".[all]"
```

Or, if `python3.12` on your `PATH` is a working 3.10+ interpreter:

```bash
make install
```

Verify:

```bash
source .venv/bin/activate
speaktype --help
python -c "import sounddevice as sd; print(sd.query_devices())"
```

Run:

```bash
speaktype          # default hotkey: F9
```

Grant **Microphone** and **Accessibility** to your terminal app — see [permissions](permissions.md).

---

## 4. Widget build prerequisites

| Tool | Install |
|------|---------|
| Xcode Command Line Tools | `xcode-select --install` |
| Rust (1.93+) | [rustup](https://rustup.rs/) — toolchain pinned in `apps/widget/rust-toolchain.toml` |
| Tauri CLI 2 | `cargo install tauri-cli` |
| cmake | `brew install cmake` (step 2) |

Verify:

```bash
rustc --version
cargo --version
cargo tauri --version
cmake --version
xcode-select -p
```

### Clone whisper.cpp (required before `make build`)

`tools/build/build.sh` expects source at:

`apps/widget/src-tauri/scripts/build/whisper.cpp`

```bash
WHISPER_SRC="apps/widget/src-tauri/scripts/build/whisper.cpp"

if [ ! -f "$WHISPER_SRC/CMakeLists.txt" ]; then
  rm -rf "$WHISPER_SRC"
  mkdir -p "$(dirname "$WHISPER_SRC")"
  git clone --depth 1 https://github.com/ggerganov/whisper.cpp.git "$WHISPER_SRC"
fi
```

Dev loop (after the sidecar binary exists — see [build guide](build-widget-from-source.md)):

```bash
make dev
```

Release pipeline (needs signing — steps 5–6):

```bash
make build
```

---

## 5. Apple signing config (`make/local.mk`)

```bash
cp make/config.example make/local.mk
```

Edit `make/local.mk` (gitignored):

```bash
export APPLE_DEVELOPER_ID="Developer ID Application: Your Name (TEAMID)"
export APPLE_TEAM_ID="TEAMID"
export NOTARY_PROFILE="SpeakType"
```

Use your Apple Developer team name and Team ID. After step 6, `APPLE_DEVELOPER_ID` must match `security find-identity` exactly.

Check current identities:

```bash
security find-identity -v -p codesigning
```

If none are listed, continue with step 6. If you already have a Developer ID identity (or a `.p12` to import), skip to the import/backup section at the end of step 6.

---

## 6. Create a Developer ID Application certificate

### 6.1 Generate CSR + private key

```bash
SIGN_DIR="$HOME/Desktop/speaktype-signing"
mkdir -p "$SIGN_DIR"
cd "$SIGN_DIR"

# Replace email / CN with your details
openssl genrsa -out DeveloperID.key 2048
chmod 600 DeveloperID.key

openssl req -new -key DeveloperID.key \
  -out DeveloperID.certSigningRequest \
  -subj "/emailAddress=you@example.com/CN=Your Name/C=US"

openssl req -in DeveloperID.certSigningRequest -noout -subject
open "$SIGN_DIR"
open "https://developer.apple.com/account/resources/certificates/add"
```

### 6.2 Apple Developer portal

1. Certificates → **+**
2. Under Software, choose **Developer ID Application** (G2 intermediate if asked)
3. Upload `DeveloperID.certSigningRequest`
4. Download the `.cer` into the **same** folder (`speaktype-signing/`)

Typical filename: `developerID_application.cer`

### 6.3 Pair key + cert, import into login keychain

OpenSSL 3’s default PKCS#12 is rejected by macOS (`MAC verification failed`). Use **legacy PBE** options.

Also install Apple’s **Developer ID G2** intermediate, or `find-identity` may still show **0** identities after import.

```bash
SIGN_DIR="$HOME/Desktop/speaktype-signing"
cd "$SIGN_DIR"

CER="developerID_application.cer"   # adjust if Apple used a different name
test -f "$CER" && test -f DeveloperID.key

# DER .cer → PEM (falls back to PEM if already PEM)
openssl x509 -inform DER -in "$CER" -out DeveloperID.pem 2>/dev/null \
  || openssl x509 -inform PEM -in "$CER" -out DeveloperID.pem

openssl x509 -in DeveloperID.pem -noout -subject -issuer -dates

# Key must match certificate
CERT_MOD=$(openssl x509 -in DeveloperID.pem -noout -modulus | openssl md5)
KEY_MOD=$(openssl rsa -in DeveloperID.key -noout -modulus | openssl md5)
echo "cert=$CERT_MOD"
echo "key =$KEY_MOD"
test "$CERT_MOD" = "$KEY_MOD"

# Choose a strong password for the .p12 backup
P12_PASS='choose-a-strong-password'

openssl pkcs12 -export \
  -inkey DeveloperID.key \
  -in DeveloperID.pem \
  -out DeveloperID.p12 \
  -name "Developer ID Application: Your Name (TEAMID)" \
  -passout pass:"$P12_PASS" \
  -certpbe PBE-SHA1-3DES \
  -keypbe PBE-SHA1-3DES \
  -macalg SHA1

# Import identity
security import DeveloperID.p12 \
  -k ~/Library/Keychains/login.keychain-db \
  -P "$P12_PASS" \
  -T /usr/bin/codesign \
  -T /usr/bin/security \
  -T /usr/bin/productsign

# Intermediate (required for a "valid" codesigning identity)
curl -fsSL -o DeveloperIDG2CA.cer \
  "https://www.apple.com/certificateauthority/DeveloperIDG2CA.cer"
security import DeveloperIDG2CA.cer \
  -k ~/Library/Keychains/login.keychain-db

# Allow codesign to use the key (may prompt for keychain password)
security set-key-partition-list -S apple-tool:,apple:,codesign: -s \
  ~/Library/Keychains/login.keychain-db

security find-identity -v -p codesigning
```

Expect a line like:

```text
1) <HASH> "Developer ID Application: Your Name (TEAMID)"
   1 valid identities found
```

Copy that exact string into `APPLE_DEVELOPER_ID` in `make/local.mk`.

### 6.4 Smoke-test codesign

```bash
IDENTITY='Developer ID Application: Your Name (TEAMID)'   # match find-identity / local.mk
TMP=$(mktemp)
echo test > "$TMP"
codesign -s "$IDENTITY" -f "$TMP"
codesign -dv "$TMP" 2>&1 | head -15
rm -f "$TMP"
```

`TeamIdentifier` in the output should match `APPLE_TEAM_ID`.

### 6.5 Backup and restore on another Mac

Keep off-machine (password manager / secure storage):

- `DeveloperID.p12` (password-protected)

**Do not** commit signing files to git.

Import an existing `.p12` on a new machine:

```bash
security import DeveloperID.p12 \
  -k ~/Library/Keychains/login.keychain-db \
  -P 'your-p12-password' \
  -T /usr/bin/codesign -T /usr/bin/security -T /usr/bin/productsign

curl -fsSL -o DeveloperIDG2CA.cer \
  "https://www.apple.com/certificateauthority/DeveloperIDG2CA.cer"
security import DeveloperIDG2CA.cer -k ~/Library/Keychains/login.keychain-db

security find-identity -v -p codesigning
```

---

## 7. Notarization (optional, for shipping)

One-time Keychain profile (interactive):

```bash
make notary-setup
```

Uses `NOTARY_PROFILE` from `make/local.mk` (default `SpeakType`) and an [app-specific password](https://appleid.apple.com/).

Without this profile, `make build` can still produce a **Developer ID–signed** DMG but will skip or warn on notarization.

---

## 8. Release build

```bash
cd /path/to/speaktype

make build
```

Output (typical):

```text
dist/SpeakType_0.1.0_aarch64.dmg
```

Local run without a full signed DMG:

```bash
open apps/widget/src-tauri/target/release/bundle/macos/SpeakType.app
# or: make dev
```

---

## 9. Troubleshooting

| Symptom | Fix |
|---------|-----|
| `cmake: command not found` | `brew install cmake` |
| `rsvg-convert required` | `brew install librsvg` |
| `Pillow required` | `.venv/bin/pip install pillow` (included in `pip install -e ".[all]"` via the `assets` extra) |
| Sidecar cmake fails / empty whisper dir | Clone whisper.cpp (step 4) |
| `Developer ID …: no identity found` | Step 6 — cert + key + G2 intermediate |
| `MAC verification failed during PKCS12 import` | Re-export `.p12` with `-certpbe PBE-SHA1-3DES -keypbe PBE-SHA1-3DES -macalg SHA1` |
| `1 identity imported` but `0 valid identities` | Import [DeveloperIDG2CA.cer](https://www.apple.com/certificateauthority/DeveloperIDG2CA.cer) |
| Wrong cargo target / missing `.app` path | Build in a normal terminal; ensure `CARGO_TARGET_DIR` is unset so output is under `apps/widget/src-tauri/target` |
| `python3.12` missing or broken | Use `/opt/homebrew/bin/python3.12` (step 3) |
| `direnv: command not found` on shell start | `brew install direnv` or guard the hook: `command -v direnv >/dev/null && eval "$(direnv hook zsh)"` |
| No input devices / can’t record | Attach an external mic if the Mac has no built-in input |

---

## 10. Checklist

```text
[ ] git clone speaktype
[ ] brew install python@3.12 portaudio cmake librsvg
[ ] venv + pip install -e ".[all]"
[ ] rustup + cargo install tauri-cli
[ ] clone whisper.cpp into src-tauri/scripts/build/whisper.cpp
[ ] cp make/config.example make/local.mk   # Team ID + Developer ID name
[ ] Developer ID identity in Keychain      # step 6 (or import .p12)
[ ] security find-identity -v -p codesigning
[ ] make build
[ ] (optional) make notary-setup
[ ] keep DeveloperID.p12 backup somewhere safe
```
