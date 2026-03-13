#!/bin/bash
set -euo pipefail

VERSION="${1:?Usage: scripts/release.sh <version>}"

if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "Error: version must be in X.Y.Z format"
    exit 1
fi

if jj root &>/dev/null; then
    VCS=jj
else
    VCS=git
fi

if [ "$VCS" = "jj" ]; then
    if [ -n "$(jj diff)" ]; then
        echo "Error: working copy is not clean"
        exit 1
    fi
else
    if ! git diff --quiet || ! git diff --cached --quiet; then
        echo "Error: working tree is not clean"
        exit 1
    fi
fi

sed -i.bak "s/^version = \".*\"/version = \"$VERSION\"/" Cargo.toml
rm -f Cargo.toml.bak

cargo check --quiet

echo "Running tests..."
cargo test --quiet

if [ "$VCS" = "jj" ]; then
    jj commit -m "Release v$VERSION"
    jj bookmark advance --to @-
    jj git push
    git tag "v$VERSION" "$(jj log -r @- --no-graph -T 'commit_id')"
else
    git add Cargo.toml Cargo.lock
    git commit -m "Release v$VERSION"
    git tag "v$VERSION"
fi
git push origin "v$VERSION"

echo "Tagged v$VERSION — CI will test and publish to crates.io"
