#!/bin/sh
# Watch a workflow run until it finishes, without touching the GitHub API.
#
#   ./scripts/watch-run.sh <run-id> [max-minutes]
#
# The Actions page carries the run state in an embedded JSON blob, so this
# does not use up the API rate limit.
set -eu

RUN_ID=${1:?usage: watch-run.sh <run-id> [max-minutes]}
MAX_MINUTES=${2:-40}
PAGE=/tmp/frag-watch-$$.html

i=0
while [ "$i" -lt $((MAX_MINUTES * 2)) ]; do
    curl -sSL "https://github.com/Jesse-Dan/fragment-rs/actions/workflows/release.yml" -o "$PAGE" 2>/dev/null || true

    state=$(RUN_ID="$RUN_ID" python3 - "$PAGE" <<'PY'
import json, os, sys

run_id = os.environ["RUN_ID"]
page = open(sys.argv[1]).read()
marker = '"workflowRuns":'
start = page.find(marker)
if start < 0:
    print("unknown")
    raise SystemExit
start += len(marker)
try:
    runs, _ = json.JSONDecoder().raw_decode(page, start)
except ValueError:
    print("unknown")
    raise SystemExit
for run in runs:
    if str(run["id"]) == run_id:
        print(run["state"]["status"], run.get("timing", {}).get("text", ""))
        break
else:
    print("unknown")
PY
)

    echo "[$(date +%H:%M:%S)] $state"
    case "$state" in
        *COMPLETED*) break ;;
    esac
    i=$((i + 1))
    sleep 30
done

rm -f "$PAGE"

echo
echo "=== release ==="
curl -sSLI "https://github.com/Jesse-Dan/fragment-rs/releases/latest" 2>/dev/null | grep -i "^location:" || true
curl -sSL "https://github.com/Jesse-Dan/fragment-rs/releases" 2>/dev/null |
    grep -oE "releases/tag/v[0-9.]+|frs-(linux|macos|windows)-[a-z0-9]+\.(tar\.gz|zip)" | sort -u || true
