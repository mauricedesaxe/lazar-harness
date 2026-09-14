package hook

import (
	"errors"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestClaudeEditPayloadYieldsPath(t *testing.T) {
	in := `{"tool_name":"Edit","tool_input":{"file_path":"/repo/main.py","old_string":"a","new_string":"b"},"session_id":"abc"}`
	paths, err := PathsFromStdin(strings.NewReader(in))
	if err != nil {
		t.Fatal(err)
	}
	if len(paths) != 1 || paths[0] != "/repo/main.py" {
		t.Fatalf("want /repo/main.py, got %v", paths)
	}
}

func TestNotebookPayloadYieldsNotebookPath(t *testing.T) {
	in := `{"tool_name":"NotebookEdit","tool_input":{"notebook_path":"/repo/n.ipynb"}}`
	paths, err := PathsFromStdin(strings.NewReader(in))
	if err != nil {
		t.Fatal(err)
	}
	if len(paths) != 1 || paths[0] != "/repo/n.ipynb" {
		t.Fatalf("want /repo/n.ipynb, got %v", paths)
	}
}

func TestUncheckedToolYieldsNothing(t *testing.T) {
	in := `{"tool_name":"Bash","tool_input":{"command":"ls"}}`
	paths, err := PathsFromStdin(strings.NewReader(in))
	if err != nil || paths != nil {
		t.Fatalf("unchecked tool must pass through, got %v err %v", paths, err)
	}
}

func TestEmptyStdinYieldsNothing(t *testing.T) {
	paths, err := PathsFromStdin(strings.NewReader(""))
	if err != nil || paths != nil {
		t.Fatalf("empty stdin must pass through, got %v err %v", paths, err)
	}
}

func TestMalformedPayloadIsUsageError(t *testing.T) {
	_, err := PathsFromStdin(strings.NewReader("{not json"))
	if err == nil || !strings.Contains(err.Error(), "malformed hook payload") {
		t.Fatalf("malformed payload must error, got %v", err)
	}
}

func TestEditPayloadWithoutPathIsUsageError(t *testing.T) {
	_, err := PathsFromStdin(strings.NewReader(`{"tool_name":"Edit","tool_input":{}}`))
	if err == nil {
		t.Fatal("Edit payload without a path must error")
	}
}

func TestPathsFromArgsRejectsMissingFile(t *testing.T) {
	dir := t.TempDir()
	ok := filepath.Join(dir, "a.py")
	if err := os.WriteFile(ok, []byte("x = 1\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	paths, err := PathsFromArgs([]string{ok})
	if err != nil || len(paths) != 1 {
		t.Fatalf("existing file must pass, got %v err %v", paths, err)
	}
	if _, err := PathsFromArgs([]string{filepath.Join(dir, "nope.py")}); !errors.Is(err, os.ErrNotExist) {
		t.Fatalf("missing file must be a not-exist error, got %v", err)
	}
}
