#!/usr/bin/env bash
set -uo pipefail

HARNESS_SOURCE="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
. "$HARNESS_SOURCE/vendor-skills.sh"
set +e

failures=0
pass() { printf 'ok   %s\n' "$1"; }
fail() { printf 'FAIL %s\n' "$1" >&2; failures=$((failures + 1)); }

write_fixture() {
  local root=$1
  mkdir -p "$root"/{handoff,wayfinder,to-spec,to-tickets,prototype}
  cat >"$root/handoff/SKILL.md" <<'EOF'
---
name: handoff
disable-model-invocation: true
---

Use `/handoff` to compact the current work.
EOF
  cat >"$root/wayfinder/SKILL.md" <<'EOF'
---
name: wayfinder
---

**Where the map, its child tickets, blocking, and frontier queries physically live is tracker-specific.** The issue tracker should have been provided to you. If not, tell the user to run `/setup-matt-pocock-skills`. Consult the tracker doc's "Wayfinding operations" section for how _this_ repo expresses them. If no tracker has been provided, default to the local-markdown tracker.

A session **claims** a ticket by assigning it to the dev driving the map, **first**, before any work, so concurrent sessions skip it. That assignee _is_ the claim: an open, unassigned ticket is unclaimed.

2. Choose the ticket. If the user named one, use it. Otherwise take the first frontier ticket in order. **Claim it**: assign it to yourself before any work.

4. Record the resolution: post the answer as a **resolution comment**, **close** the issue, and **append a context pointer** to the map's Decisions-so-far.
EOF
  cat >"$root/to-spec/SKILL.md" <<'EOF'
---
name: to-spec
---

The issue tracker and triage label vocabulary should have been provided to you. If not, tell the user to run `/setup-matt-pocock-skills`.

3. Write the spec using the template below, then publish it to the project issue tracker. Apply the `ready-for-agent` triage label - no need for additional triage.

<spec-template>

## Problem Statement
EOF
  cat >"$root/to-tickets/SKILL.md" <<'EOF'
---
name: to-tickets
---

The issue tracker and triage label vocabulary should have been provided to you. If not, tell the user to run `/setup-matt-pocock-skills`.

Publish the approved tickets. **How** depends on the tracker `/setup-matt-pocock-skills` configured; the tickets are the same either way, only the shape of the blocking edges changes:
EOF
  cat >"$root/prototype/SKILL.md" <<'EOF'
---
name: prototype
---

6. **Capture it when done.** Fold any validated decision into the real code, then capture the prototype itself as a **primary source**: commit it to a throwaway branch, out of main, and leave a context pointer to that branch on the implementation issue. Capture the answer too (the verdict and the question it settled) in the issue or a commit. The main branch keeps only the validated decision.
EOF
  cat >"$root/prototype/UI.md" <<'EOF'
Once a variant has won, capture the answer (which variant and why), then capture the prototype the way the [SKILL](SKILL.md) describes. Fold the winner into the real code and move the rest onto the throwaway branch, not into main:

- **Sub-shape A**: fold the winner into the existing page; drop the losing variants and the switcher from main.
- **Sub-shape B**: promote the winning variant to a real route; drop the throwaway route and the switcher from main.

The full set of variants is the primary source, so it lands on the throwaway branch, not the bin, since variant components and the switcher left in the main branch rot fast and confuse the next reader.

- **Promoting the prototype directly to production.** The variant code was written under prototype constraints (no tests, minimal error handling). Rewrite it properly when you fold it in.
EOF
  cat >"$root/prototype/LOGIC.md" <<'EOF'
Put the actual logic (the bit that's answering the question) in a single `<script>` block written as a small, pure module that could be lifted out and dropped into the real codebase later. The page around it is throwaway; this module isn't.

Pick whichever shape best fits the question being asked, *not* whichever is easiest to wire to a page. Keep it pure: no DOM, no `document`, no button handlers reaching inside it. The page calls into it; nothing flows the other direction. This is what makes the prototype useful past its own lifetime: once the question's answered, the validated reducer / machine / function set lifts into the real module on its own.

Once the prototype has answered its question, capture the answer, then capture the prototype the way the [SKILL](SKILL.md) describes. The logic-specific mapping: the validated reducer / machine / function set lifts into the real module (the decision, absorbed); the HTML shell rides along to the throwaway branch that keeps the prototype as a primary source, and being one self-contained file, it stays trivially re-runnable there.

- **Don't blur the logic and the page together.** If the pure module references the DOM, `document`, or button handlers, it's no longer liftable. Keep the page as a thin shell over a pure module.

- **Don't ship the HTML shell into production.** The page is optimised for being clicked through by hand. The logic module behind it is the bit worth keeping.
EOF
}

work=$(mktemp -d)
trap 'rm -rf -- "$work"' EXIT
UPSTREAM_SKILLS=(handoff wayfinder to-spec to-tickets prototype)

write_fixture "$work/pristine"
if apply_matt_adaptation "$work/pristine" >/dev/null 2>&1; then
  pass "the Matt adaptation applies to the upstream regions it was written for"
else
  fail "the Matt adaptation applies to the upstream regions it was written for"
fi

if grep -qF '## Stage boundary' "$work/pristine/handoff/SKILL.md"; then
  fail "the Matt adaptation exempts the general handoff utility"
else
  pass "the Matt adaptation exempts the general handoff utility"
fi

if grep -rFq -- '/setup-matt-pocock-skills' "$work/pristine"; then
  fail "the adapted tracker-aware leaves contain no absent setup command"
else
  pass "the adapted tracker-aware leaves contain no absent setup command"
fi

if grep -qF 'publish the spec to the project issue tracker as an unapproved draft' "$work/pristine/to-spec/SKILL.md" &&
  grep -qF 'may apply `ready-for-agent` only after the approved-body marker exists' "$work/pristine/to-spec/SKILL.md"; then
  pass "the adapted spec stays human-ready until approval is recorded"
else
  fail "the adapted spec stays human-ready until approval is recorded"
fi

if grep -qF 'Wayfinder-Claim: <session-unique-id>' "$work/pristine/wayfinder/SKILL.md" &&
  grep -qF 'A losing session releases its own claim' "$work/pristine/wayfinder/SKILL.md" &&
  ! grep -qF 'That assignee _is_ the claim' "$work/pristine/wayfinder/SKILL.md"; then
  pass "the adapted Wayfinder delegates exclusive claim mechanics to the tracker"
else
  fail "the adapted Wayfinder delegates exclusive claim mechanics to the tracker"
fi

if grep -qF 'whose first line is `Wayfinder-Resolution:`' "$work/pristine/wayfinder/SKILL.md"; then
  pass "the adapted Wayfinder records a checkable resolution"
else
  fail "the adapted Wayfinder records a checkable resolution"
fi

if grep -qF '## Product sources' "$work/pristine/to-spec/SKILL.md" &&
  grep -qF 'If Wayfinder produced the source material' "$work/pristine/to-spec/SKILL.md"; then
  pass "the adapted spec links its Wayfinder map"
else
  fail "the adapted spec links its Wayfinder map"
fi

if grep -qF 'Do not fold or lift untested prototype code into production' "$work/pristine/prototype/SKILL.md" &&
  grep -qF 'Do not lift its reducer, machine, functions, or shell into production' "$work/pristine/prototype/LOGIC.md" &&
  grep -qF 'Do not fold or promote a variant into production' "$work/pristine/prototype/UI.md"; then
  pass "the adapted prototype stops after the answer and pointer"
else
  fail "the adapted prototype stops after the answer and pointer"
fi

for skill in wayfinder to-spec to-tickets prototype; do
  if grep -qF 'It must not choose, start, or route implementation.' "$work/pristine/$skill/SKILL.md"; then
    pass "$skill cannot route implementation"
  else
    fail "$skill cannot route implementation"
  fi
done

write_fixture "$work/staged/.claude/skills"
mkdir -p "$work/expected-handoff"
cp "$work/staged/.claude/skills/handoff/SKILL.md" "$work/expected-handoff/SKILL.md"
apply_prefix_to_frontmatter "$work/expected-handoff/SKILL.md" handoff
strip_model_invocation_lock "$work/expected-handoff/SKILL.md"
apply_prefix_to_references "$work/expected-handoff"
if stage_matt_skills "$work/staged" &&
  cmp -s "$work/expected-handoff/SKILL.md" "$work/staged/.claude/skills/handoff/SKILL.md"; then
  pass "matt-handoff receives only the standard prefix and invocation transforms"
else
  fail "matt-handoff receives only the standard prefix and invocation transforms"
fi

write_fixture "$work/drifted"
python3 - "$work/drifted/to-spec/SKILL.md" <<'PY'
from pathlib import Path
import sys
path = Path(sys.argv[1])
path.write_text(path.read_text().replace("Apply the `ready-for-agent` triage label", "Apply a changed triage label"))
PY
cp -R "$work/drifted" "$work/before"
if out=$(apply_matt_adaptation "$work/drifted" 2>&1); then
  fail "upstream drift under an adapted region stops vendoring"
else
  pass "upstream drift under an adapted region stops vendoring"
fi
if [[ "$out" == *"rejected upstream drift"* ]]; then
  pass "the drift failure names the Matt adaptation"
else
  fail "the drift failure names the Matt adaptation"
fi
if diff -qr "$work/before" "$work/drifted" >/dev/null; then
  pass "a rejected adaptation leaves the staged upstream untouched"
else
  fail "a rejected adaptation leaves the staged upstream untouched"
fi

if [ "$failures" -eq 0 ]; then
  printf 'matt-adaptation: all assertions passed\n'
else
  printf 'matt-adaptation: %d assertion(s) failed\n' "$failures" >&2
  exit 1
fi
