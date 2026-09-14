package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"os"
	"path/filepath"
	"strconv"
	"strings"

	"github.com/mauricedesaxe/lazar-harness/internal/doctor"
	"github.com/mauricedesaxe/lazar-harness/internal/findings"
	"github.com/mauricedesaxe/lazar-harness/internal/hook"
	"github.com/mauricedesaxe/lazar-harness/internal/prsize"
	"github.com/mauricedesaxe/lazar-harness/internal/resolve"
	"github.com/mauricedesaxe/lazar-harness/internal/rules"
)

var version = "0.1.0-dev"

const usage = `harness-check: the harness-level code quality gate

Usage:
  harness-check check [files...]      check files given as argv, or a hook
                                      payload on stdin
  harness-check doctor [repo-root]    audit a repo against the harness baselines
  harness-check pr-size [options]     measure the branch diff against the base
      --limit N                       budget in changed lines (default 1000)
      --base <ref>                    base branch (default: origin HEAD)
      --force-size                    human-gated override, interactive only
  harness-check version`

func main() {
	if len(os.Args) < 2 {
		fmt.Fprintln(os.Stderr, usage)
		os.Exit(3)
	}
	var code int
	switch os.Args[1] {
	case "check":
		code = cmdCheck(os.Args[2:])
	case "doctor":
		code = cmdDoctor(os.Args[2:])
	case "pr-size":
		code = cmdPrSize(os.Args[2:])
	case "version", "--version":
		fmt.Println("harness-check " + version)
		return
	default:
		fmt.Fprintln(os.Stderr, usage)
		code = 3
	}
	os.Exit(code)
}

func cmdCheck(args []string) int {
	fs := flag.NewFlagSet("check", flag.ContinueOnError)
	asJSON := fs.Bool("json", false, "emit findings as JSON")
	if err := fs.Parse(args); err != nil {
		return 3
	}

	var files []string
	var err error
	stat, _ := os.Stdin.Stat()
	if stat != nil && stat.Mode()&os.ModeCharDevice == 0 {
		files, err = hook.PathsFromStdin(os.Stdin)
		if err != nil {
			fmt.Fprintln(os.Stderr, err)
			return 3
		}
	}
	if len(files) == 0 {
		files, err = hook.PathsFromArgs(fs.Args())
		if err != nil {
			fmt.Fprintln(os.Stderr, err)
			return 3
		}
	}

	opts := resolve.Options{}
	var result findings.Result
	if len(files) > 0 {
		cfgs := resolve.Detect(repoRoot(files[0]))
		var all []findings.Finding
		all = append(all, rules.NewEngine().Run(files)...)
		all = append(all, resolve.LinterFiles(cfgs, opts, files)...)
		result.Findings = dedupe(all)
	}

	if *asJSON {
		enc := json.NewEncoder(os.Stdout)
		enc.SetIndent("", "  ")
		if enc.Encode(result) != nil {
			return 3
		}
	} else {
		printReport(result)
	}
	return result.ExitCode()
}

func cmdDoctor(args []string) int {
	fs := flag.NewFlagSet("doctor", flag.ContinueOnError)
	asJSON := fs.Bool("json", false, "emit report as JSON")
	if err := fs.Parse(args); err != nil {
		return 3
	}
	root := "."
	if fs.NArg() > 0 {
		root = fs.Arg(0)
	}
	report := doctor.Run(root, resolve.Options{})
	if *asJSON {
		enc := json.NewEncoder(os.Stdout)
		enc.SetIndent("", "  ")
		if enc.Encode(report) != nil {
			return 3
		}
	} else {
		fmt.Print(report.String())
	}
	return 0
}

func cmdPrSize(args []string) int {
	fs := flag.NewFlagSet("pr-size", flag.ContinueOnError)
	limit := fs.Int("limit", 1000, "budget in changed lines")
	base := fs.String("base", "", "base branch ref")
	force := fs.Bool("force-size", false, "human-gated override, interactive only")
	if err := fs.Parse(args); err != nil {
		return 3
	}
	summary, code, err := prsize.Run(".", prsize.Options{
		Limit: *limit, Base: *base, Force: *force,
	}, os.Stdin, os.Stdout)
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		return code
	}
	if code == 2 {
		fmt.Fprintln(os.Stderr, summary.String())
	}
	return code
}

func repoRoot(from string) string {
	abs, err := filepath.Abs(from)
	if err != nil {
		return "."
	}
	dir := abs
	if info, err := os.Stat(abs); err == nil && !info.IsDir() {
		dir = filepath.Dir(abs)
	}
	for {
		if _, err := os.Stat(filepath.Join(dir, ".git")); err == nil {
			return dir
		}
		parent := filepath.Dir(dir)
		if parent == dir {
			return "."
		}
		dir = parent
	}
}

func dedupe(in []findings.Finding) []findings.Finding {
	var out []findings.Finding
	seen := make(map[string]bool)
	for _, f := range in {
		key := strings.Join([]string{f.File, f.Rule, strconv.Itoa(f.Line)}, "|")
		if seen[key] {
			continue
		}
		seen[key] = true
		out = append(out, f)
	}
	return out
}

func printReport(result findings.Result) {
	blocking := result.Count(findings.Blocking)
	advisory := result.Count(findings.Advisory)
	if len(result.Findings) == 0 {
		fmt.Println("harness-check: clean")
		return
	}
	for _, f := range result.Findings {
		fmt.Printf("%s:%d [%s/%s] %s\n", f.File, f.Line, f.Severity, f.Source, f.Message)
	}
	fmt.Printf("harness-check: %d blocking, %d advisory\n", blocking, advisory)
	if blocking > 0 {
		fmt.Fprintln(os.Stderr, "harness-check: blocking findings must be fixed before continuing")
	}
}
