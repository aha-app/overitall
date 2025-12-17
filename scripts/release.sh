#!/bin/bash
set -e

# Change to project root directory
cd "$(dirname "$0")/.."

BUMP_TYPE=${1:-patch}

if [[ ! "$BUMP_TYPE" =~ ^(major|minor|patch)$ ]]; then
  echo "Usage: scripts/release.sh [major|minor|patch]"
  echo "Default: patch"
  exit 1
fi

CARGO_VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')
IFS='.' read -r major minor patch <<< "$CARGO_VERSION"

case $BUMP_TYPE in
  major)
    major=$((major + 1))
    minor=0
    patch=0
    ;;
  minor)
    minor=$((minor + 1))
    patch=0
    ;;
  patch)
    patch=$((patch + 1))
    ;;
esac

NEW_VERSION="${major}.${minor}.${patch}"
VERSION="v${NEW_VERSION}"

echo "Bumping version: ${CARGO_VERSION} -> ${NEW_VERSION} (${BUMP_TYPE})"

sed -i '' "s/^version = \".*\"/version = \"${NEW_VERSION}\"/" Cargo.toml

BINARY_NAME="oit-macos-arm64"

echo "Building release binary..."
cargo build --release

echo "Committing version bump..."
git add Cargo.toml Cargo.lock
git commit -m "Bump version to ${NEW_VERSION}"
git push

echo "Creating release archive..."
cd target/release
tar -czf ${BINARY_NAME}.tar.gz oit
cd ../..

echo "Creating git tag ${VERSION}..."
git tag -a ${VERSION} -m "Release ${VERSION}"
git push origin ${VERSION}

echo "Creating GitHub release..."
gh release create ${VERSION} \
  --title "Release ${VERSION}" \
  --generate-notes \
  target/release/${BINARY_NAME}.tar.gz

echo ""
echo "✓ Release ${VERSION} created!"
echo ""
echo "You can install with:"
echo "  curl -L https://github.com/$(gh repo view --json nameWithOwner -q .nameWithOwner)/releases/download/${VERSION}/${BINARY_NAME}.tar.gz | tar xz"
echo "  mv oit /usr/local/bin/"
