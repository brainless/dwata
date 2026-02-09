#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "Usage: ./scripts/release.sh <version>"
  echo "Example: ./scripts/release.sh 0.1.0"
  exit 1
fi

VERSION="$1"
TAG="v$VERSION"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cd "$ROOT"

BRANCH="$(git rev-parse --abbrev-ref HEAD)"
if [[ "$BRANCH" != "main" ]]; then
  echo "Release must be run from main branch (current: $BRANCH)"
  exit 1
fi

if [[ -n "$(git status --porcelain)" ]]; then
  echo "Working tree is dirty. Commit or stash changes before releasing."
  exit 1
fi

if git rev-parse -q --verify "refs/tags/$TAG" >/dev/null; then
  echo "Tag $TAG already exists."
  exit 1
fi

python3 - <<PY
import json
from pathlib import Path

version = "${VERSION}"

cargo_toml = Path("Cargo.toml")
text = cargo_toml.read_text()
text = text.replace('version = "0.1.0"', f'version = "{version}"', 1)
cargo_toml.write_text(text)

package_json = Path("gui/package.json")
data = json.loads(package_json.read_text())
data["version"] = version
package_json.write_text(json.dumps(data, indent=2) + "\n")
PY

git add Cargo.toml gui/package.json
git commit -m "Release $TAG"
git tag "$TAG"
git push origin main
git push origin "$TAG"

gh release create "$TAG" --title "$TAG" --notes "Release $TAG"

echo "Release $TAG created. Workflow will build artifacts for the tag."
