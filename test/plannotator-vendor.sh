#!/usr/bin/env bash
set -uo pipefail

HARNESS_SOURCE="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
failures=0

pass() { printf 'ok   %s\n' "$1"; }

fail() {
  printf 'FAIL %s\n' "$1" >&2
  failures=$((failures + 1))
}

source "$HARNESS_SOURCE/vendor-skills.sh"

expected_core=$(printf '%s\n' plannotator-annotate plannotator-last plannotator-review)
actual_core=$(printf '%s\n' "${PLANNOTATOR_CORE_SKILLS[@]}")
expected_extra=$(printf '%s\n' plannotator-compound plannotator-visual-explainer)
actual_extra=$(printf '%s\n' "${PLANNOTATOR_EXTRA_SKILLS[@]}")
expected_all=$(printf '%s\n%s\n' "$expected_core" "$expected_extra")
actual_all=$(printf '%s\n' "${PLANNOTATOR_SKILLS[@]}")

if [ "$actual_core" = "$expected_core" ]; then
  pass "the Plannotator core set is explicit"
else
  fail "the Plannotator core set is explicit"
fi

if [ "$actual_extra" = "$expected_extra" ]; then
  pass "the Plannotator extra set is explicit"
else
  fail "the Plannotator extra set is explicit"
fi

if [ "$actual_all" = "$expected_all" ]; then
  pass "the selected Plannotator set derives from the source sets"
else
  fail "the selected Plannotator set derives from the source sets"
fi

fetch_body=$(declare -f fetch_plannotator_upstreams)
if [[ "$fetch_body" == *'${PLANNOTATOR_CORE_SKILLS[@]}'* ]] &&
  [[ "$fetch_body" == *'${PLANNOTATOR_EXTRA_SKILLS[@]}'* ]] &&
  [[ "$fetch_body" != *'${PLANNOTATOR_SKILLS[@]:'* ]]; then
  pass "Plannotator fetches each source set without positional slicing"
else
  fail "Plannotator fetches each source set without positional slicing"
fi

wrong_paths=""
for skill in "${PLANNOTATOR_CORE_SKILLS[@]}"; do
  path=$(jq -r --arg skill "$skill" '.skills[$skill].skillPath // ""' "$HARNESS_SOURCE/skills-lock.json")
  [[ "$path" == apps/skills/core/* ]] || wrong_paths="$wrong_paths $skill:$path"
done
for skill in "${PLANNOTATOR_EXTRA_SKILLS[@]}"; do
  path=$(jq -r --arg skill "$skill" '.skills[$skill].skillPath // ""' "$HARNESS_SOURCE/skills-lock.json")
  [[ "$path" == apps/skills/extra/* ]] || wrong_paths="$wrong_paths $skill:$path"
done

if [ -z "${wrong_paths// /}" ]; then
  pass "each Plannotator pin comes from its declared source set"
else
  fail "each Plannotator pin comes from its declared source set:$wrong_paths"
fi

if printf '%s\n' "$actual_all" | grep -qxF plannotator-setup-goal; then
  fail "the selected Plannotator set excludes setup-goal"
else
  pass "the selected Plannotator set excludes setup-goal"
fi

if [ "$failures" -eq 0 ]; then
  echo "plannotator-vendor: all assertions passed"
else
  printf 'plannotator-vendor: %d assertion(s) failed\n' "$failures" >&2
  exit 1
fi
