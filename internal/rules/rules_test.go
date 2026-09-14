package rules

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/mauricedesaxe/lazar-harness/internal/findings"
)

func fixture(name string) string {
	return filepath.Join("testdata", name)
}

func findingsFor(t *testing.T, files []string, rule string) []findings.Finding {
	t.Helper()
	var out []findings.Finding
	for _, f := range NewEngine().Run(files) {
		if f.Rule == rule {
			out = append(out, f)
		}
	}
	return out
}

func TestDebugPrintFlagsSourceNotTests(t *testing.T) {
	hit := findingsFor(t, []string{fixture("bad.py"), fixture("bad.ts")}, "debug-print")
	if len(hit) != 2 {
		t.Fatalf("want 2 debug-print findings, got %d: %+v", len(hit), hit)
	}
	miss := findingsFor(t, []string{fixture("tests/test_bad.py")}, "debug-print")
	if len(miss) != 0 {
		t.Fatalf("debug-print must skip tests, got %+v", miss)
	}
}

func TestBareExceptFlags(t *testing.T) {
	hit := findingsFor(t, []string{fixture("bad.py")}, "bare-except")
	if len(hit) != 1 || hit[0].Line != 8 {
		t.Fatalf("want bare-except at line 8, got %+v", hit)
	}
}

func TestTodoWithoutIssueRefFlags(t *testing.T) {
	hit := findingsFor(t, []string{fixture("bad.py")}, "todo-no-ref")
	if len(hit) != 1 || hit[0].Line != 12 {
		t.Fatalf("want todo-no-ref at line 11, got %+v", hit)
	}
	clean := findingsFor(t, []string{fixture("good.py")}, "todo-no-ref")
	if len(clean) != 0 {
		t.Fatalf("referenced TODO must pass, got %+v", clean)
	}
}

func TestExcludedPathsNeverScanned(t *testing.T) {
	if !Excluded("a/vendor/dep.py") {
		t.Fatal("vendored paths must be excluded for walkers")
	}
	if !Excluded("a/node_modules/b.js") || !Excluded("pkg/x.lock") || Excluded("pkg/main.go") {
		t.Fatal("exclusion globs deviate from the contract")
	}
}

func TestSeveritySplit(t *testing.T) {
	res := findings.Result{Findings: NewEngine().Run([]string{fixture("bad.py"), fixture("bad.ts")})}
	if res.Count(findings.Blocking) == 0 {
		t.Fatal("bad fixtures must produce blocking findings")
	}
	if res.ExitCode() != 2 {
		t.Fatalf("blocking result must exit 2, got %d", res.ExitCode())
	}
	onlyAdvisory := findings.Result{Findings: []findings.Finding{{Severity: findings.Advisory}}}
	if onlyAdvisory.ExitCode() != 1 {
		t.Fatalf("advisory-only result must exit 1, got %d", onlyAdvisory.ExitCode())
	}
}

func TestFileLengthAdvisory(t *testing.T) {
	dir := t.TempDir()
	long := filepath.Join(dir, "long.py")
	body := "x = 1\n"
	content := strings.Repeat(body, 801)
	if err := os.WriteFile(long, []byte(content), 0o644); err != nil {
		t.Fatal(err)
	}
	hit := findingsFor(t, []string{long}, FileLengthRule)
	if len(hit) != 1 {
		t.Fatalf("want file-length finding, got %+v", hit)
	}
}

func TestBinaryAndMissingFilesSkipped(t *testing.T) {
	dir := t.TempDir()
	bin := filepath.Join(dir, "bin.py")
	if err := os.WriteFile(bin, []byte("print(\x00\x01)"), 0o644); err != nil {
		t.Fatal(err)
	}
	if got := NewEngine().Run([]string{bin, filepath.Join(dir, "missing.py")}); len(got) != 0 {
		t.Fatalf("binary and missing files must be skipped, got %+v", got)
	}
}
