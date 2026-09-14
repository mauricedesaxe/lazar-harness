#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 1 ] || [ -z "$1" ]; then
  printf 'usage: spec-body-hash.sh <spec-number>\n' >&2
  exit 2
fi

for tool in gh jq mktemp rm; do
  command -v "$tool" >/dev/null 2>&1 || {
    printf 'spec-body-hash.sh: %s is required\n' "$tool" >&2
    exit 1
  }
done

if command -v sha256sum >/dev/null 2>&1; then
  hash_command=sha256sum
elif command -v shasum >/dev/null 2>&1; then
  hash_command=shasum
else
  printf 'spec-body-hash.sh: sha256sum or shasum is required\n' >&2
  exit 1
fi

repo=$(gh repo view --json nameWithOwner --jq .nameWithOwner)
if ! [[ "$repo" =~ ^[^/[:space:]]+/[^/[:space:]]+$ ]]; then
  printf 'spec-body-hash.sh: gh returned an invalid repository\n' >&2
  exit 1
fi

response=$(mktemp)
canonical=$(mktemp)
cleanup() {
  rm -f -- "$response" "$canonical"
}
trap cleanup EXIT

gh issue view "$1" -R "$repo" --json body >"$response"
jq -ceS '
  if type == "object"
    and keys == ["body"]
    and (.body | type) == "string"
    and (.body | test("\\S"))
  then .
  else error("expected an object with one non-empty string body")
  end
' "$response" >"$canonical"

if [ "$hash_command" = sha256sum ]; then
  hash_output=$(sha256sum "$canonical")
else
  hash_output=$(shasum -a 256 "$canonical")
fi
hash=${hash_output%% *}

if ! [[ "$hash" =~ ^[0-9a-f]{64}$ ]]; then
  printf 'spec-body-hash.sh: hash command returned an invalid SHA-256\n' >&2
  exit 1
fi
printf '%s\n' "$hash"
