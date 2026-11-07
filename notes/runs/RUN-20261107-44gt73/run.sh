#!/usr/bin/env bash
# RUN-20261107-44gt73 (EXP-20261107-413q6m): the long-lived token alone, on the host.
# From the repo root. The token is read from ~/.config/rc2s2/claude-oauth-token into the
# environment of the claude processes only; nothing here prints it. Keychain items are
# listed by service name only (dump-keychain without -d prints no secrets).
set -uo pipefail
here=notes/runs/RUN-20261107-44gt73
out=$here/out
mkdir -p "$out"
cfg=$(mktemp -d /private/tmp/rc2s2-token.XXXX)
token=$(cat "$HOME/.config/rc2s2/claude-oauth-token")
items() { security dump-keychain 2>/dev/null | grep -o '"svce"<blob>="Claude Code[^"]*"' | sort | uniq -c; }

items > "$out/keychain-before.txt"
run() { env -u ANTHROPIC_API_KEY CLAUDE_CONFIG_DIR="$cfg" CLAUDE_CODE_OAUTH_TOKEN="$token" \
  claude -p --input-format stream-json --output-format stream-json --verbose --model haiku "$@"; }

printf '%s\n' '{"type":"user","message":{"role":"user","content":"Remember the code word OCHRE-17. Reply with OK only."}}' \
  | run > "$out/turn1.jsonl" 2> "$out/turn1.err"
sid=$(jq -r 'select(.type=="system" and .subtype=="init") | .session_id' "$out/turn1.jsonl" | head -1)
printf '%s\n' '{"type":"user","message":{"role":"user","content":"What was the code word? Reply with the word only."}}' \
  | run --resume "$sid" > "$out/turn2.jsonl" 2> "$out/turn2.err"
items > "$out/keychain-after.txt"
(cd "$cfg" && find . -type f | sort) > "$out/config-listing.txt"
owner=$(claude auth status 2>/dev/null | jq -r '.loggedIn // empty')

jq -nc \
  --arg sid "$sid" \
  --arg sid2 "$(jq -r 'select(.type=="system" and .subtype=="init") | .session_id' "$out/turn2.jsonl" | head -1)" \
  --arg r1 "$(jq -r 'select(.type=="result") | .subtype' "$out/turn1.jsonl")" \
  --arg r2 "$(jq -r 'select(.type=="result") | .result' "$out/turn2.jsonl")" \
  --argjson rl "$(jq -s '[.[] | select(.type=="rate_limit_event")] | length' "$out/turn1.jsonl" "$out/turn2.jsonl")" \
  --arg auth "$(jq -r 'select(.type=="system" and .subtype=="init") | .apiKeySource // "?"' "$out/turn1.jsonl" | head -1)" \
  --argjson same_keychain "$(cmp -s "$out/keychain-before.txt" "$out/keychain-after.txt" && echo true || echo false)" \
  --argjson cred_file "$(grep -q credentials "$out/config-listing.txt" && echo true || echo false)" \
  --arg owner "$owner" \
  '{session_id:$sid, resumed_id:$sid2, turn1:$r1, code_word_kept:($r2|test("OCHRE-17")), rate_limit_events:$rl,
    api_key_source:$auth, keychain_unchanged:$same_keychain, credential_file_written:$cred_file, owner_logged_in:$owner}' \
  | sed 's/^/===RESULT=== /' | tee "$here/run.log"
rm -rf "$cfg"
