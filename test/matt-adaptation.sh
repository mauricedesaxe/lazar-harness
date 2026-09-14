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
  mkdir -p "$root"/{handoff,wayfinder,to-spec,to-tickets,prototype,domain-modeling}
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
description: Plan a map.
---

The destination varies per effort, and naming it is the first act of charting: it shapes every ticket. It might be a spec to hand off and iterate on, a decision to lock before planning starts, or a change made in place like a data-structure migration. The map is domain-agnostic: engineering work, course content, whatever fits the shape.

Wayfinder is **planning** by default: each ticket resolves a decision, and the map is done when the way is clear, with nothing left to decide before someone goes and does the thing. The pull to just do the work is usually the signal you've reached the edge of the map and it's time to hand off. An effort can override this in its **Notes**, carrying execution into the map itself, but absent that, produce decisions, not deliverables.

**Where the map, its child tickets, blocking, and frontier queries physically live is tracker-specific.** The issue tracker should have been provided to you. If not, tell the user to run `/setup-matt-pocock-skills`. Consult the tracker doc's "Wayfinding operations" section for how _this_ repo expresses them. If no tracker has been provided, default to the local-markdown tracker.

A session **claims** a ticket by assigning it to the dev driving the map, **first**, before any work, so concurrent sessions skip it. That assignee _is_ the claim: an open, unassigned ticket is unclaimed.

2. Choose the ticket. If the user named one, use it. Otherwise take the first frontier ticket in order. **Claim it**: assign it to yourself before any work.

4. Record the resolution: post the answer as a **resolution comment**, **close** the issue, and **append a context pointer** to the map's Decisions-so-far.
EOF
  cat >"$root/to-spec/SKILL.md" <<'EOF'
---
name: to-spec
description: Write a spec.
---

The issue tracker and triage label vocabulary should have been provided to you. If not, tell the user to run `/setup-matt-pocock-skills`.

3. Write the spec using the template below, then publish it to the project issue tracker. Apply the `ready-for-agent` triage label - no need for additional triage.

<spec-template>

## Problem Statement
EOF
  cat >"$root/to-tickets/SKILL.md" <<'EOF'
---
name: to-tickets
description: Write tickets.
---

Break a plan, spec, or conversation into a set of **tickets**: tracer-bullet vertical slices, each declaring the tickets that **block** it.

The issue tracker and triage label vocabulary should have been provided to you. If not, tell the user to run `/setup-matt-pocock-skills`.

Work from whatever is already in the conversation context. If the user passes a reference (a spec path, an issue number or URL) as an argument, fetch it and read its full body and comments.

Publish the approved tickets. **How** depends on the tracker `/setup-matt-pocock-skills` configured; the tickets are the same either way, only the shape of the blocking edges changes:
EOF
  cat >"$root/prototype/SKILL.md" <<'EOF'
---
name: prototype
description: Build a prototype.
---

6. **Capture it when done.** Fold any validated decision into the real code, then capture the prototype itself as a **primary source**: commit it to a throwaway branch, out of main, and leave a context pointer to that branch on the implementation issue. Capture the answer too (the verdict and the question it settled) in the issue or a commit. The main branch keeps only the validated decision.
EOF
  cat >"$root/domain-modeling/SKILL.md" <<'EOF'
---
name: domain-modeling
description: Model a domain.
---

### Offer ADRs sparingly

Only offer to create an ADR when all three are true:

1. **Hard to reverse**: the cost of changing your mind later is meaningful
2. **Surprising without context**: a future reader will wonder "why did they do it this way?"
3. **The result of a real trade-off**: there were genuine alternatives and you picked one for specific reasons

If any of the three is missing, skip the ADR. Use the format in [ADR-FORMAT.md](./ADR-FORMAT.md).
EOF
  cat >"$root/domain-modeling/ADR-FORMAT.md" <<'EOF'
## When to offer an ADR

All three of these must be true:

1. **Hard to reverse**: the cost of changing your mind later is meaningful
2. **Surprising without context**: a future reader will look at the code and wonder "why on earth did they do it this way?"
3. **The result of a real trade-off**: there were genuine alternatives and you picked one for specific reasons

If a decision is easy to reverse, skip it: you'll just reverse it. If it's not surprising, nobody will wonder why. If there was no real alternative, there's nothing to record beyond "we did the obvious thing."

### What qualifies

- **Architectural shape.** "We're using a monorepo." "The write model is event-sourced, the read model is projected into Postgres."
- **Integration patterns between contexts.** "Ordering and Billing communicate via domain events, not synchronous HTTP."
- **Technology choices that carry lock-in.** Database, message bus, auth provider, deployment target. Not every library: just the ones that would take a quarter to swap out.
- **Boundary and scope decisions.** "Customer data is owned by the Customer context; other contexts reference it by ID only." The explicit no-s are as valuable as the yes-s.
- **Deliberate deviations from the obvious path.** "We're using manual SQL instead of an ORM because X." Anything where a reasonable reader would assume the opposite. These stop the next engineer from "fixing" something that was deliberate.
- **Constraints not visible in the code.** "We can't use AWS because of compliance requirements." "Response times must be under 200ms because of the partner API contract."
- **Rejected alternatives when the rejection is non-obvious.** If you considered GraphQL and picked REST for subtle reasons, record it; otherwise someone will suggest GraphQL again in six months.
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
UPSTREAM_SKILLS=(handoff wayfinder to-spec to-tickets prototype domain-modeling)

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
  grep -qF 'Product shaping owns approval and readiness' "$work/pristine/to-spec/SKILL.md" &&
  ! grep -qF 'may apply `ready-for-agent`' "$work/pristine/to-spec/SKILL.md"; then
  pass "the adapted spec delegates all readiness decisions to Product shaping"
else
  fail "the adapted spec delegates all readiness decisions to Product shaping"
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
  grep -qF 'Link its map and every decision issue represented in the spec.' "$work/pristine/to-spec/SKILL.md" &&
  grep -qF 'Treat these links as provenance, not as requirement text.' "$work/pristine/to-spec/SKILL.md"; then
  pass "the adapted spec makes its exact Wayfinder source list authoritative"
else
  fail "the adapted spec makes its exact Wayfinder source list authoritative"
fi

if grep -qF 'Do not fold or lift untested prototype code into production' "$work/pristine/prototype/SKILL.md" &&
  grep -qF 'Do not lift its reducer, machine, functions, or shell into production' "$work/pristine/prototype/LOGIC.md" &&
  grep -qF 'Do not fold or promote a variant into production' "$work/pristine/prototype/UI.md"; then
  pass "the adapted prototype stops after the answer and pointer"
else
  fail "the adapted prototype stops after the answer and pointer"
fi

if grep -qF 'Keep product decisions in the tracker spec' "$work/pristine/domain-modeling/SKILL.md" &&
  grep -qF "Follow the repository's ADR policy." "$work/pristine/domain-modeling/SKILL.md" &&
  grep -qF 'Record settled product decisions in the tracker spec instead.' "$work/pristine/domain-modeling/ADR-FORMAT.md"; then
  pass "the adapted domain model follows the harness ADR policy"
else
  fail "the adapted domain model follows the harness ADR policy"
fi

for skill in wayfinder to-spec to-tickets prototype; do
  if grep -qF 'It must not choose, start, or route implementation.' "$work/pristine/$skill/SKILL.md" &&
    grep -qF "Natural-language and explicit \`/$skill\` requests enter the pstack-poteto-mode Product shaping playbook." "$work/pristine/$skill/SKILL.md" &&
    grep -qF "Resume at the $skill stage only when" "$work/pristine/$skill/SKILL.md" &&
    grep -qF 'description: "Internal ' "$work/pristine/$skill/SKILL.md"; then
    pass "$skill routes direct selection through its internal Product shaping stage"
  else
    fail "$skill routes direct selection through its internal Product shaping stage"
  fi
done

write_fixture "$work/pristine-second"
if apply_matt_adaptation "$work/pristine-second" >/dev/null 2>&1 &&
  diff -qr "$work/pristine" "$work/pristine-second" >/dev/null; then
  pass "two Matt adaptations from the same pristine input are byte-stable"
else
  fail "two Matt adaptations from the same pristine input are byte-stable"
fi

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
