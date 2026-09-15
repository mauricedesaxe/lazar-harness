package prsize

import (
	"bufio"
	"fmt"
	"io"
	"os"
	"os/exec"
	"regexp"
	"strconv"
	"strings"

	"github.com/mauricedesaxe/lazar-harness/internal/rules"
)

type Options struct {
	Limit int
	Base  string
	Force bool
}

type FileCount struct {
	Path  string `json:"path"`
	Lines int    `json:"lines"`
}

type Summary struct {
	Total int         `json:"total"`
	Limit int         `json:"limit"`
	Base  string      `json:"base"`
	Files []FileCount `json:"files"`
}

func (s Summary) String() string {
	var b strings.Builder
	fmt.Fprintf(&b, "PR size: %d changed lines (limit %d) against %s\n", s.Total, s.Limit, s.Base)
	for i, f := range s.Files {
		if i == 10 {
			fmt.Fprintf(&b, "  ... and %d more files\n", len(s.Files)-10)
			break
		}
		fmt.Fprintf(&b, "  %5d %s\n", f.Lines, f.Path)
	}
	return b.String()
}

// notCounted marks paths that never reach the budget: docs and data files.
// The owner policy is logic plus tests only.
var notCounted = regexp.MustCompile(
	`\.(json|md|markdown|mdx|rst|txt|adoc|lock|snap)$` +
		`|(^|/)(docs?|changelog)(/|$)`)

func Run(dir string, opts Options, stdin io.Reader, stdout io.Writer) (Summary, int, error) {
	if opts.Limit <= 0 {
		opts.Limit = 1000
	}
	base, err := detectBase(dir, opts.Base)
	if err != nil {
		return Summary{}, 3, err
	}
	summary, err := measure(dir, base, opts.Limit)
	if err != nil {
		return Summary{}, 3, err
	}
	if summary.Total <= opts.Limit {
		fmt.Fprint(stdout, summary.String())
		return summary, 0, nil
	}

	fmt.Fprint(stdout, summary.String())
	fmt.Fprintf(stdout, "PR size %d exceeds the limit of %d. Exceeding it requires explicit human acceptance.\n",
		summary.Total, opts.Limit)

	if !opts.Force {
		fmt.Fprintln(stdout, "Ask the human to accept, then have them run: harness-check pr-size --force-size")
		return summary, 2, nil
	}
	if !stdinIsTTY() {
		fmt.Fprintln(stdout, "--force-size requires an interactive terminal. Agents cannot accept on the human's behalf.")
		return summary, 2, nil
	}
	fmt.Fprintf(stdout, "Exceed the PR size limit of %d? [y/N] ", opts.Limit)
	answer, _ := bufio.NewReader(stdin).ReadString('\n')
	answer = strings.ToLower(strings.TrimSpace(answer))
	if answer == "y" || answer == "yes" {
		fmt.Fprintf(stdout, "Human accepted PR size %d over %d.\n", summary.Total, opts.Limit)
		return summary, 0, nil
	}
	fmt.Fprintln(stdout, "Not accepted. Split the change.")
	return summary, 2, nil
}

var stdinIsTTY = func() bool {
	info, err := os.Stdin.Stat()
	if err != nil {
		return false
	}
	return info.Mode()&os.ModeCharDevice != 0
}

func detectBase(dir, flag string) (string, error) {
	if flag != "" {
		return flag, nil
	}
	if out, err := runGit(dir, "symbolic-ref", "refs/remotes/origin/HEAD"); err == nil {
		return strings.TrimSpace(out), nil
	}
	for _, candidate := range []string{"origin/main", "origin/master", "main", "master"} {
		if err := runGitQuiet(dir, "rev-parse", "--verify", "--quiet", candidate); err == nil {
			return candidate, nil
		}
	}
	return "", fmt.Errorf("could not detect a base branch; pass --base <ref>")
}

func measure(dir, base string, limit int) (Summary, error) {
	mb, err := runGit(dir, "merge-base", base, "HEAD")
	if err != nil {
		return Summary{}, fmt.Errorf("merge-base against %s: %w", base, err)
	}
	out, err := runGit(dir, "diff", "--numstat", strings.TrimSpace(mb), "HEAD")
	if err != nil {
		return Summary{}, fmt.Errorf("diff against %s: %w", base, err)
	}
	summary := Summary{Limit: limit, Base: base}
	for _, line := range strings.Split(out, "\n") {
		if strings.TrimSpace(line) == "" {
			continue
		}
		parts := strings.SplitN(line, "\t", 3)
		if len(parts) != 3 || parts[0] == "-" {
			continue
		}
		if rules.Excluded(parts[2]) || notCounted.MatchString(parts[2]) {
			continue
		}
		adds, _ := strconv.Atoi(parts[0])
		dels, _ := strconv.Atoi(parts[1])
		n := adds + dels
		if n == 0 {
			continue
		}
		summary.Total += n
		summary.Files = append(summary.Files, FileCount{Path: parts[2], Lines: n})
	}
	sortFiles(summary.Files)
	return summary, nil
}

func sortFiles(files []FileCount) {
	for i := 1; i < len(files); i++ {
		for j := i; j > 0 && files[j].Lines > files[j-1].Lines; j-- {
			files[j], files[j-1] = files[j-1], files[j]
		}
	}
}

func runGit(dir string, args ...string) (string, error) {
	out, err := exec.Command("git", append([]string{"-C", dir}, args...)...).Output()
	if err != nil {
		return "", err
	}
	return string(out), nil
}

func runGitQuiet(dir string, args ...string) error {
	_, err := runGit(dir, args...)
	return err
}
