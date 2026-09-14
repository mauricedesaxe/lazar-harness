# Lint baselines

The harness baseline configs, installed to `~/.config/lazar-harness/linters/`
by `install.sh`. Repos stay responsible for their own policy: a repo with no
lint config gets the baseline applied by `harness-check check`, and a repo
that wants to own or tune policy extends the baseline.

```toml
# ruff.toml in any repo
extend = "/absolute/path/to/.config/lazar-harness/linters/ruff.toml"
```

```json
// .oxlintrc.json in any repo
{ "extends": ["/absolute/path/to/.config/lazar-harness/linters/oxlint.json"] }
```

Absolute-path `extends` is verified working in oxlint 1.83.0 and supported by
ruff. `harness-check doctor` audits whether a repo extends the baseline and
which rule families it is missing.

## Provenance and deviations

The ruff baseline is chartly's `ruff.toml` widened: chartly hand-picked
subfamilies (`SIM102`, `S110`, `RUF005`, ...) and the baseline enables the
whole families (`SIM`, `S`, `RUF`). The baseline keeps chartly's `E501` and
`UP040` ignores and adds `S101` allowances for test files. The oxlint
baseline is the pragmatic starter: correctness at error, suspicious at warn.

When the baseline changes, every repo that extends it picks the change up on
its next lint run. `doctor` reports version drift for repos pinning older
linter releases.
