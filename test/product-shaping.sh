#!/usr/bin/env bash
set -uo pipefail

HARNESS_SOURCE="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
router="$HARNESS_SOURCE/skills/pstack-poteto-mode/SKILL.md"
playbook="$HARNESS_SOURCE/skills/pstack-poteto-mode/playbooks/product-shaping.md"
failures=0

pass() { printf 'ok   %s\n' "$1"; }

fail() {
  printf 'FAIL %s\n' "$1" >&2
  failures=$((failures + 1))
}

assert_contains() {
  if grep -qF -- "$2" "$3"; then pass "$1"; else fail "$1"; fi
}

assert_not_contains() {
  if grep -qF -- "$2" "$3"; then fail "$1"; else pass "$1"; fi
}

assert_contains "the router lists Product shaping" '- **Product shaping.**' "$router"
assert_contains "the router reaches the Product shaping playbook" \
  '`playbooks/product-shaping.md`' "$router"

for trigger in 'shape this idea' 'grill this' 'grill with docs' 'wayfind this' \
  'turn this into a spec' 'split this into tickets'; do
  assert_contains "the router names the '$trigger' trigger" "$trigger" "$router"
done

for stage in matt-grill-with-docs matt-grill-me matt-wayfinder matt-research matt-prototype \
  matt-to-questionnaire matt-to-spec matt-to-tickets; do
  assert_contains "Product shaping names $stage" "$stage" "$playbook"
  if [ -f "$HARNESS_SOURCE/skills/$stage/SKILL.md" ]; then
    pass "Product shaping can reach $stage"
  else
    fail "Product shaping can reach $stage"
  fi
done

assert_contains "Product shaping resolves the tracker through CLAUDE.md" \
  'Tracker resolution order' "$playbook"
assert_contains "Product shaping passes resolved tracker context to Matt" \
  'Give the leaf the resolved tracker, its commands, and its conventions.' "$playbook"
assert_contains "Product shaping requires an approved spec before implementation" \
  'Do not start implementation before that approval.' "$playbook"
assert_contains "Product shaping routes one bounded feature to Feature" \
  'Route one bounded feature to **Feature**.' "$playbook"
assert_contains "Product shaping routes standing sandbox programs to Orchestrate" \
  'Route a standing sandbox program to **Orchestrate**.' "$playbook"
assert_contains "Product shaping forbids a second start gate" \
  'without another start gate' "$playbook"
assert_contains "Product shaping stops Matt tickets for Beads programs" \
  'Do not call **matt-to-tickets**' "$playbook"
assert_contains "Product shaping forbids a duplicate GitHub graph" \
  'Do not create a duplicate GitHub implementation dependency graph.' "$playbook"
assert_contains "Product shaping makes tickets the sole graph" \
  'sole implementation graph' "$playbook"
assert_not_contains "Product shaping never routes to the absent setup skill" \
  '/setup-matt-pocock-skills' "$playbook"

for retired in matt-ask-matt matt-implement; do
  if [ -e "$HARNESS_SOURCE/skills/$retired" ]; then
    fail "$retired remains absent"
  else
    pass "$retired remains absent"
  fi
done

router_count=$(grep -rl '^mode: true$' "$HARNESS_SOURCE/skills"/*/SKILL.md | grep -c .)
if [ "$router_count" -eq 1 ] && grep -q '^mode: true$' "$router"; then
  pass "pstack-poteto-mode remains the only mode router"
else
  fail "pstack-poteto-mode remains the only mode router"
fi

if [ "$failures" -eq 0 ]; then
  echo "product-shaping: all assertions passed"
else
  printf 'product-shaping: %d assertion(s) failed\n' "$failures" >&2
  exit 1
fi
