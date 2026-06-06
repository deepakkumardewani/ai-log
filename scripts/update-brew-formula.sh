#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# Update the Homebrew formula after publishing a new weavr release.
#
# Usage: VERSION=v1.0.0 ./scripts/update-brew-formula.sh
#
# Prerequisites:
#   1. A release tag has been pushed and GitHub has published the release.
#   2. The homebrew-weavr tap repo is checked out next to this repo:
#      ../homebrew-weavr/
#
# The formula's download URLs are version-interpolated (v#{version}), so this
# script only updates `version` and the per-target sha256 values.
# ---------------------------------------------------------------------------
set -euo pipefail

VERSION="${VERSION:?must set VERSION, e.g. VERSION=v1.0.0}"
VERSION_CLEAN="${VERSION#v}"
TAP_DIR="$(cd "$(dirname "$0")/../../homebrew-weavr" && pwd)"
FORMULA="$TAP_DIR/Formula/weavr.rb"
REPO="deepakkumardewani/weavr"

# Targets that actually ship prebuilt binaries (Intel macOS is intentionally dropped).
TARGETS=(aarch64-apple-darwin x86_64-unknown-linux-gnu)

echo "==> Computing SHA256 hashes for weavr $VERSION"

for target in "${TARGETS[@]}"; do
    URL="https://github.com/$REPO/releases/download/$VERSION/weavr-$target.tar.gz"
    echo "  fetching $URL ..."
    SHA=$(curl -fsSL "$URL" | shasum -a 256 | cut -d' ' -f1)
    if [ -z "$SHA" ]; then
        echo "ERROR: could not download $URL"
        exit 1
    fi
    echo "    SHA256: $SHA"
    # Replace the sha256 value on the line following this target's url line.
    perl -0pi -e "s{(weavr-\Q$target\E\.tar\.gz\"\s*\n\s*sha256 \")[a-f0-9]*}{\${1}$SHA}g" "$FORMULA"
done

# Update version.
perl -pi -e "s/version \".*\"/version \"$VERSION_CLEAN\"/g" "$FORMULA"

echo ""
echo "==> Done. Review, commit, and push the tap repo:"
echo "    cd $TAP_DIR"
echo "    git diff"
echo "    git add Formula/weavr.rb"
echo "    git commit -m 'weavr $VERSION'"
echo "    git push origin main"
