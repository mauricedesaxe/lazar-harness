# Beads in lazar-harness

GitHub owns product decisions, Wayfinder maps, and approved specs. Beads owns only the sandbox
execution graph that Orchestrate derives from those sources.

The root coordinator is the sole Beads writer. Children receive bead IDs in immutable briefs and
never run `bd`. Do not reinitialize an existing Beads store.

Before work, adopt the remote state with `bd bootstrap`, `bd dolt pull`, and `bd prime`. Commit and
push Dolt after each durable transition. Never use JSONL as a sync mechanism. Never force a Dolt
push.
