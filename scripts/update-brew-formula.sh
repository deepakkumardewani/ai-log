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
# ---------------------------------------------------------------------------
set -euo pipefail

VERSION="${VERSION:?must set VERSION, e.g. VERSION=v1.0.0}"
VERSION_CLEAN="${VERSION#v}"
TAP_DIR="$(cd "$(dirname "$0")/../../homebrew-weavr" && pwd)"
FORMULA="$TAP_DIR/Formula/weavr.rb"
REPO="deepakkumardewani/weavr"

echo "==> Computing SHA256 hashes for weavr $VERSION"

declare -A SHA_MAP

for target in aarch64-apple-darwin x86_64-apple-darwin x86_64-unknown-linux-gnu; do
    URL="https://github.com/$REPO/releases/download/$VERSION/weavr-$target.tar.gz"
    echo "  fetching $URL ..."
    SHA=$(curl -sL "$URL" | shasum -a 256 | cut -d' ' -f1)
    if [ -z "$SHA" ]; then
        echo "ERROR: could not download $URL"
        exit 1
    fi
    SHA_MAP[$target]="$SHA"
    echo "    SHA256: $SHA"
done

echo ""
echo "==> Updating $FORMULA"

for target in aarch64-apple-darwin x86_64-apple-darwin x86_64-unknown-linux-gnu; do
    PLACEHOLDER="REPLACE_WITH_SHA256_$target"
    SHA="${SHA_MAP[$target]}"
    sed -i '' "s/$PLACEHOLDER/$SHA/g" "$FORMULA"
done

# Update version
sed -i '' "s/version \".*\"/version \"$VERSION_CLEAN\"/g" "$FORMULA"

echo ""
echo "==> Done. Review, commit, and push the tap repo:"
echo "    cd $TAP_DIR"
echo "    git diff"
echo "    git add Formula/weavr.rb"
echo "    git commit -m 'weavr $VERSION'"
echo "    git push origin main"
