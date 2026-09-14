#!/usr/bin/env bash
set -uo pipefail

HARNESS_SOURCE="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
router="$HARNESS_SOURCE/skills/pstack-poteto-mode/SKILL.md"
playbook="$HARNESS_SOURCE/skills/pstack-poteto-mode/playbooks/product-shaping.md"
orchestrate="$HARNESS_SOURCE/skills/pstack-poteto-mode/playbooks/orchestrate.md"
tracker="$HARNESS_SOURCE/docs/agents/issue-tracker.md"
wayfinder="$HARNESS_SOURCE/skills/matt-wayfinder/SKILL.md"
to_spec="$HARNESS_SOURCE/skills/matt-to-spec/SKILL.md"
to_tickets="$HARNESS_SOURCE/skills/matt-to-tickets/SKILL.md"
prototype="$HARNESS_SOURCE/skills/matt-prototype"
hash_helper="$HARNESS_SOURCE/skills/pstack-poteto-mode/scripts/spec-body-hash.sh"
beads_readme="$HARNESS_SOURCE/.beads/README.md"
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

assert_in_order() {
  local name=$1 file=$2 previous=0 phrase line
  shift 2
  for phrase in "$@"; do
    line=$(grep -nF -m1 -- "$phrase" "$file" | cut -d: -f1)
    if [ -z "$line" ] || [ "$line" -le "$previous" ]; then
      fail "$name"
      return
    fi
    previous=$line
  done
  pass "$name"
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
assert_contains "Product shaping invokes the installed helper by absolute path" \
  '<absolute-installed-skill-path>/scripts/spec-body-hash.sh <spec-number>' "$playbook"
assert_contains "Product shaping keeps the target repository as cwd" \
  'Keep the target repository as the current directory.' "$playbook"
assert_contains "Product shaping targets approval comments to the resolved repository" \
  'gh issue comment <spec-number> -R "$repo"' "$playbook"
assert_contains "Product shaping requires a Wayfinder map source" \
  'A spec from Wayfinder must include a `## Product sources` section' "$playbook"
assert_contains "Product shaping starts only after the marker exists" \
  'Do not start implementation before the marker exists.' "$playbook"
assert_contains "Product shaping publishes an unapproved draft for a human" \
  'Publish the unapproved draft on the resolved tracker as `ready-for-human`, or leave it unlabeled.' "$playbook"
assert_contains "Product shaping forbids premature agent readiness" \
  'Never apply `ready-for-agent` to an unapproved draft.' "$playbook"
assert_contains "Product shaping applies agent readiness only after the marker" \
  'Only after the marker exists may Product shaping replace `ready-for-human` with `ready-for-agent`.' "$playbook"
assert_contains "Product shaping keeps Matt leaves stage-bound" \
  'A Product shaping Matt leaf shapes only its named stage.' "$playbook"
assert_contains "Product shaping keeps natural-language Matt reachability" \
  'Natural-language reachability stays enabled.' "$playbook"
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
  'A spec from Wayfinder must contain a `## Product sources` section' "$orchestrate"
assert_contains "Orchestrate reads required closed decision tickets" \
  'For every in-scope child, query the issue and its full comments.' "$orchestrate"
assert_contains "Orchestrate requires the shared approval marker" \
  'Approved-Spec-Body-SHA256: <hash>' "$orchestrate"
assert_contains "Orchestrate invokes the installed helper by absolute path" \
  '<absolute-installed-skill-path>/scripts/spec-body-hash.sh <spec-number>' "$orchestrate"
assert_contains "Orchestrate keeps the target repository as cwd" \
  'Keep the target repository as the current' "$orchestrate"
assert_contains "Orchestrate resolves one GitHub repository" \
  'gh repo view --json nameWithOwner --jq .nameWithOwner' "$orchestrate"
assert_contains "Orchestrate compares the newest marker with the current hash" \
  'newest exact `Approved-Spec-Body-SHA256: <hash>` marker must equal the' "$orchestrate"
assert_contains "Orchestrate rejects a stale approval marker" \
  'A missing or stale marker rejects admission.' "$orchestrate"
assert_contains "Orchestrate queries every native map child" \
  'query all native sub-issues with the paginated `sub_issues` API' "$orchestrate"
assert_contains "Orchestrate supports the complete fallback checklist" \
  'Use every linked checklist child' "$orchestrate"
assert_contains "Orchestrate rejects unresolved decisions" \
  'Reject open children' "$orchestrate"
assert_contains "Orchestrate rejects non-completed closure reasons" \
  '`NOT_PLANNED` closure, duplicate closure' "$orchestrate"
assert_contains "Orchestrate requires a non-empty resolution" \
  'a non-empty resolution comment whose first line is' "$orchestrate"
assert_contains "Orchestrate keeps unmapped bounded specs valid" \
  'Keep bounded specs without a map valid.' "$orchestrate"
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
assert_contains "Orchestrate records the approved hash on Beads tasks" \
  '--description "APPROVED_SPEC_SHA256 <hash>' "$orchestrate"
assert_contains "Orchestrate revalidates before graph mutations" \
  'any Beads graph mutation' "$orchestrate"
assert_contains "Orchestrate revalidates before new children" \
  'before each new child starts' "$orchestrate"
assert_contains "Orchestrate bootstraps recovery before admission" \
  'After a runtime restart, run `bd bootstrap`, `bd dolt pull`, and `bd prime`.' "$orchestrate"
assert_contains "Orchestrate reads the epic admission record before admission" \
  'record with `bd show <epic-id>`.' "$orchestrate"
assert_contains "Orchestrate blocks recovery mutations before admission" \
  'Do not run any Beads graph mutation or start a new child' "$orchestrate"
assert_in_order "Orchestrate pins the cold recovery order" "$orchestrate" \
  'After a runtime restart, run `bd bootstrap`, `bd dolt pull`, and `bd prime`.' \
  'Read the epic admission record with `bd show <epic-id>`.' \
  'Then repeat the full admission check.' \
  'Do not run any Beads graph mutation or start a new child'
assert_contains "Orchestrate does not ask again after product approval" \
  'Do not ask for another implementation' "$orchestrate"
assert_contains "Orchestrate pauses only tasks that depend on a new decision" \
  'Pause only the Beads tasks that depend on that decision.' "$orchestrate"

for operation in 'wayfinder:map' '/sub_issues' '/dependencies/blocked_by' \
  'gh issue close' 'Decisions so far'; do
  assert_contains "the GitHub tracker documents $operation" "$operation" "$tracker"
done
assert_contains "the tracker distinguishes database IDs from issue numbers" \
  'Issue numbers are not database IDs.' "$tracker"
assert_contains "the tracker documents a sub-issue fallback" \
  'fallback parent relation' "$tracker"
assert_contains "the tracker names the real repository" \
  'mauricedesaxe/lazar-harness' "$tracker"
assert_not_contains "the tracker drops the stale repository name" \
  'mauricedesaxe/claude-harness-template' "$tracker"
assert_contains "the tracker resolves repository operations dynamically" \
  'repo=$(gh repo view --json nameWithOwner --jq .nameWithOwner)' "$tracker"
assert_contains "the tracker records the exact approval marker" \
  "printf 'Approved-Spec-Body-SHA256: %s" "$tracker"
assert_contains "the tracker invokes the helper by absolute path" \
  'hash_helper=/absolute/path/to/pstack-poteto-mode/scripts/spec-body-hash.sh' "$tracker"
assert_contains "the tracker keeps the target repository as cwd" \
  'cd "$target_repo"' "$tracker"
assert_contains "the tracker calls the helper without changing cwd" \
  'body_sha=$("$hash_helper" "$spec")' "$tracker"
assert_contains "the tracker targets issue commands to one repository" \
  'gh issue comment "$spec" -R "$repo"' "$tracker"
assert_contains "the tracker posts exact Wayfinder claim comments" \
  "printf 'Wayfinder-Claim: %s" "$tracker"
assert_contains "the tracker fetches claim comments through the resolved repository" \
  'repos/$repo/issues/<number>/comments?per_page=100' "$tracker"
assert_contains "the tracker chooses the earliest valid claim" \
  'first_claim=$(printf' "$tracker"
assert_contains "the tracker releases a losing claim" \
  "printf 'Wayfinder-Claim-Released: %s" "$tracker"
untargeted_issue_commands=$(grep -E 'gh (issue (list|view|create|edit|comment|close)|label create)' \
  "$tracker" | grep -vF -- '-R "$repo"' || true)
if [ -z "$untargeted_issue_commands" ]; then
  pass "every documented issue command targets the resolved repository"
else
  fail "every documented issue command targets the resolved repository"
fi
assert_not_contains "the tracker has no hardcoded API repository" \
  'repos/mauricedesaxe/' "$tracker"
assert_contains "the tracker invalidates approval after a body edit" \
  'A later body edit makes' "$tracker"
assert_not_contains "Product shaping never routes to the absent setup skill" \
  '/setup-matt-pocock-skills' "$playbook"

for tracker_leaf in "$wayfinder" "$to_spec" "$to_tickets"; do
  assert_not_contains "$(basename -- "$(dirname -- "$tracker_leaf")") has no absent setup command" \
    '/setup-matt-pocock-skills' "$tracker_leaf"
done
assert_contains "matt-to-spec publishes an unapproved draft" \
  'publish the spec to the project issue tracker as an unapproved draft' "$to_spec"
assert_contains "matt-to-spec reserves agent readiness for the approval marker" \
  'may apply `ready-for-agent` only after the approved-body marker exists' "$to_spec"
assert_not_contains "matt-to-spec has no premature ready-for-agent instruction" \
  'Apply the `ready-for-agent` triage label - no need for additional triage.' "$to_spec"
assert_contains "matt-wayfinder requires a server-ordered unique claim identifier" \
  'Wayfinder-Claim: <session-unique-id>' "$wayfinder"
assert_not_contains "matt-wayfinder does not make the assignee the claim" \
  'That assignee _is_ the claim' "$wayfinder"
assert_not_contains "matt-wayfinder does not claim through assignment alone" \
  'assign it to yourself before any work' "$wayfinder"
assert_contains "matt-wayfinder defines the losing-session action" \
  'A losing session releases its own claim, skips the ticket, and refreshes the frontier.' "$wayfinder"
assert_contains "matt-wayfinder requires a marked non-empty resolution" \
  'whose first line is `Wayfinder-Resolution:`' "$wayfinder"
assert_contains "matt-to-spec links a Wayfinder map in Product sources" \
  'If Wayfinder produced the source material, include `## Product sources` and link its map.' "$to_spec"
if [ -x "$hash_helper" ]; then
  pass "the shared body hash helper is executable"
else
  fail "the shared body hash helper is executable"
fi
for contract in \
  'GitHub owns product decisions' \
  'Beads owns only the sandbox' \
  'root coordinator is the sole Beads writer' \
  'never run `bd`' \
  'Do not reinitialize' \
  '`bd bootstrap`, `bd dolt pull`, and `bd prime`' \
  'Never use JSONL' \
  'Never force'; do
  assert_contains "the Beads README carries '$contract'" "$contract" "$beads_readme"
done
if LC_ALL=C grep -q '[^ -~[:space:]]' "$beads_readme"; then
  fail "the Beads README stays ASCII"
else
  pass "the Beads README stays ASCII"
fi
assert_contains "matt-prototype stops before production code" \
  'Do not fold or lift untested prototype code into production.' "$prototype/SKILL.md"
assert_contains "the UI prototype stops before production code" \
  'Do not fold or promote a variant into production.' "$prototype/UI.md"
assert_contains "the logic prototype stops before production code" \
  'Do not lift its reducer, machine, functions, or shell into production.' "$prototype/LOGIC.md"

for vendored in "$HARNESS_SOURCE"/skills/matt-*/SKILL.md; do
  name=$(basename -- "$(dirname -- "$vendored")")
  if [ "$name" = matt-handoff ]; then
    assert_not_contains "matt-handoff remains a general compaction utility" \
      'It must not choose, start, or route implementation.' "$vendored"
  else
    assert_contains "$name cannot route implementation" \
      'It must not choose, start, or route implementation.' "$vendored"
  fi
done

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
