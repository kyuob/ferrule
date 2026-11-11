#!/usr/bin/env python3
"""Build and check native CLI archives. Standard library only; Python >=3.12."""
from __future__ import annotations

import argparse
import gzip
import hashlib
import io
import json
import os
from pathlib import Path
import re
import shutil
import stat
import subprocess
import tarfile
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[1]
DOCS = (
    "LICENSE", "README.md", "CHANGELOG.md", "docs/installation.md", "docs/faq.md",
    "docs/releases.md", "docs/modules.md", "docs/ble.md", "docs/security.md",
    "docs/network.md", "docs/recovery.md", "docs/status.md",
)
MAX_BYTES = 128 * 1024 * 1024


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def run(*args: str, cwd: Path = ROOT) -> str:
    return subprocess.check_output(args, cwd=cwd, text=True, timeout=600).strip()


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def host() -> str:
    target = next(line[6:] for line in run("rustc", "-vV").splitlines() if line.startswith("host: "))
    require(bool(re.fullmatch(r"[a-zA-Z0-9_-]+", target)), "Invalid native target")
    return target


def executable(target: str) -> str:
    return "ferrule.exe" if "windows" in target else "ferrule"


def archive_name(version: str, target: str) -> str:
    suffix = ".zip" if "windows" in target else ".tar.gz"
    return f"ferrule-{version}-{target}{suffix}"


def write_archive(path: Path, files: dict[str, bytes], binary: str, epoch: int) -> None:
    """Only caller-supplied regular files; no filesystem directory traversal."""
    if path.name.endswith(".zip"):
        import datetime
        stamp = datetime.datetime.fromtimestamp(max(epoch, 315532800), datetime.timezone.utc)
        with zipfile.ZipFile(path, "w", compression=zipfile.ZIP_DEFLATED) as archive:
            for name, contents in sorted(files.items()):
                entry = zipfile.ZipInfo(name, stamp.timetuple()[:6])
                entry.create_system = 3
                entry.external_attr = (stat.S_IFREG | (0o755 if name == binary else 0o644)) << 16
                entry.compress_type = zipfile.ZIP_DEFLATED
                archive.writestr(entry, contents)
    else:
        with path.open("wb") as raw, gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=epoch) as compressed:
            with tarfile.open(fileobj=compressed, mode="w") as archive:
                for name, contents in sorted(files.items()):
                    entry = tarfile.TarInfo(name)
                    entry.size = len(contents)
                    entry.mode = 0o755 if name == binary else 0o644
                    entry.mtime = epoch
                    archive.addfile(entry, io.BytesIO(contents))


def build(output: Path, require_clean: bool = False) -> Path:
    for name in DOCS:
        require((ROOT / name).is_file() and not (ROOT / name).is_symlink(), f"Missing/unsafe release input: {name}")
    metadata = json.loads(run("cargo", "metadata", "--locked", "--no-deps", "--format-version", "1"))
    package = next(package for package in metadata["packages"] if package["name"] == "ferrule")
    version = package["version"]
    require(bool(re.fullmatch(r"\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?", version)), "Invalid package version")
    target = host()
    dirty = bool(run("git", "status", "--porcelain", "--untracked-files=normal"))
    require(not require_clean or not dirty, "Official release builds require a clean source tree")
    revision = run("git", "rev-parse", "HEAD")
    epoch = int(run("git", "show", "-s", "--format=%ct", "HEAD"))
    subprocess.run(["cargo", "build", "--release", "--locked", "--bin", "ferrule", "--target", target], cwd=ROOT, check=True, timeout=600)
    if require_clean:
        require(run("git", "rev-parse", "HEAD") == revision
                and not run("git", "status", "--porcelain", "--untracked-files=normal"),
                "Source changed during the official build")
    binary = executable(target)
    binary_path = Path(metadata["target_directory"]) / target / "release" / binary
    require(run(str(binary_path), "--version") == f"ferrule {version}", "Built executable version mismatch")
    files = {name: (ROOT / name).read_bytes() for name in DOCS}
    files[binary] = binary_path.read_bytes()
    info = {"schema": 1, "version": version, "target": target, "revision": revision,
            "dirty": dirty, "rustc": run("rustc", "--version"),
            "sha256": {name: digest(data) for name, data in sorted(files.items())}}
    files["BUILD-INFO.json"] = (json.dumps(info, indent=2, sort_keys=True) + "\n").encode()
    output.mkdir(parents=True, exist_ok=True)
    artifact = output / archive_name(version, target)
    require(not artifact.exists() and not artifact.with_name(artifact.name + ".sha256").exists(), "Release artifact already exists; use a new output directory")
    with tempfile.TemporaryDirectory(prefix=".release-", dir=output) as temporary:
        staged = Path(temporary) / artifact.name
        write_archive(staged, files, binary, epoch)
        checksum = digest(staged.read_bytes())
        # Exclusive creation also refuses dangling symlinks and concurrent output writers.
        with artifact.open("xb") as destination, staged.open("rb") as source:
            shutil.copyfileobj(source, destination)
        with artifact.with_name(artifact.name + ".sha256").open("x", encoding="ascii", newline="\n") as checksum_file:
            checksum_file.write(f"{checksum}  {artifact.name}\n")
    print(f"Built {artifact} (source {'preview/dirty' if dirty else 'clean'} {revision[:12]})")
    return artifact


def verified_files(artifact: Path) -> dict[str, bytes]:
    require(artifact.stat().st_size <= MAX_BYTES, "Archive is too large")
    checksum_file = artifact.with_name(artifact.name + ".sha256")
    require(checksum_file.stat().st_size < 512, "Invalid checksum file size")
    checksum = re.fullmatch(r"([0-9a-f]{64})  ([A-Za-z0-9_.-]+)\n?", checksum_file.read_text(encoding="ascii"))
    require(checksum is not None and checksum[2] == artifact.name, "Invalid checksum file/name")
    require(digest(artifact.read_bytes()) == checksum[1], "Archive checksum mismatch")
    allowed = set(DOCS) | {"ferrule", "ferrule.exe", "BUILD-INFO.json"}
    files: dict[str, bytes] = {}
    total = 0

    def accept(name: str, size: int) -> None:
        nonlocal total
        require(name in allowed and name not in files, "Unsafe, duplicate or unexpected archive path")
        require(0 <= size <= MAX_BYTES, "Invalid member size")
        total += size
        require(total <= MAX_BYTES, "Archive expands beyond the size limit")

    if artifact.name.endswith(".zip"):
        with zipfile.ZipFile(artifact) as archive:
            require(len(archive.infolist()) <= len(allowed), "Too many archive members")
            for member in archive.infolist():
                require(not member.is_dir() and stat.S_ISREG(member.external_attr >> 16)
                        and not member.flag_bits & 1, "Only unencrypted regular files are allowed")
                accept(member.filename, member.file_size)
                files[member.filename] = archive.read(member)
    else:
        with tarfile.open(artifact, "r:gz") as archive:
            for member in archive:
                require(member.isfile() and member.sparse is None, "Only ordinary regular files are allowed")
                accept(member.name, member.size)
                source = archive.extractfile(member)
                require(source is not None, "Missing archive member data")
                files[member.name] = source.read()
    info = json.loads(files.get("BUILD-INFO.json", b"{}"))
    require(isinstance(info, dict) and info.get("schema") == 1, "Unsupported build information")
    target, version = info.get("target"), info.get("version")
    require(isinstance(target, str) and re.fullmatch(r"[a-zA-Z0-9_-]+", target) is not None, "Invalid target")
    require(isinstance(version, str) and re.fullmatch(r"\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?", version) is not None, "Invalid version")
    require(archive_name(version, target) == artifact.name, "Archive name disagrees with build information")
    require(type(info.get("dirty")) is bool and isinstance(info.get("revision"), str)
            and re.fullmatch(r"[a-f0-9]{40,64}", info["revision"]) is not None, "Invalid source provenance")
    expected = set(DOCS) | {executable(target)}
    require(set(files) == expected | {"BUILD-INFO.json"}, "Missing or mismatched archive files")
    require(info.get("sha256") == {name: digest(files[name]) for name in sorted(expected)}, "Member checksum mismatch")
    return files


def check(artifact: Path) -> None:
    files = verified_files(artifact)
    info = json.loads(files["BUILD-INFO.json"])
    require(info["target"] == host(), "Installation smoke requires the archive's native OS/CPU")
    work = ROOT / ".work"
    work.mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="release-check-", dir=work) as temporary:
        sandbox = Path(temporary)
        unpacked = sandbox / "unpacked"
        for name, contents in files.items():
            destination = unpacked / name
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes(contents)
        installed = sandbox / "prefix" / "bin" / executable(info["target"])
        installed.parent.mkdir(parents=True)
        shutil.copyfile(unpacked / installed.name, installed)
        installed.chmod(0o755)
        require(run(str(installed), "--version", cwd=sandbox) == f"ferrule {info['version']}", "Installed version mismatch")
        require("recover" in run(str(installed), "--help", cwd=sandbox), "Installed command help is incomplete")
        modules = json.loads(run(str(installed), "modules", "--json", cwd=sandbox))
        expected = {"core", "ble", "data", "security", "network"}
        require({module["id"] for module in modules} == expected, "Installed module catalog mismatch")
        for selection in [[], ["--modules", "ble,data,security,network"]]:
            plan = json.loads(run(str(installed), "new", "release_smoke", "--dry-run", "--directory", str(sandbox), *selection, cwd=sandbox))
            require(set(plan["modules"]) == (expected if selection else {"core"}), "Installed plan mismatch")
            require(not (sandbox / "release_smoke").exists(), "Dry-run wrote a project")
        installed.unlink()
        require(not installed.exists(), "Isolated uninstall failed")
    print(f"PASS: {artifact.name}: checksums, safe extraction, isolated installation/CLI/plans/removal")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    build_parser = commands.add_parser("build")
    build_parser.add_argument("--output", type=Path, default=ROOT / "dist")
    build_parser.add_argument("--require-clean", action="store_true")
    check_parser = commands.add_parser("check")
    check_parser.add_argument("archive", type=Path)
    args = parser.parse_args()
    if args.command == "build":
        build(args.output.resolve(), args.require_clean)
    else:
        check(args.archive.resolve())


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, subprocess.SubprocessError, tarfile.TarError, zipfile.BadZipFile) as error:
        raise SystemExit(f"Release check failed: {error}")
