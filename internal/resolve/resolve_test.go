package resolve

import (
	"os"
	"path/filepath"
	"testing"

	"github.com/mauricedesaxe/lazar-harness/internal/findings"
)

func write(t *testing.T, path, content string) {
	t.Helper()
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(path, []byte(content), 0o644); err != nil {
		t.Fatal(err)
	}
}

const fakeLinterTemplate = `#!/bin/sh
echo "$FAKE_LINTER_ARGS $@" >> "$RECORD"
echo "$FAKE_LINTER_OUTPUT"
exit 0
`

// fakeLinter installs a shell script as a stand-in linter binary that records
// its argv and prints a canned response. Outputs here were recorded from
// ruff 0.16.7 and oxlint 1.83.0 on probe files.
func fakeLinter(t *testing.T, name, output string) (binDir, record string) {
	t.Helper()
	binDir = t.TempDir()
	record = filepath.Join(t.TempDir(), "args.log")
	script := `#!/bin/sh
echo "$@" >> "` + record + `"
cat <<'JSON'
` + output + `
JSON
exit 0
`
	write(t, filepath.Join(binDir, name), script)
	if err := os.Chmod(filepath.Join(binDir, name), 0o755); err != nil {
		t.Fatal(err)
	}
	return binDir, record
}

func readRecord(t *testing.T, record string) string {
	t.Helper()
	b, err := os.ReadFile(record)
	if err != nil {
		t.Fatalf("linter was never invoked: %v", err)
	}
	return string(b)
}

const ruffOutput = `[
  {
    "code": "B006",
    "message": "Mutable default argument of type ` + "`list`" + ` is forbidden",
    "filename": "/repo/src/bad.py",
    "location": {"row": 3, "column": 9}
  }
]`

const oxlintJSON = `{
  "diagnostics": [
    {
      "code": "eslint(no-unused-vars)",
      "message": "Variable 'unused' is declared but never used.",
      "severity": "warning",
      "filename": "src/bad.ts",
      "labels": [{"span": {"offset": 21, "length": 6, "line": 1, "column": 22}}]
    }
  ]
}`

func TestRepoConfigWinsOverBaseline(t *testing.T) {
	repo := t.TempDir()
	write(t, filepath.Join(repo, "ruff.toml"), "line-length = 100\n")
	write(t, filepath.Join(repo, "src", "bad.py"), "def f(x=[]):\n    pass\n")

	binDir, record := fakeLinter(t, "ruff", ruffOutput)
	cfgs := Detect(repo)
	got := LinterFiles(cfgs, Options{RuffBin: filepath.Join(binDir, "ruff"), BaselineDir: t.TempDir()},
		[]string{filepath.Join(repo, "src", "bad.py")})

	if len(got) != 1 || got[0].Rule != "ruff:B006" || got[0].Severity != findings.Advisory {
		t.Fatalf("want one advisory ruff:B006, got %+v", got)
	}
	rec := readRecord(t, record)
	if !contains(rec, "--config "+filepath.Join(repo, "ruff.toml")) {
		t.Fatalf("repo config must win, argv was: %s", rec)
	}
}

func TestBaselineUsedWhenRepoHasNoConfig(t *testing.T) {
	repo := t.TempDir()
	write(t, filepath.Join(repo, "bad.py"), "def f(x=[]):\n    pass\n")

	baselineDir := t.TempDir()
	write(t, filepath.Join(baselineDir, "ruff.toml"), "line-length = 100\n")
	binDir, record := fakeLinter(t, "ruff", ruffOutput)

	got := LinterFiles(Detect(repo), Options{RuffBin: filepath.Join(binDir, "ruff"), BaselineDir: baselineDir},
		[]string{filepath.Join(repo, "bad.py")})

	if len(got) != 1 {
		t.Fatalf("want one finding, got %+v", got)
	}
	rec := readRecord(t, record)
	if !contains(rec, "--config "+filepath.Join(baselineDir, "ruff.toml")) {
		t.Fatalf("baseline config must apply when repo has none, argv was: %s", rec)
	}
}

func TestPyprojectWithoutRuffSectionIsNotConfig(t *testing.T) {
	repo := t.TempDir()
	write(t, filepath.Join(repo, "pyproject.toml"), "[project]\nname = \"x\"\n")

	baselineDir := t.TempDir()
	write(t, filepath.Join(baselineDir, "ruff.toml"), "")
	binDir, record := fakeLinter(t, "ruff", "[]")

	LinterFiles(Detect(repo), Options{RuffBin: filepath.Join(binDir, "ruff"), BaselineDir: baselineDir},
		[]string{filepath.Join(repo, "main.py")})

	rec := readRecord(t, record)
	if !contains(rec, filepath.Join(baselineDir, "ruff.toml")) {
		t.Fatalf("pyproject without [tool.ruff] must not count as config, argv was: %s", rec)
	}
}

func TestMissingToolDegradesSilently(t *testing.T) {
	repo := t.TempDir()
	write(t, filepath.Join(repo, "bad.py"), "x = 1\n")
	got := LinterFiles(Detect(repo), Options{RuffBin: filepath.Join(t.TempDir(), "no-such-ruff")},
		[]string{filepath.Join(repo, "bad.py")})
	if len(got) != 0 {
		t.Fatalf("missing linter must degrade to no findings, got %+v", got)
	}
}

func TestOxlintDiagnosticsParsed(t *testing.T) {
	repo := t.TempDir()
	bad := filepath.Join(repo, "src", "bad.ts")
	write(t, bad, "function f() { const unused = 1; }\nf();\n")

	binDir, _ := fakeLinter(t, "oxlint", oxlintJSON)
	t.Chdir(repo)
	got := LinterFiles(Detect(repo), Options{OxlintBin: filepath.Join(binDir, "oxlint"), BaselineDir: t.TempDir()},
		[]string{bad})

	if len(got) != 1 || got[0].Rule != "oxlint:eslint(no-unused-vars)" || got[0].Line != 1 {
		t.Fatalf("want oxlint no-unused-vars at line 1, got %+v", got)
	}
	if got[0].File != bad {
		t.Fatalf("relative linter filename must resolve to %s, got %s", bad, got[0].File)
	}
}

func TestDetectFindsOxlintConfig(t *testing.T) {
	repo := t.TempDir()
	write(t, filepath.Join(repo, ".oxlintrc.json"), "{\"rules\":{}}")
	cfgs := Detect(repo)
	if cfgs.OxlintConfig == "" {
		t.Fatal(".oxlintrc.json must be detected")
	}
}

func contains(hay, needle string) bool {
	return len(hay) >= len(needle) && (func() bool {
		for i := 0; i+len(needle) <= len(hay); i++ {
			if hay[i:i+len(needle)] == needle {
				return true
			}
		}
		return false
	})()
}
