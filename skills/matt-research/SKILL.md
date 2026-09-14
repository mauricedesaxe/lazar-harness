---
name: matt-research
description: "Internal Product shaping stage that gathers primary-source facts for a product decision."
---

## Stage boundary

This Matt leaf performs only the research stage. It must not choose, start, or route implementation. Natural-language and explicit `/matt-research` requests enter the pstack-poteto-mode Product shaping playbook. If this leaf was selected directly, invoke that playbook first. Resume at the research stage only when that playbook's installed-surface rules permit it. Product-decision prototypes before approval use Matt. Implementation experiments after approval use P stack.

Spin up a **background agent** to do the research, so you keep working while it reads.

Its job:

1. Investigate the question against **primary sources** (official docs, source code, specs, first-party APIs), not a secondary write-up of them. Follow every claim back to the source that owns it.
2. Write the findings to a single Markdown file, citing each claim's source.
3. Save it where the repo already keeps such notes; match the existing convention, and if there is none, put it somewhere sensible and say where.
