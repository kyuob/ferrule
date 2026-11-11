"""Archive validation must reject unsafe inputs before any executable is run."""
import gzip
import importlib.util
import json
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest.mock import patch
import zipfile

spec = importlib.util.spec_from_file_location("ferrule_release", Path(__file__).resolve().parents[1] / "scripts/release.py")
release = importlib.util.module_from_spec(spec)
spec.loader.exec_module(release)


class ReleaseValidation(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)

    def artifact(self, target="x86_64-unknown-linux-gnu", mutate=None):
        files = {name: b"public documentation" for name in release.DOCS}
        binary = release.executable(target)
        files[binary] = b"fixture only; never execute"
        info = {"schema": 1, "version": "0.1.0", "target": target,
                "revision": "a" * 40, "dirty": False,
                "sha256": {name: release.digest(data) for name, data in files.items()}}
        files["BUILD-INFO.json"] = json.dumps(info).encode()
        if mutate:
            mutate(files)
        path = self.root / release.archive_name("0.1.0", target)
        release.write_archive(path, files, binary, 1700000000)
        self.checksum(path)
        return path

    def checksum(self, path):
        path.with_name(path.name + ".sha256").write_text(f"{release.digest(path.read_bytes())}  {path.name}\n", encoding="ascii")

    def test_regular_tar_and_zip_round_trip_with_manifest_hashes(self):
        for target in ["x86_64-unknown-linux-gnu", "x86_64-pc-windows-msvc"]:
            with self.subTest(target=target):
                files = release.verified_files(self.artifact(target))
                self.assertIn(release.executable(target), files)
                self.assertEqual(set(files), set(release.DOCS) | {release.executable(target), "BUILD-INFO.json"})

    def test_changed_archive_is_rejected_by_external_checksum(self):
        path = self.artifact()
        with path.open("ab") as stream:
            stream.write(b"changed")
        with self.assertRaisesRegex(ValueError, "checksum mismatch"):
            release.verified_files(path)

    def test_checksum_filename_is_not_an_arbitrary_path(self):
        path = self.artifact()
        path.with_name(path.name + ".sha256").write_text(f"{'a' * 64}  ../escape\n")
        with self.assertRaisesRegex(ValueError, "checksum file/name"):
            release.verified_files(path)

    def test_traversal_and_unlisted_files_are_rejected_for_both_formats(self):
        for target in ["x86_64-unknown-linux-gnu", "x86_64-pc-windows-msvc"]:
            for name in ["../escape", "/absolute", "docs/../../escape", "docs\\escape", "credentials.json"]:
                with self.subTest(target=target, name=name):
                    path = self.artifact(target, lambda files: files.update({name: b"bad"}))
                    with self.assertRaisesRegex(ValueError, "Unsafe|Too many"):
                        release.verified_files(path)

    def test_member_mutation_cannot_keep_old_build_manifest(self):
        path = self.artifact(mutate=lambda files: files.update({"README.md": b"changed"}))
        with self.assertRaisesRegex(ValueError, "Member checksum mismatch"):
            release.verified_files(path)

    def test_missing_document_is_rejected(self):
        path = self.artifact(mutate=lambda files: files.pop("LICENSE"))
        with self.assertRaisesRegex(ValueError, "Missing or mismatched"):
            release.verified_files(path)

    def test_symlink_and_duplicate_tar_members_are_rejected(self):
        for kind in ["symlink", "duplicate"]:
            with self.subTest(kind=kind):
                path = self.root / f"{kind}.tar.gz"
                with tarfile.open(path, "w:gz") as archive:
                    member = tarfile.TarInfo("README.md")
                    if kind == "symlink":
                        member.type, member.linkname = tarfile.SYMTYPE, "../escape"
                    archive.addfile(member)
                    if kind == "duplicate":
                        archive.addfile(member)
                self.checksum(path)
                with self.assertRaisesRegex(ValueError, "regular files|duplicate"):
                    release.verified_files(path)

    def test_declared_expansion_limit_is_checked_before_reading_member(self):
        path = self.root / "oversize.tar.gz"
        member = tarfile.TarInfo("README.md")
        member.size = release.MAX_BYTES + 1
        path.write_bytes(gzip.compress(member.tobuf() + b"\0" * 1024))
        self.checksum(path)
        with self.assertRaisesRegex(ValueError, "member size"):
            release.verified_files(path)

    def test_zip_symlinks_are_rejected_before_extraction(self):
        path = self.root / "link.zip"
        with zipfile.ZipFile(path, "w") as archive:
            member = zipfile.ZipInfo("README.md")
            member.create_system = 3
            member.external_attr = 0o120777 << 16
            archive.writestr(member, "../escape")
        self.checksum(path)
        with self.assertRaisesRegex(ValueError, "regular files"):
            release.verified_files(path)

    def test_asset_name_must_match_its_version_and_target(self):
        original = self.artifact()
        renamed = self.root / "ferrule-9.9.9-x86_64-unknown-linux-gnu.tar.gz"
        original.rename(renamed)
        self.checksum(renamed)
        with self.assertRaisesRegex(ValueError, "name disagrees"):
            release.verified_files(renamed)

    def test_foreign_binary_is_never_executed(self):
        path = self.artifact()
        with patch.object(release, "host", return_value="aarch64-apple-darwin"), patch.object(release, "run") as command:
            with self.assertRaisesRegex(ValueError, "native OS/CPU"):
                release.check(path)
            command.assert_not_called()


if __name__ == "__main__":
    unittest.main()
