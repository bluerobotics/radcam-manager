#!/bin/bash

# Check if version argument is provided
if [ "$#" -ne 1 ]; then
    echo "Usage: $0 <new_semver_version>"
    exit 1
fi

NEW_VERSION="$1"

# SemVer validation using PCRE regex
if ! echo "$NEW_VERSION" | grep -Pq '^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*))?(?:\+([0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?$'; then
    echo "Error: $NEW_VERSION is not a valid SemVer string"
    echo "Example: 0.1.2 or 1.0.0-alpha+build1"
    exit 2
fi

# Update dockerfile
echo "Updating dockerfile..."
DOCKERFILE="dockerfile"
sed -i "s/\(LABEL version=\"\)[^\"]*\"/\1${NEW_VERSION}\"/" "$DOCKERFILE"

# Update workspace Cargo.toml file:
echo "Updating workspace Cargo.toml file..."
CARGO_TOML_FILE="Cargo.toml"
sed -i '/^\[workspace.package\]/, /^\[/ {
    s/^\(\s*version = "\)[^"]*"/\1'"${NEW_VERSION}"'"/
}' "$CARGO_TOML_FILE"

# Update frontend/package.json
echo "Updating frontend/package.json..."
PACKAGE_JSON_FILE="frontend/package.json"
sed -i "s/\(\"version\": \"\)[^\"]*\"/\1${NEW_VERSION}\"/" "$PACKAGE_JSON_FILE"

echo "All versions updated to $NEW_VERSION"
