# AGENTS.md

Guidance for Codex, OpenCode, and any other agent working in **this** repository.

`CLAUDE.md` here is not this repo's rules file. It's the **payload**: the instructions
`install.sh` writes to `~/.claude/CLAUDE.md` and, under OpenCode's name for the same thing,
`~/.config/opencode/AGENTS.md`. Read it as both, because a harness that doesn't hold in its own
repo doesn't hold anywhere.

It points at the philosophy rather than restating it. The philosophy is installed, never carried
by a repo: `~/.claude/rules/PHILOSOPHY.md` with the packs under `~/.claude/rules/packs/`, or the
same files under `~/.config/opencode/rules/`. This repo also holds their **source**, at
`docs/PHILOSOPHY.md` and `docs/packs/`. Edit the source here; read the installed copy anywhere
else. `§N` numbers are stable IDs, so a citation resolves through the spine's Section index.

What's specific to this repo, and easy to get wrong:

- **`CLAUDE.md` has a line budget**: keep it lean, because adherence drops as it grows. Make budget
  by pointing at the philosophy or at `pstack-poteto-mode`, not by dropping a rule.
- **`surface:local` / `surface:sandbox` blocks are per-environment prose.** The installer keeps the
  pair matching `HARNESS_SURFACE` and drops the rest, so both variants live in the markdown and are
  edited together. A file carrying one surface and not the other stops the install.
- **Most vendored skill directories are never hand-edited.** This covers `skills/matt-*`,
  `skills/lazar-tldraw`, `skills/use-railway`, `skills/plannotator-*`, and
  `skills/visual-explainer`. Edit upstream or `patches/lazar-tldraw.patch`, then re-run
  `./vendor-skills.sh`.
- **`skills/pstack-*` is the exception: a hand-maintained fork.** Its Cursor coupling is translated
  for the harness by hand, so you edit these files in place. `skills-lock.json` pins each one's
  pristine-upstream hash, and `./vendor-skills.sh --check-pstack-drift` reports when upstream has
  moved and the fork needs a manual reconcile. A re-vendor never overwrites them.
- **Never pass `--install` to `install.sh` here.** It honours `$HOME`, `$CLAUDE_CONFIG_DIR` and
  `$XDG_CONFIG_HOME`. With the flag it overwrites the live harness your current shell points at. Exercise it with `bash test/install-smoke.sh`, which scrubs the
  environment first. Without the flag it only reports, so running it bare is safe.

<!-- BEGIN BEADS INTEGRATION v:1 profile:minimal hash:970c3bf2 -->
## Beads Issue Tracker

This project uses **bd (beads)** for issue tracking. Run `bd prime` to see full workflow context and commands.

### Quick Reference

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --claim  # Claim work
bd close <id>         # Complete work
```

### Rules

- Use `bd` for ALL task tracking — do NOT use TodoWrite, TaskCreate, or markdown TODO lists
- Run `bd prime` for detailed command reference and session close protocol
- Use `bd remember` for persistent knowledge — do NOT use MEMORY.md files

**Architecture in one line:** issues live in a local Dolt DB; sync uses `refs/dolt/data` on your git remote; `.beads/issues.jsonl` is a passive export. See https://github.com/gastownhall/beads/blob/main/docs/SYNC_CONCEPTS.md for details and anti-patterns.

## Agent Context Profiles

The managed Beads block is task-tracking guidance, not permission to override repository, user, or orchestrator instructions.

- **Conservative (default)**: Use `bd` for task tracking. Do not run git commits, git pushes, or Dolt remote sync unless explicitly asked. At handoff, report changed files, validation, and suggested next commands.
- **Minimal**: Keep tool instruction files as pointers to `bd prime`; use the same conservative git policy unless active instructions say otherwise.
- **Team-maintainer**: Only when the repository explicitly opts in, agents may close beads, run quality gates, commit, and push as part of session close. A current "do not commit" or "do not push" instruction still wins.

## Session Completion

This protocol applies when ending a Beads implementation workflow. It is subordinate to explicit user, repository, and orchestrator instructions.

1. **File issues for remaining work** - Create beads for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **Handle git/sync by active profile**:
   ```bash
   # Conservative/minimal/default: report status and proposed commands; wait for approval.
   git status

   # Team-maintainer opt-in only, unless current instructions forbid it:
   git pull --rebase
   bd dolt push
   git push
   git status
   ```
5. **Hand off** - Summarize changes, validation, issue status, and any blocked sync/commit/push step

**Critical rules:**
- Explicit user or orchestrator instructions override this Beads block.
- Do not commit or push without clear authority from the active profile or the current user request.
- If a required sync or push is blocked, stop and report the exact command and error.
<!-- END BEADS INTEGRATION -->

<!-- BEGIN BEADS CODEX SETUP: generated by bd setup codex -->
## Beads Issue Tracker

Use Beads (`bd`) for durable task tracking in repositories that include it. Use the `beads` skill at `.agents/skills/beads/SKILL.md` (project install) or `~/.agents/skills/beads/SKILL.md` (global install) for Beads workflow guidance, then use the `bd` CLI for issue operations.

### Quick Reference

```bash
bd ready                # Find available work
bd show <id>            # View issue details
bd update <id> --claim  # Claim work
bd close <id>           # Complete work
bd prime                # Refresh Beads context
```

### Rules

- Use `bd` for all task tracking; do not create markdown TODO lists.
- Run `bd prime` when Beads context is missing or stale. Codex 0.129.0+ can load Beads context automatically through native hooks; use `/hooks` to inspect or toggle them.
- Keep persistent project memory in Beads via `bd remember`; do not create ad hoc memory files.

**Architecture in one line:** issues live in a local Dolt DB; sync uses `refs/dolt/data` on your git remote; `.beads/issues.jsonl` is a passive export. See https://github.com/gastownhall/beads/blob/main/docs/SYNC_CONCEPTS.md for details and anti-patterns.
<!-- END BEADS CODEX SETUP -->
