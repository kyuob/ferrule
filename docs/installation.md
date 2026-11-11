# Installation

Ferrule is a CLI project builder, not a Flutter app. No Ferrule account, API key or
login is required. Generated projects are local directories you own.

## Install from source now

Use a current stable Rust toolchain and a C linker/toolchain for your OS. Clone the
repository and install its locked dependencies:

```sh
git clone https://github.com/kyuob/ferrule.git
cd ferrule
cargo install --path . --locked
ferrule --version
ferrule modules
ferrule new my_app --dry-run
```

Cargo normally installs the executable under `~/.cargo/bin` on macOS/Linux or
`%USERPROFILE%\.cargo\bin` on Windows. If the command cannot be found, open a new
terminal and check that directory is on PATH. A custom CARGO_HOME/install root changes
the location. You can use `cargo run --locked -- --version` from the checkout as a check.

To change your installed CLI later, review the new revision and repeat the installation
command with Cargo's `--force` option. This updates the CLI executable, not generated apps.
Use `cargo uninstall ferrule` to remove a Cargo-installed CLI. Keep generated apps separately.

## Install the v0.1.0 pre-release executable

The [public v0.1.0 pre-release](https://github.com/kyuob/ferrule/releases/tag/v0.1.0)
contains these tested native archives and matching checksums:

| Host | Archive | SHA-256 file |
| --- | --- | --- |
| macOS Apple Silicon | [tar.gz](https://github.com/kyuob/ferrule/releases/download/v0.1.0/ferrule-0.1.0-aarch64-apple-darwin.tar.gz) | [checksum](https://github.com/kyuob/ferrule/releases/download/v0.1.0/ferrule-0.1.0-aarch64-apple-darwin.tar.gz.sha256) |
| Linux x86-64 GNU | [tar.gz](https://github.com/kyuob/ferrule/releases/download/v0.1.0/ferrule-0.1.0-x86_64-unknown-linux-gnu.tar.gz) | [checksum](https://github.com/kyuob/ferrule/releases/download/v0.1.0/ferrule-0.1.0-x86_64-unknown-linux-gnu.tar.gz.sha256) |
| Windows x86-64 MSVC | [zip](https://github.com/kyuob/ferrule/releases/download/v0.1.0/ferrule-0.1.0-x86_64-pc-windows-msvc.zip) | [checksum](https://github.com/kyuob/ferrule/releases/download/v0.1.0/ferrule-0.1.0-x86_64-pc-windows-msvc.zip.sha256) |

All six public downloads were fetched without credentials and compared with the approved
CI outputs. These are unsigned early-preview packages, not universally compatible binaries.

Choose the exact native OS/CPU asset from the approved GitHub Release and download its
matching `.sha256` file. Packages use the Rust host triple in the filename:
`ferrule-VERSION-TARGET.tar.gz` for macOS/Linux or `.zip` for Windows. The current CI
builds each runner's native host, not every architecture. Use source installation when
there is no matching asset. `BUILD-INFO.json` records the version, target, source revision,
dirty-preview flag and individual file hashes.

For macOS/Linux, run in the download directory, replacing the example filename:

```sh
ASSET=ferrule-0.1.0-aarch64-apple-darwin.tar.gz
shasum -a 256 -c "$ASSET.sha256"   # macOS
# On Linux use: sha256sum -c "$ASSET.sha256"
mkdir ferrule-package
tar -xzf "$ASSET" -C ferrule-package
./ferrule-package/ferrule --version
mkdir -p "$HOME/.local/bin"
# Back up an existing ferrule executable before replacing it.
cp ferrule-package/ferrule "$HOME/.local/bin/ferrule"
chmod 755 "$HOME/.local/bin/ferrule"
export PATH="$HOME/.local/bin:$PATH"
ferrule modules
```

Add that PATH entry to your own shell configuration if you want it in new terminals.
To uninstall this manual copy, remove only `$HOME/.local/bin/ferrule`. Do not delete
your generated project folders. Do not mix Cargo and manual installation paths without
checking `command -v ferrule` first.

On Windows, use PowerShell and replace the asset name if necessary:

```powershell
$asset = '.\ferrule-0.1.0-x86_64-pc-windows-msvc.zip'
$expected = ((Get-Content "$asset.sha256" -Raw) -split '\s+')[0]
if ((Get-FileHash $asset -Algorithm SHA256).Hash.ToLowerInvariant() -ne $expected) {
    throw 'Checksum mismatch; do not extract or run this archive.'
}
Expand-Archive $asset -DestinationPath .\ferrule-package
.\ferrule-package\ferrule.exe --version
$bin = Join-Path $env:LOCALAPPDATA 'Ferrule\bin'
New-Item -ItemType Directory -Force $bin | Out-Null
# Back up an existing executable before replacing it.
Copy-Item .\ferrule-package\ferrule.exe (Join-Path $bin 'ferrule.exe')
$env:PATH = "$bin;$env:PATH"
ferrule modules
```

For new terminals, add that exact bin directory to your user PATH through Windows
Environment Variables. Use `Get-Command ferrule` to locate the active copy. Remove only
the manually installed `ferrule.exe` to uninstall; generated apps remain untouched.

## Install the app-generation prerequisites

A prebuilt CLI does **not** replace Rust/Flutter/Xcode/Android build tools. To generate
and build apps, install Flutter and Rust, then the pinned bridge generator:

```sh
cargo install flutter_rust_bridge_codegen --version 2.13.0 --locked
cargo install cargo-expand --locked
flutter doctor -v
ferrule doctor
```

The tested baseline is Flutter 3.41.2 / Dart 3.11.0 / FRB 2.13.0. Use Xcode and its
command-line tools on macOS for iOS builds; use the Android SDK/NDK and required licenses
for Android. The doctor checks tools; it does not install SDKs or accept licenses.
Generation/sync can download dependencies; offline dry-run/catalog commands do not.

## Create the first app

```sh
ferrule new my_app --dry-run
ferrule new my_app
ferrule sync --project my_app
cd my_app
flutter run
```

For optional capabilities select `--modules ble,data,security,network`, or add a module
later with `ferrule add MODULE --project my_app` and sync again. Start with only what the
app needs. See [module contracts](modules.md), [FAQ](faq.md) and [recovery](recovery.md).

Official prerequisites: [Rust installation](https://www.rust-lang.org/tools/install),
[Flutter installation](https://docs.flutter.dev/install),
[flutter_rust_bridge](https://cjycode.com/flutter_rust_bridge/).
