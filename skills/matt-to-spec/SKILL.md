---
name: matt-to-spec
description: "Internal Product shaping stage that turns settled product decisions into a tracker spec."
---

## Stage boundary

This Matt leaf performs only the to-spec stage. It must not choose, start, or route implementation. Natural-language and explicit `/matt-to-spec` requests enter the pstack-poteto-mode Product shaping playbook. If this leaf was selected directly, invoke that playbook first. Resume at the to-spec stage only when that playbook's installed-surface rules permit it. Product-decision prototypes before approval use Matt. Implementation experiments after approval use P stack.

This skill takes the current conversation context and codebase understanding and produces a spec. Do NOT interview the user; just synthesize what you already know.

Resolve the issue tracker through the **Tracker resolution** contract in `CLAUDE.md`. Follow its order: repo config, machine-local note, inference, then ask once and save the answer. Use the resolved tracker's commands, conventions, and triage labels.

## Process

1. Explore the repo to understand the current state of the codebase, if you haven't already. Use the project's domain glossary vocabulary throughout the spec, and respect any ADRs in the area you're touching.

2. Sketch out the seams at which you're going to test the feature. Existing seams should be preferred to new ones. Use the highest seam possible. If new seams are needed, propose them at the highest point you can. The fewer seams across the codebase, the better - the ideal number is one.

Check with the user that these seams match their expectations.

3. Write the spec using the template below. If Wayfinder produced the source material, include `## Product sources`. Link its map and every decision issue represented in the spec. Copy every settled requirement into the spec body because source links are provenance only. This exact list becomes authoritative when Product shaping hashes the approved body. Omit that section for a bounded spec without a map. Then publish the spec to the project issue tracker as an unapproved draft. Apply `ready-for-human` when the tracker supports that label, or publish it without a triage label. Do not apply `ready-for-agent`. Product shaping owns approval and readiness.

<spec-template>

## Product sources

For a Wayfinder spec, link the product-decision map and every decision issue represented in the spec. This exact list becomes authoritative when the approved spec body is hashed. Treat these links as provenance, not as requirement text. Omit this section when no map exists.

## Problem Statement

The problem that the user is facing, from the user's perspective.

## Solution

The solution to the problem, from the user's perspective.

## User Stories

A LONG, numbered list of user stories. Each user story should be in the format of:

1. As an <actor>, I want a <feature>, so that <benefit>

<user-story-example>
1. As a mobile bank customer, I want to see balance on my accounts, so that I can make better informed decisions about my spending
</user-story-example>

This list of user stories should be extremely extensive and cover all aspects of the feature.

## Implementation Decisions

A list of implementation decisions that were made. This can include:

- The modules that will be built/modified
- The interfaces of those modules that will be modified
- Technical clarifications from the developer
- Architectural decisions
- Schema changes
- API contracts
- Specific interactions

Do NOT include specific file paths or code snippets. They may end up being outdated very quickly.

Exception: if a prototype produced a snippet that encodes a decision more precisely than prose can (state machine, reducer, schema, type shape), inline it within the relevant decision and note briefly that it came from a prototype. Trim to the decision-rich parts, not a working demo, just the important bits.

## Testing Decisions

A list of testing decisions that were made. Include:

- A description of what makes a good test (only test external behavior, not implementation details)
- Which modules will be tested
- Prior art for the tests (i.e. similar types of tests in the codebase)

## Out of Scope

A description of the things that are out of scope for this spec.

## Further Notes

Any further notes about the feature.

</spec-template>
