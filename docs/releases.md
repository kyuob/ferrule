# Release preparation

The CLI is version **0.1.0**; core and optional modules have separate contract versions.
The [v0.1.0 public pre-release](https://github.com/kyuob/ferrule/releases/tag/v0.1.0)
was published on 2026-11-11 from commit `31811553ec3b7e6545fdc423eba5cbd36f44450d`.
It includes the six tested archives/checksums from [CI run 42951802964](https://github.com/kyuob/ferrule/actions/runs/42951802964).
The annotated tag, public state, release notes and unauthenticated download hashes were verified.
Source and native archive installation are available. No crates.io name is reserved and
Cargo publication is disabled. The workflow prepares native GitHub Release assets;
it does not publish them, push tags or create a GitHub Release automatically.

## Local preview

Python 3.12 and the native stable Rust/C toolchain are needed for packaging:

```sh
sh scripts/release-smoke.sh
# Or, with cargo/rustc on PATH:
python3 scripts/release.py build --output dist
python3 scripts/release.py check dist/ferrule-0.1.0-YOUR-HOST-TRIPLE.tar.gz
```

Use `.zip` on Windows. A build refuses an existing archive/checksum pair; choose a new
output directory for another preview. The shell smoke uses a fresh `.work/release-*`
directory and checks an installation under a disposable prefix, never the global PATH.
Local uncommitted changes produce `dirty: true` build information and are previews,
not official releases. `--require-clean` checks the source tree before and after building.

Each package contains only the native executable, the license, the approved Markdown
guides/changelog and `BUILD-INFO.json`. It does not contain toolchains, caches, project
manuscripts, credentials or recovery journals. Version, target, source revision, compiler
and per-file SHA-256 hashes are recorded. A sibling `.sha256` file covers the archive.
Checksums detect changes; they are not a signature or an independent trust source.
Artifacts are currently unsigned/not notarized and are not reproducible-build claims.

## Verification

```sh
python3 -m unittest discover -s tests -p test_release.py
```

The archive checker verifies checksums, a strict file allowlist, ordinary regular-file
entries, duplicate/path/symlink rejection, expansion bounds and the build manifest
before extraction or execution. A native-target check prevents executing a foreign
binary. It then copies the binary to an isolated prefix, checks version/help/module
catalog, runs core and four-module offline plans without writes, removes the installed
copy and removes its temporary prefix.

CI runs those checks after the existing Rust tests on Ubuntu, macOS and Windows native
runners. It uploads `ferrule-OS-ARCH` workflow artifacts with a fourteen-day retention.
The actual target triple is recorded in the asset; this is not a matrix of every OS/CPU.
These artifacts are build outputs, not a public release. Remote runs are unverified until
the workflow has actually executed successfully at the intended source revision.

## Release checklist

1. Review changelog, user-reported device verification and known limitations in
   [status](status.md). Keep the source commit clean and record the exact revision.
2. Commit/push only with user authorization and inspect all three CI jobs. Do not
   call the Linux/Windows packages verified based on local macOS success.
3. Obtain fresh explicit approval for creating/pushing a version tag and publishing
   a GitHub Release. A previous push approval does not authorize release publication.
4. Download the three successful CI artifacts for the exact revision. Preserve each
   native archive and matching checksum; inspect their BUILD-INFO records and require
   `dirty: false`. Attach those tested files rather than silently rebuilding new bytes.
5. Prepare a draft GitHub Release with tag/version matching Cargo.toml, the changelog,
   supported native targets and limits. Review its assets before publishing under the
   granted approval. Do not attach `.work` folders or local preview packages.
6. After publication, verify actual download URLs and checksums, then change the
   installation guide from future-release instructions to those concrete assets.

Module merging improvements, automatic module upgrades/removal and optional sensor
protocol examples are separate development work; they are not silently included in 0.1.0.
