#!/bin/sh
# Publish the Fragment Rust crates to crates.io.
#
#   ./scripts/publish.sh            # publish every crate
#   ./scripts/publish.sh --dry-run  # check what would be published
#
# The crates are published in dependency order: the macros first, then the
# framework that uses them, then the CLI.
#
# crates.io needs a moment to index a crate before another crate can depend
# on it, so each step waits for the previous one to become visible.
set -eu

cd "$(dirname "$0")/.."

DRY_RUN=0
for arg in "$@"; do
    case "$arg" in
        --dry-run) DRY_RUN=1 ;;
        -*) echo "Unknown option: $arg" >&2; exit 2 ;;
    esac
done

CRATES="fragment-macros fragment-rs frs-cli"

# The version every crate is published under.
VERSION=$(sed -n 's/^version = "\(.*\)"$/\1/p' fragment-rs/Cargo.toml | head -1)

# Whether a crate is already on crates.io at this version.
already_published() {
    crate=$1
    body=$(curl -sS --max-time 20 "https://crates.io/api/v1/crates/$crate" \
        -H "User-Agent: frs-cli (https://github.com/Jesse-Dan/fragment-rs)" 2>/dev/null || true)
    echo "$body" | grep -q "\"num\":\"$VERSION\"" && return 0
    return 1
}

# The token comes from the environment, or from the `.env` file in the
# project root. The `.env` file is ignored by git.
if [ -f .env ]; then
    if git ls-files --error-unmatch .env >/dev/null 2>&1; then
        echo "The .env file is tracked by git. Remove it from the index:" >&2
        echo "  git rm --cached .env" >&2
        exit 1
    fi
    set -a
    # shellcheck disable=SC1091
    . ./.env
    set +a
fi

if [ "$DRY_RUN" -eq 0 ]; then
    if ! cargo publish --help >/dev/null 2>&1; then
        echo "cargo publish is not available. Install the crates.io tooling first." >&2
        exit 1
    fi

    # `cargo publish` needs a token. `cargo login` stores one.
    if [ -z "${CARGO_REGISTRY_TOKEN:-}" ] && [ ! -f "$HOME/.cargo/credentials.toml" ] && [ ! -f "$HOME/.cargo/credentials" ]; then
        echo "No crates.io token found." >&2
        echo "Put CARGO_REGISTRY_TOKEN in .env, or run 'cargo login'." >&2
        exit 1
    fi
fi

publish_one() {
    crate=$1
    echo
    echo "=== $crate ==="

    if [ "$DRY_RUN" -eq 1 ]; then
        cargo publish --dry-run --allow-dirty --package "$crate"
        return
    fi

    # A crate that is already on crates.io cannot be published again, so it
    # is skipped. That makes the script safe to run again after a failure.
    if already_published "$crate"; then
        echo "$crate $VERSION is already on crates.io. Skipping."
        return
    fi

    # The crate has to be indexed before the next one can depend on it, so
    # a failure is retried for a while before giving up.
    attempt=1
    while [ "$attempt" -le 30 ]; do
        log=$(mktemp)
        # The output goes to the log first, because a pipeline would report
        # the status of the last command rather than of cargo.
        if cargo publish --package "$crate" --allow-dirty >"$log" 2>&1; then
            cat "$log"
            rm -f "$log"
            echo "Published $crate"
            return
        fi
        cat "$log"

        # A crate that is already on crates.io cannot be published again.
        if grep -qiE "already exists|already uploaded" "$log"; then
            rm -f "$log"
            echo "$crate $VERSION is already on crates.io. Skipping."
            return
        fi

        # A token that is not accepted will not be accepted on the next
        # attempt either, so there is no point in waiting.
        if grep -qiE "invalid token|unauthorized|not authorized|401|403|forbidden|missing or invalid|required permissions" "$log"; then
            rm -f "$log"
            echo "crates.io rejected the token. Check CARGO_REGISTRY_TOKEN in .env." >&2
            echo "It needs the 'publish-new-crate' scope the first time." >&2
            exit 1
        fi

        # crates.io limits how many new crates an account may create in a
        # short period. That clears on its own, but not in thirty seconds.
        wait_for=30
        if grep -qiE "429|too many (new )?crates|rate limit" "$log"; then
            wait_for=120
            echo "crates.io is limiting how many new crates can be created." >&2
            echo "Waiting ${wait_for} seconds before trying again..." >&2
        fi
        rm -f "$log"

        echo "Attempt $attempt failed. Waiting ${wait_for} seconds..." >&2
        attempt=$((attempt + 1))
        sleep "$wait_for"
    done

    echo "Could not publish $crate" >&2
    exit 1
}

previous=""
for crate in $CRATES; do
    # crates.io needs a moment to index a crate before another crate can
    # depend on it. Waiting here means the next crate resolves on the first
    # attempt instead of failing and being retried.
    if [ -n "$previous" ]; then
        echo
        echo "Waiting for crates.io to index $previous..."
        sleep 60
    fi
    publish_one "$crate"
    previous=$crate
done

echo
echo "Published: $CRATES"
echo "They can be installed with:"
echo "  cargo install frs-cli"
