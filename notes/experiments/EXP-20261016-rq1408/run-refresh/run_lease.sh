#!/bin/bash
# usage: run_lease.sh <configdir> <outfile> [extra claude args]
set -u
cfg=$1; out=$2; shift 2
for v in $(env | grep -o '^CLAUDE[^=]*'); do unset "$v"; done
unset AI_AGENT
export CLAUDE_CONFIG_DIR=$cfg
cd "$(dirname "$out")"
claude -p "Reply with just the word ok." --output-format json --model sonnet "$@" > "$out" 2> "$out.stderr"
echo "exit=$?"
python3 -c "
import json,sys
try:
    d=json.load(open('$out'))
    print({k:d.get(k) for k in ('type','subtype','is_error','result','duration_ms','num_turns')})
except Exception as e:
    print('no json:', e); print(open('$out').read()[:500])
"
echo "stderr: $(head -c 400 "$out.stderr")"
