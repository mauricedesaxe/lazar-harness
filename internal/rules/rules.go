package rules

import (
	"bufio"
	"os"
	"path/filepath"
	"regexp"
	"strconv"
	"strings"

	"github.com/mauricedesaxe/lazar-harness/internal/findings"
)

// Rule is one declarative entry in the harness rule table. Adding a rule is
// adding an entry, not editing control flow.
type Rule struct {
	ID        string
	Exts      map[string]bool
	Pattern   *regexp.Regexp
	SkipTests bool
	Severity  findings.Severity
	Message   string
}

const FileLengthRule = "file-length"

var Default = []Rule{
	{
		ID:        "debug-print",
		Exts:      set(".py", ".ts", ".tsx", ".js", ".jsx"),
		Pattern:   regexp.MustCompile(`\bprint\(|\bconsole\.log\(`),
		SkipTests: true,
		Severity:  findings.Blocking,
		Message:   "Debug print in source. Remove it or switch to a logger.",
	},
	{
		ID:        "bare-except",
		Exts:      set(".py"),
		Pattern:   regexp.MustCompile(`^\s*except\s*:\s*$`),
		SkipTests: true,
		Severity:  findings.Blocking,
		Message:   "Bare except swallows every error. Catch the specific exception or let it propagate.",
	},
	{
		ID:       "trailing-whitespace",
		Exts:     set(".py", ".ts", ".tsx", ".js", ".jsx", ".go"),
		Pattern:  regexp.MustCompile(`[ \t]+$`),
		Severity: findings.Advisory,
		Message:  "Trailing whitespace.",
	},
	{
		ID:       "todo-no-ref",
		Exts:     set(".py", ".ts", ".tsx", ".js", ".jsx", ".go"),
		Pattern:  regexp.MustCompile(`\b(TODO|FIXME)\b`),
		Severity: findings.Advisory,
		Message:  "TODO or FIXME without an issue reference. Link the work or finish it.",
	},
}

// Excluded reports whether a path is generated, vendored, or lockfile noise
// that no harness rule should ever scan.
func Excluded(path string) bool {
	if filepath.IsAbs(path) {
		path = filepath.ToSlash(path)
	}
	return excludedRe.MatchString(path)
}

var excludedRe = regexp.MustCompile(
	`(^|/)(node_modules|\.venv|venv|vendor|dist|build|out|__pycache__|__marimo__|\.beads|\.git|\.jj|fixtures|testdata)(/|/\.|$)` +
		`|\.lock$|\.min\.(js|jsx|css)$|\.d\.ts$|\.snap$|\.pb\.go$|_pb2\.py$`)

var testRe = regexp.MustCompile(
	`(^|/)(tests?|__tests__)(/|$)|_test\.go$|\.test\.[jt]sx?$|\.spec\.[jt]sx?$|_test\.py$|^test_[^/]*\.py$`)

var issueRefRe = regexp.MustCompile(`#\d+|[A-Za-z][A-Za-z0-9]*-\w*\d[\w-]*|https?://`)

func set(exts ...string) map[string]bool {
	m := make(map[string]bool, len(exts))
	for _, e := range exts {
		m[e] = true
	}
	return m
}

type Engine struct {
	Rules      []Rule
	MaxFileLen int
}

func NewEngine() Engine {
	return Engine{Rules: Default, MaxFileLen: 800}
}

// Run scans the given files and returns findings. Files that do not exist,
// are excluded, or are binary are skipped silently so the caller can pass a
// raw list without pre-filtering.
func (e Engine) Run(files []string) []findings.Finding {
	var out []findings.Finding
	seen := make(map[string]bool)
	for _, file := range files {
		for _, f := range e.scan(file) {
			key := f.File + "|" + f.Rule + "|" + strconv.Itoa(f.Line)
			if seen[key] {
				continue
			}
			seen[key] = true
			out = append(out, f)
		}
	}
	return out
}

func (e Engine) scan(file string) []findings.Finding {
	ext := strings.ToLower(filepath.Ext(file))
	content, err := os.ReadFile(file)
	if err != nil || isBinary(content) {
		return nil
	}
	isTest := testRe.MatchString(file)

	var out []findings.Finding
	scanner := bufio.NewScanner(strings.NewReader(string(content)))
	scanner.Buffer(make([]byte, 0, 64*1024), 1024*1024)
	line := 0
	for scanner.Scan() {
		line++
		text := scanner.Text()
		for _, r := range e.Rules {
			if len(r.Exts) > 0 && !r.Exts[ext] {
				continue
			}
			if r.SkipTests && isTest {
				continue
			}
			if r.ID == "todo-no-ref" && issueRefRe.MatchString(text) {
				continue
			}
			if r.Pattern.MatchString(text) {
				out = append(out, findings.Finding{
					File: file, Line: line, Rule: r.ID,
					Severity: r.Severity, Message: r.Message, Source: "harness",
				})
			}
		}
	}
	if e.MaxFileLen > 0 && line > e.MaxFileLen && !isTest && codeExts[ext] {
		out = append(out, findings.Finding{
			File: file, Line: line, Rule: FileLengthRule,
			Severity: findings.Advisory,
			Message:  "File is over " + strconv.Itoa(e.MaxFileLen) + " lines. Consider splitting it.",
			Source:   "harness",
		})
	}
	return out
}

var codeExts = set(".py", ".ts", ".tsx", ".js", ".jsx", ".go")

func isBinary(b []byte) bool {
	n := len(b)
	if n > 8000 {
		n = 8000
	}
	return strings.ContainsRune(string(b[:n]), 0)
}
