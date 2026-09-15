# harness-check

The harness-level code quality gate. One Rust binary, three subcommands. It
enforces harness-level standards in every repo the harness touches, with zero
required changes inside those repos. Repos own policy; the harness owns the
writer.

## Subcommands

`check [files...]` reads a Claude Code hook payload from stdin, or file paths
from argv, and runs the harness rule table plus repo linters over the files.
The rule table is data: debug prints and bare excepts block, trailing
whitespace and unreferenced TODOs advise. Length rules are two-tier and
test-exempt: files advise past 500 lines and block past 1000; functions
advise past 60 and block past 100; classes advise past 200 and block past
400. Span detection is indentation for Python and brace-depth for the
brace languages, so strings containing lone braces can fool it by a line
or two; it is a length heuristic, not a parser. Repo lint configs win when
they exist; otherwise the harness baseline applies via `--config`. Linter
findings are advisory. Exit codes: 0 clean, 1 advisory, 2 blocking,
3 usage error.

`doctor [repo-root]` audits a repo against the baselines: are the baseline
files installed, does the repo config extend the baseline, which rule
families are missing, is the linter current. Audit only, always exit 0.

`pr-size [--limit N] [--base ref] [--force-size]` measures the branch diff
against the base, excluding generated paths. Over budget exits 2. Only a
human on an interactive terminal can accept an oversized PR: `--force-size`
prompts on a TTY and refuses in non-interactive contexts, so an agent cannot
decide to exceed the budget.

### What pr-size counts

The budget is changed lines, default 1000, overridable with `--limit`. The
range is every commit between the merge base and HEAD, where the base is
`--base`, then `origin/HEAD`, then `origin/main`, `origin/master`, `main`,
`master`, in that order.

Per file it sums added plus deleted lines from `git diff --numstat`. Binary
files are skipped. Files under an excluded path are skipped entirely:
`node_modules`, `.venv`, `venv`, `vendor`, `dist`, `build`, `out`,
`__pycache__`, `__marimo__`, `.beads`, `.git`, `.jj`, `fixtures`, `testdata`,
plus `*.lock`, `*.min.js`, `*.min.css`, `*.d.ts`, `*.snap`, `*.pb.go`, and
`*_pb2.py` anywhere in the path.

Only logic and tests count, per owner policy. Docs and data never do:
`*.md`, `*.markdown`, `*.mdx`, `*.rst`, `*.txt`, `*.adoc`, `*.json`,
`*.lock`, `*.snap`, and anything in a `docs/`, `doc/`, or `changelog/`
directory. Deleted files contribute their deletion count. Renames contribute
their net diff.

## Baselines

`lint-baselines/` holds the ruff and oxlint baselines. See
[lint-baselines/README.md](../lint-baselines/README.md) for provenance and
how a repo extends them. The baseline directory resolves from
`$HARNESS_LINTERS_DIR`, then `~/.config/lazar-harness/linters`.

## Wiring

Claude Code: a `PostToolUse` hook matched on `Edit|Write` runs `check` on
every file the agent touches; blocking findings reach the agent through
stderr and it fixes them inline. The same hooks run inside subagents.
OpenCode: a global plugin calls `check` on edit tools. install.sh writes both.

## Latency budget

The edit loop must stay fast. The binary alone answers in a couple of
milliseconds; each linter invocation is capped at 500 ms and a missing or
slow linter degrades silently to the harness rules.

## Building

`cargo build --release` produces `target/release/harness-check` (about 1.7 MB
with the release profile: LTO, stripped, size-optimized). Dependencies are
serde, serde_json, toml, and regex, chosen so real config parsing replaces
the hand-rolled parsers a no-dependency build would need.
