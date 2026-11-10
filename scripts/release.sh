#!/bin/sh
# Release the Fragment Rust crates.
#
#   ./scripts/release.sh            # release the version in the manifests
#   ./scripts/release.sh 0.3.0      # release a specific version
#   ./scripts/release.sh --dry-run  # show what would happen
#
# Bumps the version in every crate, commits it, tags the commit, and pushes
# both. The tag starts the release workflow, which builds the `frs` CLI and
# publishes it as a GitHub release.
#
# The crates are published to crates.io as well, unless --no-publish is
# given. The token comes from the `.env` file in the project root, or from
# CARGO_REGISTRY_TOKEN in the environment.
set -eu

cd "$(dirname "$0")/.."

DRY_RUN=0
PUBLISH=1
VERSION=""
for arg in "$@"; do
    case "$arg" in
        --dry-run) DRY_RUN=1 ;;
        --no-publish) PUBLISH=0 ;;
        -*) echo "Unknown option: $arg" >&2; exit 2 ;;
        *) VERSION=$arg ;;
    esac
done

if [ -z "$VERSION" ]; then
    VERSION=$(sed -n 's/^version = "\(.*\)"$/\1/p' fragment-rs/Cargo.toml | head -1)
    [ -n "$VERSION" ] || { echo "Could not read the version from fragment-rs/Cargo.toml" >&2; exit 1; }
fi

case "$VERSION" in
    [0-9]*.[0-9]*.[0-9]*) ;;
    *) echo "Version must look like 1.2.3, got '$VERSION'" >&2; exit 1 ;;
esac

TAG="v$VERSION"

if [ -n "$(git status --porcelain)" ]; then
    echo "The working tree has changes. Commit them first." >&2
    exit 1
fi

if git rev-parse "$TAG" >/dev/null 2>&1; then
    echo "Tag $TAG already exists." >&2
    exit 1
fi

echo "Releasing $TAG"

if [ "$DRY_RUN" -eq 1 ]; then
    echo "Would update the version in:"
    echo "  fragment-rs/Cargo.toml"
    echo "  fragment-macros/Cargo.toml"
    echo "  frs-cli/Cargo.toml"
    echo "Would commit, tag $TAG, and push."
    if [ "$PUBLISH" -eq 1 ]; then
        echo "Would publish the crates to crates.io."
    fi
    exit 0
fi

# The version of every crate, and of the path dependency between them.
sed -i '' \
    -e "s/^version = \".*\"/version = \"$VERSION\"/" \
    -e "s|\(fragment-macros = { path = \"../fragment-macros\", version = \)\"[^\"]*\"|\1\"$VERSION\"|" \
    fragment-rs/Cargo.toml
sed -i '' "s/^version = \".*\"/version = \"$VERSION\"/" fragment-macros/Cargo.toml
sed -i '' "s/^version = \".*\"/version = \"$VERSION\"/" frs-cli/Cargo.toml

# The lockfile records the version of the workspace members.
cargo check --workspace >/dev/null 2>&1 || true
cargo build --locked --release --package frs-cli >/dev/null

git add fragment-rs/Cargo.toml fragment-macros/Cargo.toml frs-cli/Cargo.toml Cargo.lock
git commit -m "release $TAG"

git tag -a "$TAG" -m "Fragment Rust $TAG"
git push origin HEAD
git push origin "$TAG"

echo
echo "Released $TAG"
echo "The release workflow is running:"
echo "  https://github.com/Jesse-Dan/fragment-rs/actions"
echo "Watch it here: gh run watch --repo Jesse-Dan/fragment-rs"

# Publish the crates to crates.io. The tag has already been pushed, so the
# GitHub release is not held up by this.
if [ "$PUBLISH" -eq 1 ]; then
    echo
    if [ -f .env ] || [ -n "${CARGO_REGISTRY_TOKEN:-}" ]; then
        ./scripts/publish.sh
    else
        echo "No crates.io token found. Skipping the crates.io publish."
        echo "Put CARGO_REGISTRY_TOKEN in .env, or run ./scripts/publish.sh."
    fi
fi
