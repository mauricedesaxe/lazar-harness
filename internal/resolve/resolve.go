package resolve

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"time"

	"github.com/mauricedesaxe/lazar-harness/internal/findings"
)

type Options struct {
	BaselineDir string
	Timeout     time.Duration
	RuffBin     string
	OxlintBin   string
}

func (o Options) baselineDir() string {
	if o.BaselineDir != "" {
		return o.BaselineDir
	}
	if d := os.Getenv("HARNESS_LINTERS_DIR"); d != "" {
		return d
	}
	home, err := os.UserHomeDir()
	if err != nil {
		return ""
	}
	return filepath.Join(home, ".config", "lazar-harness", "linters")
}

func (o Options) timeout() time.Duration {
	if o.Timeout > 0 {
		return o.Timeout
	}
	return 500 * time.Millisecond
}

func (o Options) ruffBin() string {
	if o.RuffBin != "" {
		return o.RuffBin
	}
	return "ruff"
}

func (o Options) oxlintBin() string {
	if o.OxlintBin != "" {
		return o.OxlintBin
	}
	return "oxlint"
}

type RepoConfigs struct {
	RuffConfig   string
	OxlintConfig string
}

var ruffConfigNames = []string{"ruff.toml", ".ruff.toml"}

// Detect inspects root for repo-owned lint configs. A pyproject.toml only
// counts as a ruff config when it declares [tool.ruff], so a repo with an
// unrelated pyproject still gets the harness baseline.
func Detect(root string) RepoConfigs {
	var c RepoConfigs
	for _, name := range ruffConfigNames {
		p := filepath.Join(root, name)
		if fileExists(p) {
			c.RuffConfig = p
			break
		}
	}
	if c.RuffConfig == "" {
		py := filepath.Join(root, "pyproject.toml")
		if hasRuffSection(py) {
			c.RuffConfig = py
		}
	}
	ox := filepath.Join(root, ".oxlintrc.json")
	if fileExists(ox) {
		c.OxlintConfig = ox
	}
	return c
}

func hasRuffSection(path string) bool {
	b, err := os.ReadFile(path)
	if err != nil {
		return false
	}
	return strings.Contains(string(b), "[tool.ruff]")
}

func fileExists(p string) bool {
	info, err := os.Stat(p)
	return err == nil && !info.IsDir()
}

// LinterFiles runs repo linters over files, falling back to the harness
// baseline when the repo owns no config. Linter findings come back advisory:
// repos gate them in CI, the harness surfaces them at edit time. A missing
// or timed-out linter degrades silently.
func LinterFiles(cfgs RepoConfigs, opts Options, files []string) []findings.Finding {
	var out []findings.Finding
	var pyFiles, tsFiles []string
	for _, f := range files {
		switch strings.ToLower(filepath.Ext(f)) {
		case ".py":
			pyFiles = append(pyFiles, f)
		case ".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs":
			tsFiles = append(tsFiles, f)
		}
	}
	if len(pyFiles) > 0 && lookPath(opts.ruffBin()) {
		cfg := cfgs.RuffConfig
		if cfg == "" {
			cfg = filepath.Join(opts.baselineDir(), "ruff.toml")
		}
		out = append(out, runRuff(opts, cfg, pyFiles)...)
	}
	if len(tsFiles) > 0 && lookPath(opts.oxlintBin()) {
		cfg := cfgs.OxlintConfig
		if cfg == "" {
			cfg = filepath.Join(opts.baselineDir(), "oxlint.json")
		}
		out = append(out, runOxlint(opts, cfg, tsFiles)...)
	}
	return out
}

func lookPath(bin string) bool {
	_, err := exec.LookPath(bin)
	return err == nil
}

// stdoutOf captures stdout even when the linter exits nonzero, which both
// ruff and oxlint do when they report findings. A nonzero exit that still
// produced output is not an error here; only a failed start or a timeout is.
func stdoutOf(ctx context.Context, bin string, args []string) ([]byte, error) {
	var buf bytes.Buffer
	cmd := exec.CommandContext(ctx, bin, args...)
	cmd.Stdout = &buf
	err := cmd.Run()
	if err == nil {
		return buf.Bytes(), nil
	}
	if _, ok := err.(*exec.ExitError); ok && buf.Len() > 0 {
		return buf.Bytes(), nil
	}
	return nil, err
}

type ruffDiag struct {
	Code     string `json:"code"`
	Message  string `json:"message"`
	Filename string `json:"filename"`
	Location struct {
		Row    int `json:"row"`
		Column int `json:"column"`
	} `json:"location"`
}

func runRuff(opts Options, config string, files []string) []findings.Finding {
	ctx, cancel := context.WithTimeout(context.Background(), opts.timeout())
	defer cancel()
	args := []string{"check", "--config", config, "--output-format", "json", "--no-cache"}
	args = append(args, files...)
	out, err := stdoutOf(ctx, opts.ruffBin(), args)
	if err != nil || len(out) == 0 {
		return nil
	}
	var diags []ruffDiag
	if json.Unmarshal(out, &diags) != nil {
		return nil
	}
	var res []findings.Finding
	for _, d := range diags {
		res = append(res, findings.Finding{
			File: d.Filename, Line: d.Location.Row,
			Rule: "ruff:" + d.Code, Severity: findings.Advisory,
			Message: d.Message, Source: "ruff",
		})
	}
	return res
}

type oxlintOutput struct {
	Diagnostics []oxlintDiag `json:"diagnostics"`
}

type oxlintDiag struct {
	Code     string `json:"code"`
	Message  string `json:"message"`
	Severity string `json:"severity"`
	Filename string `json:"filename"`
	Labels   []struct {
		Span struct {
			Line   int `json:"line"`
			Column int `json:"column"`
		} `json:"span"`
	} `json:"labels"`
}

func runOxlint(opts Options, config string, tsFiles []string) []findings.Finding {
	ctx, cancel := context.WithTimeout(context.Background(), opts.timeout())
	defer cancel()
	args := []string{"-c", config, "-f", "json", "--quiet"}
	args = append(args, tsFiles...)
	out, err := stdoutOf(ctx, opts.oxlintBin(), args)
	if err != nil || len(out) == 0 {
		return nil
	}
	var parsed oxlintOutput
	if json.Unmarshal(out, &parsed) != nil {
		return nil
	}
	var res []findings.Finding
	for _, d := range parsed.Diagnostics {
		line := 0
		if len(d.Labels) > 0 {
			line = d.Labels[0].Span.Line
		}
		if !filepath.IsAbs(d.Filename) {
			if abs, err := filepath.Abs(d.Filename); err == nil {
				d.Filename = abs
			}
		}
		res = append(res, findings.Finding{
			File: d.Filename, Line: line,
			Rule: "oxlint:" + d.Code, Severity: findings.Advisory,
			Message: d.Message, Source: "oxlint",
		})
	}
	return res
}

// BaselinePath returns the path to a named baseline config file.
func BaselinePath(opts Options, name string) string {
	return filepath.Join(opts.baselineDir(), name)
}

// ValidateBaseline errors when the baseline directory is missing a config.
func ValidateBaseline(opts Options) error {
	dir := opts.baselineDir()
	for _, name := range []string{"ruff.toml", "oxlint.json"} {
		if !fileExists(filepath.Join(dir, name)) {
			return fmt.Errorf("baseline %s not found in %s (set HARNESS_LINTERS_DIR or run install.sh)", name, dir)
		}
	}
	return nil
}
