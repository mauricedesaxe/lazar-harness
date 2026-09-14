package doctor

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/mauricedesaxe/lazar-harness/internal/resolve"
)

const baselineRuff = `select = ["E", "F", "I", "UP", "B", "SIM", "RET", "S", "RUF"]
`

func TestNoConfigWarnsWithBaselineMessage(t *testing.T) {
	repo := t.TempDir()
	report := Run(repo, resolve.Options{BaselineDir: t.TempDir()})
	if !hasCheck(report, "ruff-config", WARN, "baseline applies") {
		t.Fatalf("missing-config repo must warn about baseline, got %+v", report.Checks)
	}
}

func TestExtendingRepoPasses(t *testing.T) {
	repo := t.TempDir()
	writeRepo(t, filepath.Join(repo, "ruff.toml"),
		"extend = \"/home/x/.config/lazar-harness/linters/ruff.toml\"\n"+baselineRuff)
	report := Run(repo, resolve.Options{BaselineDir: writeBaseline(t)})
	if !hasCheck(report, "ruff-config", OK, "extends") {
		t.Fatalf("extending repo must pass, got %+v", report.Checks)
	}
	if !hasCheck(report, "ruff-families", OK, "covers all") {
		t.Fatalf("full-family repo must pass families, got %+v", report.Checks)
	}
}

func TestMissingFamiliesReported(t *testing.T) {
	repo := t.TempDir()
	writeRepo(t, filepath.Join(repo, "ruff.toml"),
		"extend = \"~/.config/lazar-harness/linters/ruff.toml\"\nselect = [\"E\", \"F\"]\n")
	report := Run(repo, resolve.Options{BaselineDir: writeBaseline(t)})
	if !hasCheck(report, "ruff-families", WARN, "UP, B, SIM, RET, S, RUF") {
		t.Fatalf("missing families must be listed, got %+v", report.Checks)
	}
}

func TestOwnConfigNotExtendingWarns(t *testing.T) {
	repo := t.TempDir()
	writeRepo(t, filepath.Join(repo, "ruff.toml"), baselineRuff)
	report := Run(repo, resolve.Options{BaselineDir: writeBaseline(t)})
	if !hasCheck(report, "ruff-config", WARN, "does not extend") {
		t.Fatalf("non-extending config must warn, got %+v", report.Checks)
	}
}

func TestOxlintConfigChecked(t *testing.T) {
	repo := t.TempDir()
	writeRepo(t, filepath.Join(repo, ".oxlintrc.json"),
		"{\"extends\": [\"/h/.config/lazar-harness/linters/oxlint.json\"]}")
	report := Run(repo, resolve.Options{BaselineDir: t.TempDir()})
	if !hasCheck(report, "oxlint-config", OK, "extends") {
		t.Fatalf("extending oxlint config must pass, got %+v", report.Checks)
	}
}

func TestToolVersionReported(t *testing.T) {
	repo := t.TempDir()
	report := Run(repo, resolve.Options{BaselineDir: t.TempDir(), RuffBin: "/no/such/ruff", OxlintBin: "/no/such/oxlint"})
	if !hasCheck(report, "ruff", INFO, "not on PATH") {
		t.Fatalf("missing tool must be info, got %+v", report.Checks)
	}
}

func writeBaseline(t *testing.T) string {
	t.Helper()
	dir := t.TempDir()
	writeRepo(t, filepath.Join(dir, "ruff.toml"), baselineRuff)
	writeRepo(t, filepath.Join(dir, "oxlint.json"), "{}")
	return dir
}

func writeRepo(t *testing.T, path, content string) {
	t.Helper()
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(path, []byte(content), 0o644); err != nil {
		t.Fatal(err)
	}
}

func hasCheck(r Report, name string, status Status, detailPart string) bool {
	for _, c := range r.Checks {
		if c.Name == name && c.Status == status && strings.Contains(c.Detail, detailPart) {
			return true
		}
	}
	return false
}
