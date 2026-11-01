"""Offline regression tests; never contact a registry or upload a package."""
import base64
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("registry", Path(__file__).parents[1] / "release-registry.py")
r = importlib.util.module_from_spec(spec)
spec.loader.exec_module(r)


def archive(files, prefix="fixture-1.0.0", mtime=0):
    output = io.BytesIO()
    with tarfile.open(fileobj=output, mode="w:gz") as contents:
        for name, data in files.items():
            entry = tarfile.TarInfo(prefix + "/" + name)
            entry.size, entry.mtime = len(data), mtime
            contents.addfile(entry, io.BytesIO(data))
    return output.getvalue()


class RegistryTests(unittest.TestCase):
    def response(self, code, body=b"{}", status=0):
        return subprocess.CompletedProcess([], status, body + b"\n" + str(code).encode(), b"")

    def test_only_404_proves_absence(self):
        for code in (200, 404, 403, 503):
            with self.subTest(code=code), patch.object(r.subprocess, "run", return_value=self.response(code)) as run, \
                    patch.object(r.time, "sleep"):
                if code in (200, 404):
                    self.assertEqual(b"{}" if code == 200 else None, r.fetch("https://fixture"))
                    self.assertEqual(1, run.call_count)
                else:
                    with self.assertRaises(r.RegistryError):
                        r.fetch("https://fixture")
                    self.assertEqual(3 if code == 503 else 1, run.call_count)

    def test_transport_errors_retry_three_times_and_never_become_absence(self):
        with patch.object(r.subprocess, "run", return_value=self.response(0, status=56)) as run, \
                patch.object(r.time, "sleep"):
            with self.assertRaises(r.RegistryError):
                r.fetch("https://fixture")
            self.assertEqual(3, run.call_count)

    def test_request_and_backoff_cannot_outlive_remaining_deadline(self):
        now = [0.0]
        def request(args, **options):
            self.assertLessEqual(options["timeout"], 0.25)
            self.assertEqual("0", args[args.index("--retry") + 1])
            self.assertLessEqual(float(args[args.index("--max-time") + 1]), 0.25)
            now[0] += options["timeout"]
            raise subprocess.TimeoutExpired(args, options["timeout"])
        def sleep(seconds):
            now[0] += seconds
        with patch.object(r.subprocess, "run", side_effect=request) as run, \
                patch.object(r.time, "monotonic", side_effect=lambda: now[0]), \
                patch.object(r.time, "sleep", side_effect=sleep):
            with self.assertRaisesRegex(r.RegistryError, "deadline"):
                r.fetch("https://fixture", deadline=0.25)
        self.assertEqual(1, run.call_count)
        self.assertLessEqual(now[0], 0.25)

    def test_wait_passes_the_same_overall_deadline_to_every_request(self):
        now = [0.0]
        def absent(*args, **kwargs):
            self.assertEqual(6, kwargs["deadline"])
            return None
        with patch.object(r, "metadata", side_effect=absent), \
                patch.object(r.time, "monotonic", side_effect=lambda: now[0]), \
                patch.object(r.time, "sleep", side_effect=lambda seconds: now.__setitem__(0, now[0] + seconds)):
            with self.assertRaisesRegex(r.RegistryError, "deadline"):
                r.wait("npm", "fixture", "1.0.0", seconds=6)
        self.assertEqual(6, now[0])

    def test_registry_metadata_must_match_exact_identity(self):
        for kind, data in (("crate", {"version": {"crate": "other", "num": "1.0.0"}}),
                           ("npm", {"name": "fixture", "version": "2.0.0"}), ("npm", {})):
            with self.subTest(kind=kind, data=data), patch.object(r, "fetch", return_value=json.dumps(data).encode()):
                with self.assertRaisesRegex(r.RegistryError, "Invalid registry metadata"):
                    r.metadata(kind, "fixture", "1.0.0")

    def verify(self, kind, local, remote, **metadata):
        identity = ({"checksum": hashlib.sha256(remote).hexdigest()} if kind == "crate" else
                    {"dist": {"tarball": "https://fixture/archive.tgz", "integrity": "sha512-" +
                              base64.b64encode(hashlib.sha512(remote).digest()).decode()}})
        identity.update(metadata)
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "package.tgz"
            path.write_bytes(local)
            with patch.object(r, "metadata", return_value=identity), patch.object(r, "fetch", return_value=remote):
                r.verify(kind, "fixture", "1.0.0", path)

    def test_crate_equivalent_bytes_ignore_archive_headers_and_vcs_bookkeeping(self):
        self.verify("crate", archive({"src/lib.rs": b"same", ".cargo_vcs_info.json": b"local"}),
                    archive({"src/lib.rs": b"same", ".cargo_vcs_info.json": b"published"}, mtime=100))

    def test_same_crate_version_with_different_sources_or_locks_is_rejected(self):
        for path in ("src/lib.rs", "Cargo.lock", "Cargo.toml"):
            with self.subTest(path=path), self.assertRaisesRegex(r.RegistryError, "Occupied version"):
                self.verify("crate", archive({path: b"local"}), archive({path: b"other source"}))

    def test_npm_bytes_must_match_after_registry_integrity_verification(self):
        self.verify("npm", archive({"protocol.json": b"same"}, "package"),
                    archive({"protocol.json": b"same"}, "package", mtime=100))
        with self.assertRaisesRegex(r.RegistryError, "Occupied version"):
            self.verify("npm", archive({"protocol.json": b"local"}, "package"),
                        archive({"protocol.json": b"other"}, "package"))

    def test_bad_registry_checksums_do_not_establish_equivalence(self):
        contents = archive({"src/lib.rs": b"same"})
        with self.assertRaisesRegex(r.RegistryError, "checksum mismatch"):
            self.verify("crate", contents, contents, checksum="0" * 64)
        contents = archive({"protocol.json": b"same"}, "package")
        with self.assertRaisesRegex(r.RegistryError, "integrity mismatch"):
            self.verify("npm", contents, contents, dist={"tarball": "https://fixture/archive.tgz", "integrity": "sha512-wrong"})

    def test_unsafe_and_duplicate_archive_names_are_rejected(self):
        with self.assertRaises(r.RegistryError):
            r.archive_files(archive({"../escape": b"bad"}), "fixture-1.0.0")


class PublicationScriptTests(unittest.TestCase):
    def test_registry_errors_and_mismatches_block_writes_and_upload_errors_do_not_retry(self):
        script = Path(__file__).parents[1] / "publish-tag.sh"
        for exists, verified, upload, succeeds, writes in (
            (0, 0, 0, True, 0), (0, 2, 0, False, 0), (2, 0, 0, False, 0), (1, 0, 0, False, 0),
            (3, 0, 1, True, 1), (3, 2, 1, False, 1), (3, 0, 0, True, 1),
        ):
            with self.subTest(exists=exists, verified=verified, upload=upload), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                manifest = root / "crates/script-sdk-macros/Cargo.toml"
                manifest.parent.mkdir(parents=True)
                manifest.write_text('version = "1.0.0"\n')
                tools = root / "bin"; tools.mkdir()
                trace = root / "trace"
                cargo = tools / "cargo"
                cargo.write_text('#!' + sys.executable + '\n' + '''import json,os,pathlib,sys
args=sys.argv[1:]
with open(os.environ['FAKE_TRACE'],'a') as log:log.write('cargo '+' '.join(args)+'\\n')
if args[0]=='metadata':print(json.dumps({'target_directory':os.environ['FAKE_TARGET']}))
if args[0]=='package':
 path=pathlib.Path(os.environ['FAKE_TARGET'])/'package/mhome-script-sdk-macros-1.0.0.crate'
 path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(b'prepared local archive')
if args[0]=='publish':
 if '--dry-run' in args:
  path=pathlib.Path(os.environ['FAKE_TARGET'])/'package/tmp-crate/mhome-script-sdk-macros-1.0.0.crate'
  path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(b'prepared local archive')
 else:raise SystemExit(int(os.environ['UPLOAD_RESULT']))
''')
                python = tools / "python3"
                python.write_text('#!' + sys.executable + '\n' + '''import os,sys
args=sys.argv[1:]
if args and args[0]=='-B':args=args[1:]
if args and args[0].endswith('/release-registry.py'):
 with open(os.environ['FAKE_TRACE'],'a') as log:log.write('registry '+' '.join(args[1:])+'\\n')
 raise SystemExit(int(os.environ['REGISTRY_EXISTS' if args[1]=='exists' else 'VERIFY_RESULT']))
os.execv(''' + repr(sys.executable) + ''',[''' + repr(sys.executable) + ''',*sys.argv[1:]])
''')
                node = tools / "node"; node.write_text('#!/bin/sh\nexit 0\n')
                for executable in (cargo, python, node):
                    executable.chmod(0o755)
                env = {**os.environ, "PATH": str(tools) + os.pathsep + os.environ["PATH"],
                       "FAKE_TRACE": str(trace), "FAKE_TARGET": str(root / "target"),
                       "REGISTRY_EXISTS": str(exists), "VERIFY_RESULT": str(verified), "UPLOAD_RESULT": str(upload)}
                result = subprocess.run(["bash", str(script), "mhome-script-sdk-macros-v1.0.0", "--publish"],
                                        cwd=root, env=env, text=True, capture_output=True, timeout=10)
                self.assertEqual(succeeds, result.returncode == 0, result.stdout + result.stderr)
                calls = trace.read_text().splitlines()
                uploads = [call for call in calls if call.startswith('cargo publish ') and '--dry-run' not in call]
                self.assertEqual(writes, len(uploads), calls)
                if exists in (0, 3):
                    self.assertTrue(any(call.startswith('registry verify crate ') for call in calls), calls)


class PreparedCrateArchiveTests(unittest.TestCase):
    def test_publisher_creates_a_real_reference_archive_after_a_temporary_dry_run(self):
        """Exercise real offline Cargo packaging, not a mock-created archive."""
        script = Path(__file__).parents[1] / "publish-tag.sh"
        cargo = shutil.which("cargo")
        self.assertIsNotNone(cargo)
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            package = root / "crates/script-sdk-macros"
            (package / "src").mkdir(parents=True)
            (root / "Cargo.toml").write_text('[workspace]\nmembers = ["crates/script-sdk-macros"]\nresolver = "2"\n')
            (package / "Cargo.toml").write_text(
                '[package]\nname = "mhome-script-sdk-macros"\nversion = "1.0.0"\nedition = "2021"\n'
                'license = "MIT"\ndescription = "Offline reference archive fixture"\n')
            (package / "src/lib.rs").write_text("pub fn fixture() {}\n")
            env = {key: value for key, value in os.environ.items()
                   if key not in ("CARGO_TARGET_DIR", "CARGO_BUILD_TARGET_DIR", "CARGO_BUILD_BUILD_DIR")}
            env.update(CARGO_BUILD_JOBS="1", CARGO_NET_OFFLINE="true", REAL_CARGO=cargo)
            subprocess.run([cargo, "generate-lockfile", "--offline"], cwd=root, env=env,
                           capture_output=True, check=True)
            tools = root / "bin"
            tools.mkdir()
            trace = root / "trace"
            (tools / "cargo").write_text('#!' + sys.executable + '\n' + '''import os,pathlib,sys
args=sys.argv[1:]
with open(os.environ['FAKE_TRACE'],'a') as log:log.write('cargo '+' '.join(args)+'\\n')
if args[0] in ('package','metadata'):
 os.execv(os.environ['REAL_CARGO'],[os.environ['REAL_CARGO'],*args])
if args[0]=='publish':
 if '--dry-run' not in args:raise SystemExit('Test must never upload')
 path=pathlib.Path('target/package/tmp-crate/mhome-script-sdk-macros-1.0.0.crate')
 path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(b'temporary dry-run artifact')
''')
            (tools / "node").write_text("#!/bin/sh\nexit 0\n")
            for path in tools.iterdir():
                path.chmod(0o755)
            env.update(PATH=str(tools) + os.pathsep + env["PATH"], FAKE_TRACE=str(trace))
            result = subprocess.run(["bash", str(script), "mhome-script-sdk-macros-v1.0.0"],
                                    cwd=root, env=env, capture_output=True, text=True, timeout=30)
            self.assertEqual(0, result.returncode, result.stdout + result.stderr)
            reference = root / "target/package/mhome-script-sdk-macros-1.0.0.crate"
            files = r.archive_files(reference.read_bytes(), "mhome-script-sdk-macros-1.0.0", crate=True)
            self.assertEqual(b"pub fn fixture() {}\n", files["src/lib.rs"])
            self.assertIn("Cargo.lock", files)
            commands = trace.read_text().splitlines()
            packaging = next(index for index, line in enumerate(commands) if line.startswith("cargo package "))
            dry_run = next(index for index, line in enumerate(commands) if "publish" in line and "--dry-run" in line)
            self.assertLess(dry_run, packaging)
            self.assertIn("--locked", commands[packaging])
            self.assertIn("--offline", commands[packaging])


if __name__ == "__main__":
    unittest.main()
