#!/usr/bin/env bash
set -euo pipefail

tag="${1:?release tag is required}"
publish="${2:-}"
protocol=""
npm_package=""

case "${tag}" in
  mhome-artifact-api-v*)
    package="mhome-artifact-api"
    manifest="crates/artifact/Cargo.toml"
    version="${tag#mhome-artifact-api-v}"
    protocol="artifact"
    npm_package="@mhome/artifact-protocol"
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
    protocol="core"
    npm_package="@mhome/core-protocol"
    ;;
  mhome-conversation-api-v*)
    package="mhome-conversation-api"
    manifest="crates/conversation-api/Cargo.toml"
    version="${tag#mhome-conversation-api-v}"
    protocol="conversation"
    npm_package="@mhome/conversation-protocol"
    ;;
  mhome-app-facade-api-v*)
    package="mhome-app-facade-api"
    manifest="crates/app-facade-api/Cargo.toml"
    version="${tag#mhome-app-facade-api-v}"
    protocol="appFacade"
    npm_package="@mhome/app-facade-protocol"
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

if [[ ( "${package}" == "mhome-core-api" || "${package}" == "mhome-app-facade-api" ) && ! "${version}" =~ ^1\. ]]; then
  echo "${package} releases must stay on major version 1: ${version}" >&2
  exit 2
fi

manifest_version="$(sed -n 's/^version = "\([^"]*\)"$/\1/p' "${manifest}" | head -n 1)"
if [[ "${manifest_version}" != "${version}" ]]; then
  echo "tag version ${version} does not match ${package} manifest ${manifest_version}" >&2
  exit 2
fi

cargo fmt --all -- --check
cargo clippy -p "${package}" --all-targets --locked -- -D warnings
cargo test -p "${package}" --locked
cargo publish -p "${package}" --locked --dry-run --allow-dirty

staging_root=""
protocol_tarball=""
if [[ -n "${protocol}" ]]; then
  staging_root="$(mktemp -d)"
  trap 'rm -rf "${staging_root}"' EXIT
  node scripts/stage-protocol-package.mjs "${protocol}" "${staging_root}/package"
  npm pack "${staging_root}/package" --pack-destination "${staging_root}" >/dev/null
  protocol_tarball="$(find "${staging_root}" -maxdepth 1 -name '*.tgz' -print -quit)"
  if [[ -z "${protocol_tarball}" ]]; then
    echo "failed to build ${npm_package} ${version}" >&2
    exit 1
  fi
  npm publish "${protocol_tarball}" --dry-run --access public --tag latest >/dev/null
fi

if [[ "${publish}" != "--publish" ]]; then
  if [[ -n "${npm_package}" ]]; then
    echo "verified ${package} and ${npm_package} ${version}"
  else
    echo "verified ${package} ${version}"
  fi
  exit 0
fi

crate_url="https://crates.io/api/v1/crates/${package}/${version}"
is_published() {
  # `cargo info` may resolve the package from this workspace and report success
  # even when the exact version has not reached crates.io. Query the registry
  # API directly so an unpublished local version cannot be mistaken for a
  # completed immutable upload.
  curl \
    --fail \
    --silent \
    --show-error \
    --location \
    --user-agent "mhome-foundation-release/1.0" \
    "${crate_url}" >/dev/null 2>&1
}

if is_published; then
  echo "${package} ${version} is already published"
else
  cargo_publish_failed="false"
  cargo publish -p "${package}" --locked || cargo_publish_failed="true"
  if [[ "${cargo_publish_failed}" == "true" ]]; then
    # Cargo can time out while waiting for the index after a successful immutable upload.
    published_after_retry="false"
    for attempt in 1 2 3 4 5 6; do
      if is_published; then
        published_after_retry="true"
        break
      fi
      sleep $((attempt * 5))
    done
    if [[ "${published_after_retry}" != "true" ]]; then
      echo "publishing ${package} ${version} failed" >&2
      exit 1
    fi
  fi
fi

if [[ -n "${npm_package}" ]]; then
  if npm view "${npm_package}@${version}" version >/dev/null 2>&1; then
    echo "${npm_package} ${version} is already published"
  else
    # Always make the current canonical release the default, including when a
    # previously published erroneous version has a numerically higher semver.
    npm publish "${protocol_tarball}" --access public --provenance --tag latest
  fi
fi
