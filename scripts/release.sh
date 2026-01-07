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

echo "Building release binaries for all platforms..."

# macOS ARM64 (native)
echo "  Building macOS ARM64..."
cargo build --release
mkdir -p target/release/dist
cp target/release/oit target/release/dist/
cd target/release/dist && tar -czf oit-macos-arm64.tar.gz oit && rm oit && cd ../../..

# macOS x86_64
echo "  Building macOS x86_64..."
cargo build --release --target x86_64-apple-darwin
cp target/x86_64-apple-darwin/release/oit target/release/dist/
cd target/release/dist && tar -czf oit-macos-x86_64.tar.gz oit && rm oit && cd ../../..

# Linux x86_64
echo "  Building Linux x86_64..."
cargo zigbuild --release --target x86_64-unknown-linux-gnu
cp target/x86_64-unknown-linux-gnu/release/oit target/release/dist/
cd target/release/dist && tar -czf oit-linux-x86_64.tar.gz oit && rm oit && cd ../../..

# Linux ARM64
echo "  Building Linux ARM64..."
cargo zigbuild --release --target aarch64-unknown-linux-gnu
cp target/aarch64-unknown-linux-gnu/release/oit target/release/dist/
cd target/release/dist && tar -czf oit-linux-arm64.tar.gz oit && rm oit && cd ../../..

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

echo "Creating git tag ${VERSION}..."
git tag -a ${VERSION} -m "Release ${VERSION}"
git push origin ${VERSION}

echo "Creating GitHub release..."
gh release create ${VERSION} \
  --title "Release ${VERSION}" \
  --generate-notes \
  target/release/dist/oit-macos-arm64.tar.gz \
  target/release/dist/oit-macos-x86_64.tar.gz \
  target/release/dist/oit-linux-x86_64.tar.gz \
  target/release/dist/oit-linux-arm64.tar.gz \
  vscode-extension/${VSIX_FILE}

REPO=$(gh repo view --json nameWithOwner -q .nameWithOwner)

echo ""
echo "✓ Release ${VERSION} created!"
echo ""
echo "Install oit binary:"
echo "  macOS ARM64:  curl -L https://github.com/${REPO}/releases/download/${VERSION}/oit-macos-arm64.tar.gz | tar xz"
echo "  macOS x86_64: curl -L https://github.com/${REPO}/releases/download/${VERSION}/oit-macos-x86_64.tar.gz | tar xz"
echo "  Linux x86_64: curl -L https://github.com/${REPO}/releases/download/${VERSION}/oit-linux-x86_64.tar.gz | tar xz"
echo "  Linux ARM64:  curl -L https://github.com/${REPO}/releases/download/${VERSION}/oit-linux-arm64.tar.gz | tar xz"
echo ""
echo "  Then: mv oit /usr/local/bin/"
echo ""
echo "Install VS Code extension:"
echo "  Download: https://github.com/${REPO}/releases/download/${VERSION}/${VSIX_FILE}"
echo "  Then: code --install-extension ${VSIX_FILE}"
