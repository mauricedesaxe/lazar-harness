#!/usr/bin/env bash
set -uo pipefail

HARNESS_SOURCE="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
HELPER="$HARNESS_SOURCE/skills/pstack-poteto-mode/scripts/spec-body-hash.sh"
TMP=$(mktemp -d)
failures=0
trap 'rm -rf -- "$TMP"' EXIT

pass() { printf 'ok   %s\n' "$1"; }
fail() { printf 'FAIL %s\n' "$1" >&2; failures=$((failures + 1)); }

mkdir -p "$TMP/bin" "$TMP/target-repo"
printf 'target repository\n' >"$TMP/target-repo/.fake-target"
cat >"$TMP/bin/gh" <<'GH'
#!/usr/bin/env bash
printf '%s\n' "$*" >>"$GH_LOG"
printf '%s\n' "$PWD" >>"${GH_CWD_LOG:-/dev/null}"
if [ "$1 $2" = "repo view" ]; then
  [ "${GH_REQUIRE_TARGET:-0}" = 1 ] && [ ! -f .fake-target ] && exit 9
  [ "${GH_SCENARIO:-success}" = repo_failure ] && exit 1
  [ "${GH_SCENARIO:-success}" = empty_repo ] && exit 0
  [ "${GH_SCENARIO:-success}" = invalid_repo ] && { printf 'not-a-repository\n'; exit 0; }
  printf 'mauricedesaxe/lazar-harness\n'
  exit 0
fi

case "${GH_SCENARIO:-success}" in
  fetch_failure) exit 1 ;;
  malformed_json) printf '{broken\n' ;;
  empty_body) printf '{"body":""}\n' ;;
  whitespace_body) printf '{"body":"   "}\n' ;;
  wrong_shape) printf '{"body":"Approved body","extra":true}\n' ;;
  *) printf '{"body":"Approved body"}\n' ;;
esac
GH
chmod +x "$TMP/bin/gh"

run_helper() {
  (cd "$TMP/target-repo" &&
    GH_LOG="$TMP/gh.log" GH_CWD_LOG="$TMP/gh.cwd" GH_REQUIRE_TARGET=1 GH_SCENARIO=$1 \
      PATH="$TMP/bin:$PATH" "$HELPER" 42)
}

: >"$TMP/gh.log"
out=$(run_helper success 2>"$TMP/error"); status=$?
expected=99f3086cc285dbc973bb620c32c93a52d426b7a29751bf8570f9393f77bbf643
if [ "$status" -eq 0 ] && [ "$out" = "$expected" ] && [ ! -s "$TMP/error" ]; then
  pass "a valid GitHub body produces only its canonical SHA-256"
else
  fail "a valid GitHub body produces only its canonical SHA-256"
fi

if grep -qxF 'repo view --json nameWithOwner --jq .nameWithOwner' "$TMP/gh.log" &&
  grep -qxF 'issue view 42 -R mauricedesaxe/lazar-harness --json body' "$TMP/gh.log"; then
  pass "the helper resolves one repository and targets the issue read to it"
else
  fail "the helper resolves one repository and targets the issue read to it"
fi

if [[ "$HELPER" != "$TMP/target-repo"/* ]] &&
  [ "$(sort -u "$TMP/gh.cwd")" = "$TMP/target-repo" ]; then
  pass "an external absolute helper path resolves GitHub from the target repository cwd"
else
  fail "an external absolute helper path resolves GitHub from the target repository cwd"
fi

assert_failure() {
  local name=$1 scenario=$2 out status
  out=$(run_helper "$scenario" 2>"$TMP/error"); status=$?
  if [ "$status" -ne 0 ] && [ -z "$out" ]; then pass "$name"; else fail "$name"; fi
}

assert_failure "a repository lookup failure stops approval" repo_failure
assert_failure "an empty repository lookup stops approval" empty_repo
assert_failure "an invalid repository lookup stops approval" invalid_repo
assert_failure "a body fetch failure stops approval" fetch_failure
assert_failure "malformed JSON stops approval" malformed_json
assert_failure "an empty body stops approval" empty_body
assert_failure "a whitespace-only body stops approval" whitespace_body
assert_failure "an unexpected JSON shape stops approval" wrong_shape

make_path() {
  local directory=$1
  shift
  mkdir -p "$directory"
  for tool in "$@"; do ln -s "$(command -v "$tool")" "$directory/$tool"; done
}

make_path "$TMP/no-gh" bash jq sha256sum mktemp rm
out=$(PATH="$TMP/no-gh" "$HELPER" 42 2>"$TMP/error"); status=$?
if [ "$status" -ne 0 ] && [ -z "$out" ]; then pass "a missing gh stops approval"; else fail "a missing gh stops approval"; fi

make_path "$TMP/no-jq" bash sha256sum mktemp rm
cp "$TMP/bin/gh" "$TMP/no-jq/gh"
out=$(GH_LOG="$TMP/gh.log" PATH="$TMP/no-jq" "$HELPER" 42 2>"$TMP/error"); status=$?
if [ "$status" -ne 0 ] && [ -z "$out" ]; then pass "a missing jq stops approval"; else fail "a missing jq stops approval"; fi

make_path "$TMP/no-mktemp" bash jq sha256sum rm
cp "$TMP/bin/gh" "$TMP/no-mktemp/gh"
out=$(GH_LOG="$TMP/gh.log" PATH="$TMP/no-mktemp" "$HELPER" 42 2>"$TMP/error"); status=$?
if [ "$status" -ne 0 ] && [ -z "$out" ]; then pass "a missing runtime tool stops approval"; else fail "a missing runtime tool stops approval"; fi

make_path "$TMP/no-hash" bash jq mktemp rm
cp "$TMP/bin/gh" "$TMP/no-hash/gh"
out=$(GH_LOG="$TMP/gh.log" PATH="$TMP/no-hash" "$HELPER" 42 2>"$TMP/error"); status=$?
if [ "$status" -ne 0 ] && [ -z "$out" ]; then pass "missing SHA-256 tools stop approval"; else fail "missing SHA-256 tools stop approval"; fi

make_path "$TMP/shasum-only" bash jq shasum mktemp rm
cp "$TMP/bin/gh" "$TMP/shasum-only/gh"
out=$(GH_LOG="$TMP/gh.log" PATH="$TMP/shasum-only" "$HELPER" 42 2>"$TMP/error"); status=$?
if [ "$status" -eq 0 ] && [ "$out" = "$expected" ]; then pass "shasum is a working fallback"; else fail "shasum is a working fallback"; fi

make_path "$TMP/hash-failure" bash jq mktemp rm
cp "$TMP/bin/gh" "$TMP/hash-failure/gh"
cat >"$TMP/hash-failure/sha256sum" <<'HASH'
#!/usr/bin/env bash
exit 1
HASH
chmod +x "$TMP/hash-failure/sha256sum"
out=$(GH_LOG="$TMP/gh.log" PATH="$TMP/hash-failure" "$HELPER" 42 2>"$TMP/error"); status=$?
if [ "$status" -ne 0 ] && [ -z "$out" ]; then pass "a hash process failure stops approval"; else fail "a hash process failure stops approval"; fi

make_path "$TMP/hash-invalid" bash jq mktemp rm
cp "$TMP/bin/gh" "$TMP/hash-invalid/gh"
cat >"$TMP/hash-invalid/sha256sum" <<'HASH'
#!/usr/bin/env bash
printf 'not-a-hash  %s\n' "$1"
HASH
chmod +x "$TMP/hash-invalid/sha256sum"
out=$(GH_LOG="$TMP/gh.log" PATH="$TMP/hash-invalid" "$HELPER" 42 2>"$TMP/error"); status=$?
if [ "$status" -ne 0 ] && [ -z "$out" ]; then pass "an invalid hash result stops approval"; else fail "an invalid hash result stops approval"; fi

if [ "$failures" -eq 0 ]; then
  printf 'spec-body-hash: all assertions passed\n'
  exit 0
fi
printf 'spec-body-hash: %d assertion(s) failed\n' "$failures" >&2
exit 1
