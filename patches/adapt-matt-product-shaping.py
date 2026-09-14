#!/usr/bin/env python3
from pathlib import Path
import sys

NON_SHAPING_SKILLS = {"handoff"}

STAGE_BOUNDARY = """## Stage boundary

This Matt leaf performs only its named stage. It must not choose, start, or route implementation. Product shaping passes product decisions to P stack.

"""

REPLACEMENTS = {
    "wayfinder/SKILL.md": [
        (
            "**Where the map, its child tickets, blocking, and frontier queries physically live is tracker-specific.** The issue tracker should have been provided to you. If not, tell the user to run `/setup-matt-pocock-skills`. Consult the tracker doc's \"Wayfinding operations\" section for how _this_ repo expresses them. If no tracker has been provided, default to the local-markdown tracker.",
            "**Where the map, its child tickets, blocking, claims, and frontier queries physically live is tracker-specific.** Resolve the issue tracker through the **Tracker resolution** contract in `CLAUDE.md`. Follow its order: repo config, machine-local note, inference, then ask once and save the answer. Consult the resolved tracker contract's \"Wayfinding operations\" section for this repo's commands and conventions.",
        ),
        (
            "A session **claims** a ticket by assigning it to the dev driving the map, **first**, before any work, so concurrent sessions skip it. That assignee _is_ the claim: an open, unassigned ticket is unclaimed.",
            "A session **claims** a ticket through the resolved tracker contract, **first**, before any work. The claim needs a server-ordered unique identifier that concurrent sessions can compare. An assignee may identify the worker, but assignment alone is not an exclusive claim. The tracker contract defines the claim operation and how a losing session skips the ticket.",
        ),
        (
            "2. Choose the ticket. If the user named one, use it. Otherwise take the first frontier ticket in order. **Claim it**: assign it to yourself before any work.",
            "2. Choose the ticket. If the user named one, use it. Otherwise take the first frontier ticket in order. **Claim it** through the resolved tracker contract before any work, and continue only if this session owns the server-ordered unique claim identifier.",
        ),
    ],
    "to-spec/SKILL.md": [
        (
            "The issue tracker and triage label vocabulary should have been provided to you. If not, tell the user to run `/setup-matt-pocock-skills`.",
            "Resolve the issue tracker through the **Tracker resolution** contract in `CLAUDE.md`. Follow its order: repo config, machine-local note, inference, then ask once and save the answer. Use the resolved tracker's commands, conventions, and triage labels.",
        ),
        (
            "3. Write the spec using the template below, then publish it to the project issue tracker. Apply the `ready-for-agent` triage label - no need for additional triage.",
            "3. Write the spec using the template below, then publish it to the project issue tracker as an unapproved draft. Apply `ready-for-human` when the tracker supports that label, or publish it without a triage label. Do not apply `ready-for-agent`. Product shaping owns approval and may apply `ready-for-agent` only after the approved-body marker exists.",
        ),
    ],
    "to-tickets/SKILL.md": [
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


def add_stage_boundary(text: str, path: Path) -> str:
    if STAGE_BOUNDARY.strip() in text:
        raise ValueError(f"{path}: stage boundary already exists upstream")
    lines = text.splitlines(keepends=True)
    delimiters = [index for index, line in enumerate(lines) if line.rstrip("\r\n") == "---"]
    if len(delimiters) < 2:
        raise ValueError(f"{path}: frontmatter has no closing delimiter")
    index = delimiters[1] + 1
    if index >= len(lines) or lines[index].strip():
        raise ValueError(f"{path}: expected a blank line after frontmatter")
    lines.insert(index + 1, STAGE_BOUNDARY)
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
            adapted[path] = add_stage_boundary(text, path)
    except (OSError, ValueError) as error:
        print(f"Matt adaptation rejected upstream drift: {error}", file=sys.stderr)
        return 1

    for path, text in adapted.items():
        path.write_text(text)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
