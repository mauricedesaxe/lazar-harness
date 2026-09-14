package doctor

import (
	"fmt"
	"os"
	"os/exec"
	"regexp"
	"strings"

	"github.com/mauricedesaxe/lazar-harness/internal/resolve"
)

type Status string

const (
	OK   Status = "ok"
	WARN Status = "warn"
	INFO Status = "info"
)

type Check struct {
	Name   string `json:"name"`
	Status Status `json:"status"`
	Detail string `json:"detail"`
}

type Report struct {
	Root   string  `json:"root"`
	Checks []Check `json:"checks"`
}

func (r Report) String() string {
	var b strings.Builder
	fmt.Fprintf(&b, "harness-check doctor: %s\n", r.Root)
	for _, c := range r.Checks {
		fmt.Fprintf(&b, "%-4s %s: %s\n", strings.ToUpper(string(c.Status)), c.Name, c.Detail)
	}
	return b.String()
}

func Run(root string, opts resolve.Options) Report {
	cfgs := resolve.Detect(root)
	var checks []Check
	checks = append(checks, ruffChecks(root, cfgs, opts)...)
	checks = append(checks, oxlintChecks(root, cfgs, opts)...)
	checks = append(checks, toolCheck("ruff", orDefault(opts.RuffBin, "ruff")), toolCheck("oxlint", orDefault(opts.OxlintBin, "oxlint")))
	return Report{Root: root, Checks: checks}
}

func orDefault(bin, fallback string) string {
	if bin == "" {
		return fallback
	}
	return bin
}

func ruffChecks(root string, cfgs resolve.RepoConfigs, opts resolve.Options) []Check {
	var out []Check
	if cfgs.RuffConfig == "" {
		return append(out, Check{
			Name: "ruff-config", Status: WARN,
			Detail: "no repo ruff config found; the harness baseline applies at edit time. Extend it to own this repo's policy.",
		})
	}
	if extendsBaseline(cfgs.RuffConfig) {
		out = append(out, Check{Name: "ruff-config", Status: OK, Detail: "extends the harness baseline"})
	} else {
		out = append(out, Check{
			Name: "ruff-config", Status: WARN,
			Detail: "does not extend the harness baseline; add extend = \"<baseline>/ruff.toml\" to inherit updates",
		})
	}
	missing := missingFamilies(cfgs.RuffConfig, opts)
	if len(missing) > 0 {
		out = append(out, Check{
			Name: "ruff-families", Status: WARN,
			Detail: "baseline families missing from repo config: " + strings.Join(missing, ", "),
		})
	} else {
		out = append(out, Check{Name: "ruff-families", Status: OK, Detail: "covers all baseline families"})
	}
	return out
}

func oxlintChecks(root string, cfgs resolve.RepoConfigs, opts resolve.Options) []Check {
	var out []Check
	if cfgs.OxlintConfig == "" {
		return append(out, Check{
			Name: "oxlint-config", Status: WARN,
			Detail: "no .oxlintrc.json found; the harness baseline applies at edit time. Extend it to own this repo's policy.",
		})
	}
	if extendsBaseline(cfgs.OxlintConfig) {
		out = append(out, Check{Name: "oxlint-config", Status: OK, Detail: "extends the harness baseline"})
	} else {
		out = append(out, Check{
			Name: "oxlint-config", Status: WARN,
			Detail: "does not extend the harness baseline; add it under extends to inherit updates",
		})
	}
	return out
}

func toolCheck(name string, bin string) Check {
	path, err := exec.LookPath(bin)
	if err != nil {
		return Check{Name: name, Status: INFO, Detail: bin + " not on PATH; edit-time " + name + " checks are skipped"}
	}
	out, err := exec.Command(bin, "--version").Output()
	detail := path
	if err == nil {
		detail = strings.TrimSpace(string(out))
	}
	return Check{Name: name, Status: OK, Detail: detail}
}

var extendHint = regexp.MustCompile(`lazar-harness/linters|lint-baselines`)

func extendsBaseline(configPath string) bool {
	b, err := os.ReadFile(configPath)
	return err == nil && extendHint.Match(b)
}

var selectRe = regexp.MustCompile(`select\s*=\s*\[([^\]]*)\]`)

// parseSelects is a best-effort reader for the ruff `select` list. It strips
// TOML line comments first, because real configs annotate every family, then
// reads the plain quoted-name subset.
func parseSelects(configPath string) []string {
	b, err := os.ReadFile(configPath)
	if err != nil {
		return nil
	}
	var noComments strings.Builder
	for _, line := range strings.Split(string(b), "\n") {
		if idx := strings.Index(line, "#"); idx >= 0 {
			line = line[:idx]
		}
		noComments.WriteString(line)
		noComments.WriteString(" ")
	}
	m := selectRe.FindStringSubmatch(noComments.String())
	if m == nil {
		return nil
	}
	var out []string
	for _, part := range strings.Split(m[1], ",") {
		name := strings.Trim(strings.TrimSpace(part), "\"'")
		if name != "" {
			out = append(out, name)
		}
	}
	return out
}

func missingFamilies(repoConfig string, opts resolve.Options) []string {
	repo := parseSelects(repoConfig)
	if len(repo) == 0 {
		return nil
	}
	base := parseSelects(resolve.BaselinePath(opts, "ruff.toml"))
	have := make(map[string]bool, len(repo))
	for _, f := range repo {
		have[f] = true
	}
	var missing []string
	for _, f := range base {
		if !have[f] {
			missing = append(missing, f)
		}
	}
	return missing
}
