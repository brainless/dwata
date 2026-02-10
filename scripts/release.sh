#!/usr/bin/env bash
set -euo pipefail

API_VERSION_OVERRIDE=""
GUI_VERSION_OVERRIDE=""
VERSION_OVERRIDE=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --api-version)
      API_VERSION_OVERRIDE="$2"
      shift 2
      ;;
    --gui-version)
      GUI_VERSION_OVERRIDE="$2"
      shift 2
      ;;
    --version)
      VERSION_OVERRIDE="$2"
      shift 2
      ;;
    -h|--help)
      echo "Usage: ./scripts/release.sh [--version X.Y.Z] [--api-version X.Y.Z] [--gui-version X.Y.Z]"
      echo "If no versions are provided, the default is a minor bump for both."
      exit 0
      ;;
    *)
      echo "Unknown argument: $1"
      exit 1
      ;;
  esac
done

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cd "$ROOT"

BRANCH="$(git rev-parse --abbrev-ref HEAD)"
if [[ "$BRANCH" != "main" ]]; then
  echo "Release must be run from main branch (current: $BRANCH)"
  exit 1
fi

if [[ -n "$(git status --porcelain --untracked-files=no)" ]]; then
  echo "Working tree has tracked changes. Commit or stash before releasing."
  exit 1
fi

FALLBACK_VERSION=""
if TAG="$(git tag --list "v*" --sort=-v:refname | head -n1)"; then
  if [[ -n "$TAG" ]]; then
    FALLBACK_VERSION="${TAG#v}"
  fi
fi

read -r CURRENT_API_VERSION < <(FALLBACK_VERSION="$FALLBACK_VERSION" python3 - <<'PY'
import re
from pathlib import Path
import os
text = Path("Cargo.toml").read_text()
m = re.search(r'^\[workspace\.package\][\s\S]*?^version\s*=\s*"([^"]+)"', text, re.MULTILINE)
if not m:
  raise SystemExit("Failed to read workspace version from Cargo.toml")
version = m.group(1)
if re.match(r'^\d+\.\d+\.\d+$', version):
  print(version)
  raise SystemExit(0)
fallback = os.environ.get("FALLBACK_VERSION", "")
if fallback:
  print(fallback)
  raise SystemExit(0)
raise SystemExit("Failed to resolve current version (Cargo.toml uses template and no tags found)")
PY
)

read -r CURRENT_GUI_VERSION < <(FALLBACK_VERSION="$CURRENT_API_VERSION" python3 - <<'PY'
import json
from pathlib import Path
import os
data = json.loads(Path("gui/package.json").read_text())
version = data.get("version", "0.0.0")
if version.count(".") == 2 and all(part.isdigit() for part in version.split(".")):
  print(version)
  raise SystemExit(0)
fallback = os.environ.get("FALLBACK_VERSION", "")
if fallback:
  print(fallback)
  raise SystemExit(0)
raise SystemExit("Failed to resolve current GUI version (package.json uses template and no fallback found)")
PY
)

read -r DEFAULT_VERSION < <(CURRENT_VERSION="$CURRENT_API_VERSION" python3 - <<'PY'
import os
parts = os.environ["CURRENT_VERSION"].split(".")
if len(parts) < 3:
  raise SystemExit("Version must be in X.Y.Z form")
major, minor, patch = parts[0], parts[1], parts[2]
minor = str(int(minor) + 1)
patch = "0"
print(".".join([major, minor, patch]))
PY
)

if [[ -n "$VERSION_OVERRIDE" ]]; then
  API_VERSION="$VERSION_OVERRIDE"
  GUI_VERSION="$VERSION_OVERRIDE"
else
  API_VERSION="${API_VERSION_OVERRIDE:-$DEFAULT_VERSION}"
  GUI_VERSION="${GUI_VERSION_OVERRIDE:-$DEFAULT_VERSION}"
fi

echo "Current versions:"
echo "  dwata-api/workspace: $CURRENT_API_VERSION"
echo "  gui: $CURRENT_GUI_VERSION"
echo
echo "Next versions:"
echo "  dwata-api/workspace: $API_VERSION"
echo "  gui: $GUI_VERSION"
echo
read -r -p "Proceed with release? [y/N] " CONFIRM
if [[ ! "$CONFIRM" =~ ^[Yy]$ ]]; then
  echo "Release cancelled."
  exit 1
fi

TAG="v$API_VERSION"
if git rev-parse -q --verify "refs/tags/$TAG" >/dev/null; then
  echo "Tag $TAG already exists."
  exit 1
fi

API_VERSION="$API_VERSION" GUI_VERSION="$GUI_VERSION" python3 - <<'PY'
import json
import re
from pathlib import Path
import os

api_version = os.environ["API_VERSION"]
gui_version = os.environ["GUI_VERSION"]

cargo_toml = Path("Cargo.toml")
text = cargo_toml.read_text()

pattern = re.compile(r'(\[workspace\.package\][\s\S]*?^version\s*=\s*")[^"]+(")', re.MULTILINE)
new_text, count = pattern.subn(rf'\1{api_version}\2', text, count=1)
if count != 1:
  raise SystemExit("Failed to update Cargo.toml workspace version")
cargo_toml.write_text(new_text)

package_json = Path("gui/package.json")
data = json.loads(package_json.read_text())
data["version"] = gui_version
package_json.write_text(json.dumps(data, indent=2) + "\n")
PY

git add Cargo.toml gui/package.json
git commit -m "Release $TAG"
git tag "$TAG"
git push origin main
git push origin "$TAG"

gh release create "$TAG" --title "$TAG" --notes "Release $TAG"

echo "Release $TAG created. Workflow will build artifacts for the tag."
