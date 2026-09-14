package prsize

import (
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
)

func git(t *testing.T, dir string, args ...string) string {
	t.Helper()
	cmd := exec.Command("git", append([]string{"-C", dir}, args...)...)
	cmd.Env = append(os.Environ(),
		"GIT_AUTHOR_NAME=test", "GIT_AUTHOR_EMAIL=t@t",
		"GIT_COMMITTER_NAME=test", "GIT_COMMITTER_EMAIL=t@t")
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("git %v: %v\n%s", args, err, out)
	}
	return string(out)
}

func fixtureRepo(t *testing.T) string {
	t.Helper()
	dir := t.TempDir()
	git(t, dir, "init", "-b", "main")
	writeFile(t, dir, "src/keep.py", "x = 1\n", "initial")
	git(t, dir, "checkout", "-b", "feature")
	return dir
}

func writeFile(t *testing.T, dir, name, content, msg string) {
	t.Helper()
	path := filepath.Join(dir, name)
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(path, []byte(content), 0o644); err != nil {
		t.Fatal(err)
	}
	git(t, dir, "add", name)
	git(t, dir, "commit", "-m", msg)
}

func TestUnderBudgetExitsClean(t *testing.T) {
	dir := fixtureRepo(t)
	writeFile(t, dir, "src/small.py", strings.Repeat("x = 1\n", 10), "small change")
	var out strings.Builder
	summary, code, err := Run(dir, Options{Base: "main"}, strings.NewReader(""), &out)
	if err != nil || code != 0 {
		t.Fatalf("want exit 0, got %d err %v\n%s", code, err, out.String())
	}
	if summary.Total != 10 {
		t.Fatalf("want 10 changed lines, got %d", summary.Total)
	}
}

func TestOverBudgetExitsTwoWithoutForce(t *testing.T) {
	dir := fixtureRepo(t)
	writeFile(t, dir, "src/big.py", strings.Repeat("x = 1\n", 50), "big change")
	var out strings.Builder
	_, code, err := Run(dir, Options{Base: "main", Limit: 10}, strings.NewReader(""), &out)
	if err != nil {
		t.Fatal(err)
	}
	if code != 2 {
		t.Fatalf("over budget without force must exit 2, got %d\n%s", code, out.String())
	}
	if !strings.Contains(out.String(), "explicit human acceptance") {
		t.Fatalf("must state human acceptance policy, got: %s", out.String())
	}
	if !strings.Contains(out.String(), "Ask the human") {
		t.Fatalf("must tell the agent to ask, got: %s", out.String())
	}
}

func TestForceWithoutTTYIsRefused(t *testing.T) {
	dir := fixtureRepo(t)
	writeFile(t, dir, "src/big.py", strings.Repeat("x = 1\n", 50), "big change")
	orig := stdinIsTTY
	stdinIsTTY = func() bool { return false }
	defer func() { stdinIsTTY = orig }()

	var out strings.Builder
	_, code, err := Run(dir, Options{Base: "main", Limit: 10, Force: true}, strings.NewReader("y\n"), &out)
	if err != nil {
		t.Fatal(err)
	}
	if code != 2 {
		t.Fatalf("force over a pipe must be refused with exit 2, got %d\n%s", code, out.String())
	}
	if !strings.Contains(out.String(), "interactive terminal") {
		t.Fatalf("refusal must name the interactive requirement, got: %s", out.String())
	}
}

func TestExcludedPathsDoNotCount(t *testing.T) {
	dir := fixtureRepo(t)
	writeFile(t, dir, "pkg/bun.lockb-to-ignore.lock", strings.Repeat("x\n", 100), "lockfile noise")
	var out strings.Builder
	summary, code, err := Run(dir, Options{Base: "main", Limit: 5}, strings.NewReader(""), &out)
	if err != nil || code != 0 {
		t.Fatalf("lockfile lines must not count, got exit %d err %v\n%s", code, err, out.String())
	}
	if summary.Total != 0 {
		t.Fatalf("want 0 counted lines, got %d", summary.Total)
	}
}

func TestForceWithTTYAndHumanYesAccepts(t *testing.T) {
	dir := fixtureRepo(t)
	writeFile(t, dir, "src/big.py", strings.Repeat("x = 1\n", 50), "big change")
	orig := stdinIsTTY
	stdinIsTTY = func() bool { return true }
	defer func() { stdinIsTTY = orig }()

	var out strings.Builder
	_, code, err := Run(dir, Options{Base: "main", Limit: 10, Force: true}, strings.NewReader("y\n"), &out)
	if err != nil {
		t.Fatal(err)
	}
	if code != 0 {
		t.Fatalf("human acceptance must exit 0, got %d\n%s", code, out.String())
	}
	if !strings.Contains(out.String(), "Human accepted") {
		t.Fatalf("acceptance must be recorded, got: %s", out.String())
	}
}

func TestForceWithTTYAndHumanNoRefuses(t *testing.T) {
	dir := fixtureRepo(t)
	writeFile(t, dir, "src/big.py", strings.Repeat("x = 1\n", 50), "big change")
	orig := stdinIsTTY
	stdinIsTTY = func() bool { return true }
	defer func() { stdinIsTTY = orig }()

	var out strings.Builder
	_, code, err := Run(dir, Options{Base: "main", Limit: 10, Force: true}, strings.NewReader("n\n"), &out)
	if err != nil {
		t.Fatal(err)
	}
	if code != 2 {
		t.Fatalf("declined acceptance must exit 2, got %d\n%s", code, out.String())
	}
}

func TestBaseDetectionFallsBackToMain(t *testing.T) {
	dir := fixtureRepo(t)
	writeFile(t, dir, "src/tiny.py", "x = 1\n", "tiny")
	base, err := detectBase(dir, "")
	if err != nil {
		t.Fatal(err)
	}
	if base != "main" {
		t.Fatalf("want main as detected base, got %s", base)
	}
}
