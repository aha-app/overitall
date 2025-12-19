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

# Update VS Code extension version to match
echo "Updating VS Code extension version..."
cd vscode-extension
npm version ${NEW_VERSION} --no-git-tag-version
cd ..

BINARY_NAME="oit-macos-arm64"

echo "Building release binary..."
cargo build --release

echo "Building VS Code extension..."
cd vscode-extension
npm run compile
npm run package
VSIX_FILE="vscode-overitall-${NEW_VERSION}.vsix"
cd ..

echo "Committing version bump..."
git add Cargo.toml Cargo.lock vscode-extension/package.json vscode-extension/package-lock.json
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
  target/release/${BINARY_NAME}.tar.gz \
  vscode-extension/${VSIX_FILE}

REPO=$(gh repo view --json nameWithOwner -q .nameWithOwner)

echo ""
echo "✓ Release ${VERSION} created!"
echo ""
echo "Install oit binary:"
echo "  curl -L https://github.com/${REPO}/releases/download/${VERSION}/${BINARY_NAME}.tar.gz | tar xz"
echo "  mv oit /usr/local/bin/"
echo ""
echo "Install VS Code extension:"
echo "  Download: https://github.com/${REPO}/releases/download/${VERSION}/${VSIX_FILE}"
echo "  Then: code --install-extension ${VSIX_FILE}"
