#!/usr/bin/env bash
set -euo pipefail

tag="${1:?release tag is required}"
publish="${2:-}"

case "${tag}" in
  mhome-artifact-api-v*)
    package="mhome-artifact-api"
    manifest="crates/artifact/Cargo.toml"
    version="${tag#mhome-artifact-api-v}"
    ;;
  mhome-machine-identity-v*)
    package="mhome-machine-identity"
    manifest="crates/machine-identity/Cargo.toml"
    version="${tag#mhome-machine-identity-v}"
    ;;
  mhome-core-api-v*)
    package="mhome-core-api"
    manifest="crates/core-api/Cargo.toml"
    version="${tag#mhome-core-api-v}"
    ;;
  mhome-playground-models-v*)
    package="mhome-playground-models"
    manifest="crates/playground-models/Cargo.toml"
    version="${tag#mhome-playground-models-v}"
    ;;
  mhome-runtime-paths-v*)
    package="mhome-runtime-paths"
    manifest="crates/runtime-paths/Cargo.toml"
    version="${tag#mhome-runtime-paths-v}"
    ;;
  *)
    echo "unsupported release tag: ${tag}" >&2
    exit 2
    ;;
esac

if [[ ! "${version}" =~ ^[0-9]+\.[0-9]+\.[0-9]+([+-][0-9A-Za-z.-]+)?$ ]]; then
  echo "tag does not contain a valid semantic version: ${tag}" >&2
  exit 2
fi

manifest_version="$(sed -n 's/^version = "\([^"]*\)"$/\1/p' "${manifest}" | head -n 1)"
if [[ "${manifest_version}" != "${version}" ]]; then
  echo "tag version ${version} does not match ${package} manifest ${manifest_version}" >&2
  exit 2
fi

cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo publish -p "${package}" --locked --dry-run --allow-dirty

if [[ "${publish}" != "--publish" ]]; then
  echo "verified ${package} ${version}"
  exit 0
fi

crate_url="https://crates.io/api/v1/crates/${package}/${version}"
if curl --fail --silent --show-error --location "${crate_url}" >/dev/null 2>&1; then
  echo "${package} ${version} is already published"
  exit 0
fi

if cargo publish -p "${package}" --locked; then
  exit 0
fi

# Cargo can time out while waiting for the index after a successful immutable upload.
for attempt in 1 2 3 4 5 6; do
  if curl --fail --silent --show-error --location "${crate_url}" >/dev/null 2>&1; then
    echo "${package} ${version} is published"
    exit 0
  fi
  sleep $((attempt * 5))
done

echo "publishing ${package} ${version} failed" >&2
exit 1
