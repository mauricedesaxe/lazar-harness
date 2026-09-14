### Product shaping

**You own the route. The human owns product decisions. Matt shapes the product. P stack implements the approved product.**

Use this playbook for these requests:

- "shape this idea"
- "grill this"
- "grill with docs"
- "wayfind this"
- "turn this into a spec"
- "split this into tickets"

An explicit request for one Matt stage can start at that stage. `pstack-poteto-mode` remains the router.

1. **Choose the entry stage.** A bounded vague idea in a repository starts with **matt-grill-with-docs**. A stateless discussion without a repository starts with **matt-grill-me**. An explicit stage request can skip earlier stages when its required context exists. Keep the grill human-led. Ask the current decision frontier, then wait for answers. Do not implement until the human confirms shared understanding.
2. **Gather the required evidence.** Call **matt-research** when a decision needs primary-source facts from outside the working directory. Call **matt-prototype** when the human needs a rough product artifact to judge behavior, a state model, or a UI. Call **matt-to-questionnaire** when another stakeholder holds required facts or decisions. Matt prototypes are throwaway product-shaping evidence. The P stack Prototype playbook remains available for implementation-time experiments.
3. **Use a map only for large foggy work.** Route a large, foggy effort to **matt-wayfinder** only when its decisions cannot fit one session. Preserve its map, decision-ticket, frontier, and fog semantics. Continue the grill without a map when the decisions fit one session. When the map clears, require **matt-to-spec** before implementation.
4. **Resolve the tracker before tracker-aware Matt work.** The tracker-aware leaves are **matt-wayfinder**, **matt-to-spec**, and **matt-to-tickets**. Resolve the tracker through `CLAUDE.md`'s Tracker resolution order before each leaf. Give the leaf the resolved tracker, its commands, and its conventions. Never route to `setup-matt-pocock-skills`.
5. **Publish the spec and obtain product approval.** Call **matt-to-spec** after shared understanding or a clear Wayfinder map. Publish the spec on the resolved tracker. A GitHub-backed repository gets a GitHub spec issue. Obtain the human's approval for the published spec. Do not start implementation before that approval.
6. **Choose one implementation graph after approval.** For a standing Beads-backed sandbox program, stop Matt at the approved spec. Route the program to **Orchestrate**. Do not call **matt-to-tickets**. Do not create a duplicate GitHub implementation dependency graph. For non-Beads multi-ticket work, call **matt-to-tickets** after tracker resolution. Let the leaf draft the granularity and blocking edges for human approval. After approval, publish those tickets as the sole implementation graph.
7. **Hand implementation to P stack.** Route one bounded feature to **Feature**. Route a bounded multi-issue run to **Multi-phase or multi-PR plan**, **Autopilot-full**, or **Autopilot-stack**. Choose between them from the user's autonomy and landing request. Route a standing sandbox program to **Orchestrate**. After final product approval, start the selected P stack workflow without another start gate. Return to product shaping only for a genuinely unresolved product decision.

**Reply.** Name the current shaping stage, settled decisions, open decisions, and the approved spec link. Name the selected P stack route and the sole implementation graph when either exists.
