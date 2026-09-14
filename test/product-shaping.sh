#!/usr/bin/env bash
set -uo pipefail

HARNESS_SOURCE="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
router="$HARNESS_SOURCE/skills/pstack-poteto-mode/SKILL.md"
playbook="$HARNESS_SOURCE/skills/pstack-poteto-mode/playbooks/product-shaping.md"
orchestrate="$HARNESS_SOURCE/skills/pstack-poteto-mode/playbooks/orchestrate.md"
tracker="$HARNESS_SOURCE/docs/agents/issue-tracker.md"
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
assert_contains "Product shaping persists approval on the spec" \
  'Approved-Spec-Body-SHA256: <hash>' "$playbook"
assert_contains "Product shaping hashes the current tracker body" \
  'gh issue view <spec-number> --json body | jq -cS .' "$playbook"
assert_contains "Product shaping starts only after the marker exists" \
  'Do not start implementation before the marker exists.' "$playbook"
assert_contains "Product shaping routes one bounded feature to Feature" \
  'Route one bounded feature to **Feature**.' "$playbook"
assert_contains "Product shaping routes standing sandbox programs to Orchestrate" \
  'Route a standing sandbox program to **Orchestrate**.' "$playbook"
assert_contains "Product shaping forbids a second start gate" \
  'without another start gate' "$playbook"
assert_contains "Product shaping has a local implementation-graph block" \
  '<!-- surface:local -->' "$playbook"
assert_contains "Product shaping has a sandbox implementation-graph block" \
  '<!-- surface:sandbox -->' "$playbook"
assert_contains "local Product shaping can create tracker tickets" \
  'For multi-ticket work, call **matt-to-tickets**' "$playbook"
assert_contains "sandbox Product shaping stops Matt at the approved spec" \
  'stop Matt at the approved tracker spec' "$playbook"
assert_contains "sandbox Product shaping does not create Matt tickets" \
  'Do not call **matt-to-tickets**' "$playbook"
assert_contains "sandbox Product shaping forbids a duplicate GitHub graph" \
  'Do not publish an implementation dependency graph there.' "$playbook"
assert_contains "sandbox Product shaping makes Orchestrate derive the sole graph" \
  'derives the sole implementation task graph' "$playbook"

assert_contains "Orchestrate reads the approved spec body and comments" \
  'Read its full body and every comment.' "$orchestrate"
assert_contains "Orchestrate follows a linked Wayfinder map" \
  'If it links a Wayfinder map' "$orchestrate"
assert_contains "Orchestrate reads required closed decision tickets" \
  'every closed decision ticket that the spec needs' "$orchestrate"
assert_contains "Orchestrate requires the shared approval marker" \
  'Approved-Spec-Body-SHA256: <hash>' "$orchestrate"
assert_contains "Orchestrate recomputes the current tracker body hash" \
  'gh issue view <spec-number> --json body | jq -cS .' "$orchestrate"
assert_contains "Orchestrate compares the newest marker with the current hash" \
  'newest approval marker equals the current hash' "$orchestrate"
assert_contains "Orchestrate rejects a stale approval marker" \
  'marker is absent or stale' "$orchestrate"
assert_contains "Orchestrate rejects unresolved decisions" \
  'ticket remains open' "$orchestrate"
assert_contains "Orchestrate rejects in-scope fog" \
  'under `Not yet specified`' "$orchestrate"
assert_contains "Orchestrate keeps one Beads implementation graph" \
  'one implementation task graph.' "$orchestrate"
assert_contains "Orchestrate derives tasks from product sources and code" \
  'approved spec, its product sources, and' "$orchestrate"
assert_contains "Orchestrate briefs identify their product sources" \
  'SOURCE       approved spec URL and relevant closed decision issue URLs' "$orchestrate"
assert_contains "Orchestrate briefs keep complete execution context" \
  'CONTEXT      files, PRs, and complete upstream reports needed by this task' "$orchestrate"
assert_contains "Orchestrate records source links on Beads tasks" \
  '--description "SOURCE <relevant-source-urls>"' "$orchestrate"
assert_contains "Orchestrate does not ask again after product approval" \
  'Do not ask for another implementation approval after admission.' "$orchestrate"
assert_contains "Orchestrate pauses only tasks that depend on a new decision" \
  'Pause only the Beads tasks that depend on that decision.' "$orchestrate"

for operation in 'wayfinder:map' '/sub_issues' '/dependencies/blocked_by' \
  '--add-assignee @me' 'gh issue close' 'Decisions so far'; do
  assert_contains "the GitHub tracker documents $operation" "$operation" "$tracker"
done
assert_contains "the tracker distinguishes database IDs from issue numbers" \
  'Issue numbers are not database IDs.' "$tracker"
assert_contains "the tracker documents a sub-issue fallback" \
  'fallback parent relation' "$tracker"
assert_contains "the tracker records the exact approval marker" \
  "printf 'Approved-Spec-Body-SHA256: %s" "$tracker"
assert_contains "the tracker canonicalizes the body before hashing" \
  'gh issue view "$spec" --json body | jq -cS .' "$tracker"
assert_contains "the tracker supports the macOS hash command" \
  'shasum -a 256' "$tracker"
assert_contains "the tracker invalidates approval after a body edit" \
  'Any later body edit invalidates the marker.' "$tracker"
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
