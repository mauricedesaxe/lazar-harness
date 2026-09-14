package hook

import (
	"encoding/json"
	"fmt"
	"io"
	"os"
)

// Payload mirrors the documented Claude Code hook input shape. Only the
// fields this tool needs are declared; unknown fields are ignored.
type Payload struct {
	ToolName  string `json:"tool_name"`
	ToolInput struct {
		FilePath     string `json:"file_path"`
		NotebookPath string `json:"notebook_path"`
	} `json:"tool_input"`
}

var checkedTools = map[string]bool{
	"Edit": true, "Write": true, "MultiEdit": true, "NotebookEdit": true,
}

// PathsFromStdin extracts the files to check from a hook payload. An empty
// stream or a payload for an unchecked tool returns no paths and no error,
// so callers proceed with argv files. A malformed payload is a usage error.
func PathsFromStdin(r io.Reader) ([]string, error) {
	raw, err := io.ReadAll(io.LimitReader(r, 1<<20))
	if err != nil {
		return nil, fmt.Errorf("read stdin: %w", err)
	}
	trimmed := trimSpace(raw)
	if len(trimmed) == 0 {
		return nil, nil
	}
	if trimmed[0] != '{' {
		return nil, nil
	}
	var p Payload
	if err := json.Unmarshal(trimmed, &p); err != nil {
		return nil, fmt.Errorf("malformed hook payload: %w", err)
	}
	if !checkedTools[p.ToolName] {
		return nil, nil
	}
	path := p.ToolInput.FilePath
	if path == "" {
		path = p.ToolInput.NotebookPath
	}
	if path == "" {
		return nil, fmt.Errorf("payload for %q has no file path", p.ToolName)
	}
	return []string{path}, nil
}

// PathsFromArgs validates argv file paths and reports missing ones as usage
// errors so manual runs fail loudly.
func PathsFromArgs(args []string) ([]string, error) {
	var paths []string
	for _, a := range args {
		if a == "-" {
			continue
		}
		if _, err := os.Stat(a); err != nil {
			return nil, fmt.Errorf("file not found: %w", err)
		}
		paths = append(paths, a)
	}
	return paths, nil
}

func trimSpace(b []byte) []byte {
	start := 0
	for start < len(b) && isSpace(b[start]) {
		start++
	}
	end := len(b)
	for end > start && isSpace(b[end-1]) {
		end--
	}
	return b[start:end]
}

func isSpace(c byte) bool {
	return c == ' ' || c == '\t' || c == '\r' || c == '\n'
}
