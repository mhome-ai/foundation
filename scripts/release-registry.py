#!/usr/bin/env python3
"""Bounded registry reads and content checks; never upload or retry a write."""
import argparse
import base64
import hashlib
import io
import json
from pathlib import Path, PurePosixPath
import re
import subprocess
import sys
import tarfile
import time
from urllib.parse import quote


class RegistryError(Exception):
    pass


# npm publish-time scanning commonly takes five minutes and can exceed fifteen.
# This is a deadline, not a fixed delay; stop waiting as soon as the data appears.
NPM_READINESS_SECONDS = 20 * 60


def fetch(url, *, deadline=None, request_seconds=15):
    """404 alone means absent. Bound connect, body, retries and the subprocess."""
    for attempt in range(3):
        remaining = request_seconds if deadline is None else min(request_seconds, deadline - time.monotonic())
        if remaining <= 0:
            raise RegistryError(f"Registry deadline exceeded: {url}")
        detail = "request failed"
        try:
            result = subprocess.run(
                ["curl", "--silent", "--show-error", "--location", "--retry", "0",
                 "--connect-timeout", str(min(10, remaining)), "--max-time", str(remaining),
                 "--user-agent", "mhome-foundation-release/1.0", "--write-out", "\n%{http_code}", url],
                capture_output=True, timeout=remaining)
            body, separator, status = result.stdout.rpartition(b"\n")
            if result.returncode == 0 and separator and status == b"200":
                return body
            if result.returncode == 0 and separator and status == b"404":
                return None
            detail = (result.stderr.decode(errors="replace").strip() or f"HTTP {status.decode(errors='replace')}")
            if result.returncode == 0 and status not in (b"408", b"429") and not status.startswith(b"5"):
                raise RegistryError(f"Cannot verify registry {url}: {detail}")
        except (OSError, subprocess.TimeoutExpired) as error:
            detail = str(error)
        if attempt == 2:
            raise RegistryError(f"Cannot verify registry {url}: {detail}")
        delay = 2 * (attempt + 1)
        if deadline is not None:
            delay = min(delay, max(0, deadline - time.monotonic()))
        print(f"Registry read failed; retry {attempt + 2}/3: {url}: {detail}", file=sys.stderr)
        time.sleep(delay)


def metadata(kind, name, version, *, deadline=None):
    if not re.fullmatch(r"\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?", version):
        raise RegistryError("Expected an exact package version")
    if kind == "crate":
        if not re.fullmatch(r"[A-Za-z][A-Za-z0-9_-]*", name):
            raise RegistryError("Invalid crate name")
        url = f"https://crates.io/api/v1/crates/{name}/{version}"
    else:
        if not re.fullmatch(r"(?:@[a-z0-9-]+/)?[a-z0-9-]+", name):
            raise RegistryError("Invalid npm package name")
        url = f"https://registry.npmjs.org/{quote(name, safe='@/')}/{version}"
    contents = fetch(url, deadline=deadline)
    if contents is None:
        return None
    try:
        data = json.loads(contents)
        identity = data["version"] if kind == "crate" else data
        actual = (identity["crate"], identity["num"]) if kind == "crate" else (identity["name"], identity["version"])
        if actual != (name, version):
            raise ValueError("response belongs to a different package/version")
        return identity
    except (ValueError, KeyError, TypeError) as error:
        raise RegistryError(f"Invalid registry metadata for {name} {version}: {error}") from error


def wait(kind, name, version, *, seconds=None):
    if seconds is None:
        seconds = NPM_READINESS_SECONDS if kind == "npm" else 600
    if seconds <= 0:
        raise RegistryError("Registry wait timeout must be positive")
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        value = metadata(kind, name, version, deadline=deadline)
        if value is not None:
            return value
        time.sleep(min(5, max(0, deadline - time.monotonic())))
    raise RegistryError(f"{name} {version} did not appear before the registry deadline")


def archive_files(contents, prefix, *, crate=False):
    files = {}
    try:
        with tarfile.open(fileobj=io.BytesIO(contents), mode="r:gz") as archive:
            for member in archive:
                if member.isdir():
                    continue
                if not member.isfile() or not member.name.startswith(prefix + "/"):
                    raise RegistryError(f"Unexpected package archive entry: {member.name}")
                relative = member.name[len(prefix) + 1:]
                path = PurePosixPath(relative)
                if not relative or path.is_absolute() or ".." in path.parts or relative in files:
                    raise RegistryError(f"Unsafe or duplicate package archive entry: {member.name}")
                # A rerun can package the same files with different checkout VCS
                # bookkeeping. All other files, including Cargo.lock, must match.
                if crate and relative == ".cargo_vcs_info.json":
                    continue
                files[relative] = archive.extractfile(member).read()
    except (tarfile.TarError, OSError, EOFError) as error:
        raise RegistryError(f"Invalid package archive: {error}") from error
    if not files:
        raise RegistryError("Package archive contains no regular files")
    return files


def wait_for_archive(url, *, deadline):
    """Registry metadata can become visible before its immutable archive."""
    announced = False
    while time.monotonic() < deadline:
        contents = fetch(url, deadline=deadline, request_seconds=60)
        if contents is not None:
            return contents
        if not announced:
            print(f"Published package archive not yet available; waiting: {url}", file=sys.stderr)
            announced = True
        time.sleep(min(5, max(0, deadline - time.monotonic())))
    raise RegistryError(f"Published package archive did not appear before the registry deadline: {url}")


def verify(kind, name, version, local):
    deadline = time.monotonic() + (NPM_READINESS_SECONDS if kind == "npm" else 600)
    info = metadata(kind, name, version, deadline=deadline)
    if info is None:
        raise RegistryError(f"Cannot verify missing published package {name} {version}")
    if kind == "crate":
        checksum = info.get("checksum", "")
        if not re.fullmatch(r"[a-f0-9]{64}", checksum):
            raise RegistryError("Missing published crate checksum")
        url = f"https://static.crates.io/crates/{name}/{name}-{version}.crate"
        remote = wait_for_archive(url, deadline=deadline)
        if hashlib.sha256(remote).hexdigest() != checksum:
            raise RegistryError("Published crate checksum mismatch")
        prefix = name + "-" + version
    else:
        dist = info.get("dist") or {}
        integrity, url = dist.get("integrity", ""), dist.get("tarball", "")
        if not integrity.startswith("sha512-") or not url.startswith("https://"):
            raise RegistryError("Missing npm SHA-512 integrity or HTTPS tarball")
        remote = wait_for_archive(url, deadline=deadline)
        if integrity != "sha512-" + base64.b64encode(hashlib.sha512(remote).digest()).decode():
            raise RegistryError("Published npm archive integrity mismatch")
        prefix = "package"
    actual = archive_files(remote, prefix, crate=kind == "crate")
    expected = archive_files(Path(local).read_bytes(), prefix, crate=kind == "crate")
    if actual != expected:
        changed = sorted(path for path in actual.keys() | expected.keys() if actual.get(path) != expected.get(path))
        raise RegistryError(f"Occupied version {name} {version} differs from this source: {', '.join(changed[:10])}")
    print(f"Verified registry package contents: {name} {version}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("exists", "wait", "verify"))
    parser.add_argument("kind", choices=("crate", "npm"))
    parser.add_argument("name")
    parser.add_argument("version")
    parser.add_argument("archive", nargs="?", type=Path)
    args = parser.parse_args()
    try:
        if args.action == "exists":
            # Reserve 3 for proven absence. An unexpected Python/process exit 1
            # must never look like an HTTP 404 to a conditional shell caller.
            return 0 if metadata(args.kind, args.name, args.version) is not None else 3
        if args.action == "wait":
            wait(args.kind, args.name, args.version)
        else:
            if args.archive is None:
                raise RegistryError("Package verification needs the freshly prepared local archive")
            verify(args.kind, args.name, args.version, args.archive)
        return 0
    except (RegistryError, OSError) as error:
        print(str(error), file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
