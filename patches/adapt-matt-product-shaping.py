#!/usr/bin/env python3
from pathlib import Path
import sys

NON_SHAPING_SKILLS = {"handoff"}

STAGE_DESCRIPTIONS = {
    "grilling": "Internal Product shaping stage that interviews the user until the product decisions are clear.",
    "grill-me": "Internal Product shaping entry stage for a product interview without repository context.",
    "grill-with-docs": "Internal Product shaping entry stage for a product interview with repository context.",
    "domain-modeling": "Internal Product shaping stage that sharpens product terms and records durable domain decisions.",
    "wayfinder": "Internal Product shaping stage that resolves a large product question through a map of decision issues.",
    "to-spec": "Internal Product shaping stage that turns settled product decisions into a tracker spec.",
    "to-tickets": "Internal Product shaping stage that turns an approved spec into delivery tickets where the installed surface permits them.",
    "research": "Internal Product shaping stage that gathers primary-source facts for a product decision.",
    "prototype": "Internal Product shaping stage that builds throwaway evidence for a product decision.",
    "to-questionnaire": "Internal Product shaping stage that asks a stakeholder for required product facts or decisions.",
}

STAGE_BOUNDARY = """## Stage boundary

This Matt leaf performs only the {stage} stage. It must not choose, start, or route implementation. Natural-language and explicit `/{stage}` requests enter the pstack-poteto-mode Product shaping playbook. If this leaf was selected directly, invoke that playbook first. Resume at the {stage} stage only when that playbook's installed-surface rules permit it. Product-decision prototypes before approval use Matt. Implementation experiments after approval use P stack.

"""

REPLACEMENTS = {
    "domain-modeling/SKILL.md": [
        (
            "### Offer ADRs sparingly\n\nOnly offer to create an ADR when all three are true:\n\n1. **Hard to reverse**: the cost of changing your mind later is meaningful\n2. **Surprising without context**: a future reader will wonder \"why did they do it this way?\"\n3. **The result of a real trade-off**: there were genuine alternatives and you picked one for specific reasons\n\nIf any of the three is missing, skip the ADR. Use the format in [ADR-FORMAT.md](./ADR-FORMAT.md).",
            "### Keep product decisions in the tracker spec\n\nRecord settled product decisions in the tracker spec. Follow the repository's ADR policy. Use an ADR only for temporary discussion or for a lasting decision that code cannot express, such as a vendor, process, SLA, or contract. Archive or remove a temporary ADR after the chosen direction lands. Use [ADR-FORMAT.md](./ADR-FORMAT.md) only when an ADR clears that bar.",
        ),
    ],
    "domain-modeling/ADR-FORMAT.md": [
        (
            "## When to offer an ADR\n\nAll three of these must be true:\n\n1. **Hard to reverse**: the cost of changing your mind later is meaningful\n2. **Surprising without context**: a future reader will look at the code and wonder \"why on earth did they do it this way?\"\n3. **The result of a real trade-off**: there were genuine alternatives and you picked one for specific reasons\n\nIf a decision is easy to reverse, skip it: you'll just reverse it. If it's not surprising, nobody will wonder why. If there was no real alternative, there's nothing to record beyond \"we did the obvious thing.\"\n\n### What qualifies\n\n- **Architectural shape.** \"We're using a monorepo.\" \"The write model is event-sourced, the read model is projected into Postgres.\"\n- **Integration patterns between contexts.** \"Ordering and Billing communicate via domain events, not synchronous HTTP.\"\n- **Technology choices that carry lock-in.** Database, message bus, auth provider, deployment target. Not every library: just the ones that would take a quarter to swap out.\n- **Boundary and scope decisions.** \"Customer data is owned by the Customer context; other contexts reference it by ID only.\" The explicit no-s are as valuable as the yes-s.\n- **Deliberate deviations from the obvious path.** \"We're using manual SQL instead of an ORM because X.\" Anything where a reasonable reader would assume the opposite. These stop the next engineer from \"fixing\" something that was deliberate.\n- **Constraints not visible in the code.** \"We can't use AWS because of compliance requirements.\" \"Response times must be under 200ms because of the partner API contract.\"\n- **Rejected alternatives when the rejection is non-obvious.** If you considered GraphQL and picked REST for subtle reasons, record it; otherwise someone will suggest GraphQL again in six months.",
            "## When to offer an ADR\n\nFollow the repository's ADR policy. Use this format for a temporary discussion artifact or a lasting decision that code cannot express. Lasting examples include a vendor choice, a process change, an SLA, or a contractual constraint. Archive or remove a temporary ADR after the chosen direction lands. Record settled product decisions in the tracker spec instead.",
        ),
    ],
    "wayfinder/SKILL.md": [
        (
            "The destination varies per effort, and naming it is the first act of charting: it shapes every ticket. It might be a spec to hand off and iterate on, a decision to lock before planning starts, or a change made in place like a data-structure migration. The map is domain-agnostic: engineering work, course content, whatever fits the shape.",
            "The destination varies per effort, and naming it is the first act of charting: it shapes every ticket. It might be a spec to hand off and iterate on or a decision to lock before planning starts. The map is domain-agnostic: engineering work, course content, whatever fits the shape.",
        ),
        (
            "Wayfinder is **planning** by default: each ticket resolves a decision, and the map is done when the way is clear, with nothing left to decide before someone goes and does the thing. The pull to just do the work is usually the signal you've reached the edge of the map and it's time to hand off. An effort can override this in its **Notes**, carrying execution into the map itself, but absent that, produce decisions, not deliverables.",
            "Wayfinder resolves decisions only. Each ticket resolves one decision, and the map is done when the way is clear. The pull to do the work means that the map has reached its handoff point. Notes cannot carry implementation into the map. Produce decisions, not deliverables.",
        ),
        (
            "**Where the map, its child tickets, blocking, and frontier queries physically live is tracker-specific.** The issue tracker should have been provided to you. If not, tell the user to run `/setup-matt-pocock-skills`. Consult the tracker doc's \"Wayfinding operations\" section for how _this_ repo expresses them. If no tracker has been provided, default to the local-markdown tracker.",
            "**Where the map, its child tickets, blocking, claims, and frontier queries physically live is tracker-specific.** Resolve the issue tracker through the **Tracker resolution** contract in `CLAUDE.md`. Follow its order: repo config, machine-local note, inference, then ask once and save the answer. Consult the resolved tracker contract's \"Wayfinding operations\" section for this repo's commands and conventions.",
        ),
        (
            "A session **claims** a ticket by assigning it to the dev driving the map, **first**, before any work, so concurrent sessions skip it. That assignee _is_ the claim: an open, unassigned ticket is unclaimed.",
            "A session **claims** a ticket through the resolved tracker contract, **first**, before any work. Post `Wayfinder-Claim: <session-unique-id>`, then fetch every exact claim comment in server order. Continue only when this session owns the earliest valid claim. A released claim is not valid. An assignee may identify the worker, but assignment alone is not an exclusive claim. A losing session releases its own claim, skips the ticket, and refreshes the frontier.",
        ),
        (
            "2. Choose the ticket. If the user named one, use it. Otherwise take the first frontier ticket in order. **Claim it**: assign it to yourself before any work.",
            "2. Choose the ticket. If the user named one, use it. Otherwise take the first frontier ticket in order. **Claim it** through the resolved tracker contract before any work. Continue only if this session owns the earliest valid `Wayfinder-Claim: <id>` comment in server order. If this session loses, release its claim, skip the ticket, and refresh the frontier.",
        ),
        (
            "4. Record the resolution: post the answer as a **resolution comment**, **close** the issue, and **append a context pointer** to the map's Decisions-so-far.",
            "4. Record the resolution: post a non-empty answer whose first line is `Wayfinder-Resolution:`, close the issue with reason `completed`, and append a context pointer to the map's Decisions-so-far.",
        ),
    ],
    "to-spec/SKILL.md": [
        (
            "<spec-template>\n\n## Problem Statement",
            "<spec-template>\n\n## Product sources\n\nFor a Wayfinder spec, link the product-decision map and every decision issue represented in the spec. This exact list becomes authoritative when the approved spec body is hashed. Treat these links as provenance, not as requirement text. Omit this section when no map exists.\n\n## Problem Statement",
        ),
        (
            "The issue tracker and triage label vocabulary should have been provided to you. If not, tell the user to run `/setup-matt-pocock-skills`.",
            "Resolve the issue tracker through the **Tracker resolution** contract in `CLAUDE.md`. Follow its order: repo config, machine-local note, inference, then ask once and save the answer. Use the resolved tracker's commands, conventions, and triage labels.",
        ),
        (
            "3. Write the spec using the template below, then publish it to the project issue tracker. Apply the `ready-for-agent` triage label - no need for additional triage.",
            "3. Write the spec using the template below. If Wayfinder produced the source material, include `## Product sources`. Link its map and every decision issue represented in the spec. Copy every settled requirement into the spec body because source links are provenance only. This exact list becomes authoritative when Product shaping hashes the approved body. Omit that section for a bounded spec without a map. Then publish the spec to the project issue tracker as an unapproved draft. Apply `ready-for-human` when the tracker supports that label, or publish it without a triage label. Do not apply `ready-for-agent`. Product shaping owns approval and readiness.",
        ),
    ],
    "to-tickets/SKILL.md": [
        (
            "Break a plan, spec, or conversation into a set of **tickets**: tracer-bullet vertical slices, each declaring the tickets that **block** it.",
            "Break an approved tracker spec into a set of **tickets**: tracer-bullet vertical slices, each declaring the tickets that **block** it.",
        ),
        (
            "Work from whatever is already in the conversation context. If the user passes a reference (a spec path, an issue number or URL) as an argument, fetch it and read its full body and comments.",
            "Require an approved tracker spec reference. Product shaping must verify its current approval marker before this stage starts. Fetch the spec and read its full body and comments. Stop if the reference or verified approval is missing.",
        ),
        (
            "The issue tracker and triage label vocabulary should have been provided to you. If not, tell the user to run `/setup-matt-pocock-skills`.",
            "Resolve the issue tracker through the **Tracker resolution** contract in `CLAUDE.md`. Follow its order: repo config, machine-local note, inference, then ask once and save the answer. Use the resolved tracker's commands, conventions, and triage labels.",
        ),
        (
            "Publish the approved tickets. **How** depends on the tracker `/setup-matt-pocock-skills` configured; the tickets are the same either way, only the shape of the blocking edges changes:",
            "Publish the approved tickets. **How** depends on the resolved tracker contract. The tickets are the same either way; only the shape of the blocking edges changes:",
        ),
    ],
    "prototype/SKILL.md": [
        (
            "6. **Capture it when done.** Fold any validated decision into the real code, then capture the prototype itself as a **primary source**: commit it to a throwaway branch, out of main, and leave a context pointer to that branch on the implementation issue. Capture the answer too (the verdict and the question it settled) in the issue or a commit. The main branch keeps only the validated decision.",
            "6. **Capture it when done.** Capture the answer, including the verdict and the question it settled. Capture the prototype as a **primary source** on a throwaway branch, out of main, and leave its context pointer on the shaping issue. Stop there. Do not fold or lift untested prototype code into production. After spec approval, Product shaping passes the decision and prototype pointer to P stack. P stack rewrites and tests the production code.",
        ),
    ],
    "prototype/UI.md": [
        (
            "Once a variant has won, capture the answer (which variant and why), then capture the prototype the way the [SKILL](SKILL.md) describes. Fold the winner into the real code and move the rest onto the throwaway branch, not into main:\n\n- **Sub-shape A**: fold the winner into the existing page; drop the losing variants and the switcher from main.\n- **Sub-shape B**: promote the winning variant to a real route; drop the throwaway route and the switcher from main.\n\nThe full set of variants is the primary source, so it lands on the throwaway branch, not the bin, since variant components and the switcher left in the main branch rot fast and confuse the next reader.",
            "Once a variant has won, capture the answer, including which variant won and why. Capture the full prototype on the throwaway branch as the [SKILL](SKILL.md) describes. Leave its context pointer on the shaping issue, then stop. Do not fold or promote a variant into production. After spec approval, Product shaping passes the decision and prototype pointer to P stack. P stack rewrites the chosen design against the production constraints and adds tests.",
        ),
        (
            "- **Promoting the prototype directly to production.** The variant code was written under prototype constraints (no tests, minimal error handling). Rewrite it properly when you fold it in.",
            "- **Promoting the prototype directly to production.** The variant code was written under prototype constraints (no tests, minimal error handling). P stack rewrites and tests the chosen design after spec approval.",
        ),
    ],
    "prototype/LOGIC.md": [
        (
            "Put the actual logic (the bit that's answering the question) in a single `<script>` block written as a small, pure module that could be lifted out and dropped into the real codebase later. The page around it is throwaway; this module isn't.",
            "Put the actual logic (the bit that's answering the question) in a single `<script>` block written as a small, pure module. Keep the page around it thin so the prototype isolates the decision under test. The full prototype remains throwaway evidence.",
        ),
        (
            "Pick whichever shape best fits the question being asked, *not* whichever is easiest to wire to a page. Keep it pure: no DOM, no `document`, no button handlers reaching inside it. The page calls into it; nothing flows the other direction. This is what makes the prototype useful past its own lifetime: once the question's answered, the validated reducer / machine / function set lifts into the real module on its own.",
            "Pick whichever shape best fits the question being asked, *not* whichever is easiest to wire to a page. Keep it pure: no DOM, no `document`, no button handlers reaching inside it. The page calls into it; nothing flows the other direction. This keeps the decision clear when P stack later rewrites and tests the production module.",
        ),
        (
            "Once the prototype has answered its question, capture the answer, then capture the prototype the way the [SKILL](SKILL.md) describes. The logic-specific mapping: the validated reducer / machine / function set lifts into the real module (the decision, absorbed); the HTML shell rides along to the throwaway branch that keeps the prototype as a primary source, and being one self-contained file, it stays trivially re-runnable there.",
            "Once the prototype has answered its question, capture the answer and the prototype pointer as the [SKILL](SKILL.md) describes, then stop. Keep the self-contained HTML file on the throwaway branch as a re-runnable primary source. Do not lift its reducer, machine, functions, or shell into production. After spec approval, Product shaping passes the decision and pointer to P stack. P stack rewrites and tests the production logic.",
        ),
        (
            "- **Don't blur the logic and the page together.** If the pure module references the DOM, `document`, or button handlers, it's no longer liftable. Keep the page as a thin shell over a pure module.",
            "- **Don't blur the logic and the page together.** If the pure module references the DOM, `document`, or button handlers, the prototype no longer isolates the decision. Keep the page as a thin shell over a pure module.",
        ),
        (
            "- **Don't ship the HTML shell into production.** The page is optimised for being clicked through by hand. The logic module behind it is the bit worth keeping.",
            "- **Don't ship prototype code into production.** The page and logic module were written for manual exploration. P stack rewrites and tests the production logic after spec approval.",
        ),
    ],
}


def replace_exact(text: str, old: str, new: str, path: Path) -> str:
    count = text.count(old)
    if count != 1:
        raise ValueError(f"{path}: expected one adapted region, found {count}")
    return text.replace(old, new)


def adapt_description(text: str, skill_name: str, path: Path) -> str:
    lines = text.splitlines(keepends=True)
    delimiters = [index for index, line in enumerate(lines) if line.rstrip("\r\n") == "---"]
    if len(delimiters) < 2:
        raise ValueError(f"{path}: frontmatter has no closing delimiter")
    matches = [
        index for index in range(delimiters[0] + 1, delimiters[1])
        if lines[index].startswith("description:")
    ]
    if len(matches) != 1:
        raise ValueError(f"{path}: expected one frontmatter description, found {len(matches)}")
    newline = "\r\n" if lines[matches[0]].endswith("\r\n") else "\n"
    lines[matches[0]] = f'description: "{STAGE_DESCRIPTIONS[skill_name]}"{newline}'
    return "".join(lines)


def add_stage_boundary(text: str, skill_name: str, path: Path) -> str:
    stage_boundary = STAGE_BOUNDARY.format(stage=skill_name)
    if "## Stage boundary" in text:
        raise ValueError(f"{path}: stage boundary already exists upstream")
    lines = text.splitlines(keepends=True)
    delimiters = [index for index, line in enumerate(lines) if line.rstrip("\r\n") == "---"]
    if len(delimiters) < 2:
        raise ValueError(f"{path}: frontmatter has no closing delimiter")
    index = delimiters[1] + 1
    if index >= len(lines) or lines[index].strip():
        raise ValueError(f"{path}: expected a blank line after frontmatter")
    lines.insert(index + 1, stage_boundary)
    return "".join(lines)


def main() -> int:
    if len(sys.argv) < 3:
        print("usage: adapt-matt-product-shaping.py <skills-root> <skill>...", file=sys.stderr)
        return 2

    root = Path(sys.argv[1])
    skill_names = sys.argv[2:]
    adapted: dict[Path, str] = {}

    try:
        for relative, replacements in REPLACEMENTS.items():
            path = root / relative
            text = path.read_text()
            for old, new in replacements:
                text = replace_exact(text, old, new, path)
            adapted[path] = text

        for skill_name in skill_names:
            if skill_name in NON_SHAPING_SKILLS:
                continue
            path = root / skill_name / "SKILL.md"
            text = adapted.get(path, path.read_text())
            text = adapt_description(text, skill_name, path)
            adapted[path] = add_stage_boundary(text, skill_name, path)
    except (OSError, ValueError) as error:
        print(f"Matt adaptation rejected upstream drift: {error}", file=sys.stderr)
        return 1

    for path, text in adapted.items():
        path.write_text(text)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
