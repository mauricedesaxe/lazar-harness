mod comments;
mod doctor;
mod findings;
mod hook;
mod prsize;
mod resolve;
mod rules;

use std::io::Read;

use findings::Severity;

const USAGE: &str = "harness-check: the harness-level code quality gate

Usage:
  harness-check check [files...]      check files given as argv, or a hook
                                      payload on stdin
  harness-check doctor [repo-root]    audit a repo against the harness baselines
  harness-check pr-size [options]     measure the branch diff against the base
      --limit N                       budget in changed lines (default 1000)
      --base <ref>                    base branch (default: origin HEAD)
      --force-size                    human-gated override, interactive only
  harness-check version";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args.first().map(|s| s.as_str()) {
        Some("check") => cmd_check(&args[1..]),
        Some("doctor") => cmd_doctor(&args[1..]),
        Some("pr-size") => cmd_pr_size(&args[1..]),
        Some("version") | Some("--version") => {
            println!("harness-check {}", env!("CARGO_PKG_VERSION"));
            0
        }
        _ => {
            eprintln!("{USAGE}");
            3
        }
    };
    std::process::exit(code);
}

fn cmd_check(args: &[String]) -> i32 {
    let mut as_json = false;
    let mut rest = vec![];
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--json" => as_json = true,
            other => rest.push(other.to_string()),
        }
        i += 1;
    }

    let mut files: Vec<String> = vec![];
    if !is_stdin_a_tty() {
        let mut raw = String::new();
        if std::io::stdin().read_to_string(&mut raw).is_err() {
            eprintln!("failed to read stdin");
            return 3;
        }
        match hook::paths_from_stdin(&raw) {
            hook::StdinPayload::Paths(p) => files = p,
            hook::StdinPayload::None => {}
            hook::StdinPayload::Malformed(msg) => {
                eprintln!("{msg}");
                return 3;
            }
        }
    }
    if files.is_empty() {
        match hook::paths_from_args(&rest) {
            Ok(p) => files = p,
            Err(e) => {
                eprintln!("{e}");
                return 3;
            }
        }
    }

    let opts = resolve::Options::default();
    let mut all = vec![];
    if !files.is_empty() {
        let root = repo_root(&files[0]);
        let cfgs = resolve::detect(&root);
        all.extend(rules::Engine::default().run(&files));
        all.extend(comments::check_files(&files));
        all.extend(resolve::linter_files(&cfgs, &opts, &files));
        all = dedupe(all);
    }
    let result = findings::Outcome { findings: all };

    if as_json {
        println!("{}", serde_json::to_string_pretty(&result).unwrap());
    } else {
        print_report(&result.findings);
    }
    result.exit_code()
}

fn cmd_doctor(args: &[String]) -> i32 {
    let mut as_json = false;
    let mut rest = vec![];
    for a in args {
        if a == "--json" {
            as_json = true;
        } else {
            rest.push(a.clone());
        }
    }
    let root = rest.first().cloned().unwrap_or_else(|| ".".into());
    let report = doctor::run(&root, &resolve::Options::default());
    if as_json {
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
    } else {
        print!("{report}");
    }
    0
}

fn cmd_pr_size(args: &[String]) -> i32 {
    let mut opts = prsize::Options::default();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--limit" => {
                i += 1;
                match args.get(i).and_then(|v| v.parse::<usize>().ok()) {
                    Some(n) if n > 0 => opts.limit = n,
                    _ => {
                        eprintln!("--limit needs a number of at least 1");
                        return 3;
                    }
                }
            }
            "--base" => {
                i += 1;
                match args.get(i) {
                    Some(b) => opts.base = Some(b.clone()),
                    None => {
                        eprintln!("--base needs a ref");
                        return 3;
                    }
                }
            }
            "--force-size" => opts.force = true,
            other => {
                eprintln!("unknown pr-size argument: {other}");
                return 3;
            }
        }
        i += 1;
    }
    prsize::run_main(".", opts)
}

fn repo_root(from: &str) -> String {
    let mut dir = std::fs::canonicalize(from).unwrap_or_else(|_| ".".into());
    if dir.is_file() {
        dir = dir.parent().unwrap().to_path_buf();
    }
    loop {
        if dir.join(".git").exists() {
            return dir.to_string_lossy().into_owned();
        }
        match dir.parent() {
            Some(p) => dir = p.to_path_buf(),
            None => return ".".into(),
        }
    }
}

fn dedupe(all: Vec<findings::Finding>) -> Vec<findings::Finding> {
    let mut out = vec![];
    let mut seen = std::collections::HashSet::new();
    for f in all {
        let key = format!("{}|{}|{}", f.file, f.rule, f.line);
        if seen.insert(key) {
            out.push(f);
        }
    }
    out
}

fn is_stdin_a_tty() -> bool {
    // The release matrix includes darwin, where /proc does not exist, so
    // this must go through libc rather than the filesystem.
    unsafe { libc::isatty(0) == 1 }
}

fn print_report(all: &[findings::Finding]) {
    if all.is_empty() {
        println!("harness-check: clean");
        return;
    }
    for f in all {
        println!(
            "{}:{} [{:?}/{}] {}",
            f.file, f.line, f.severity, f.source, f.message
        );
    }
    let blocking = all
        .iter()
        .filter(|f| f.severity == Severity::Blocking)
        .count();
    let advisory = all.len() - blocking;
    println!("harness-check: {blocking} blocking, {advisory} advisory");
    if blocking > 0 {
        // The PostToolUse hook feeds stderr back to the agent, so the
        // specifics of what to fix travel on stderr, not the summary.
        for f in all.iter().filter(|f| f.severity == Severity::Blocking) {
            eprintln!("{}:{} [{}] {}", f.file, f.line, f.rule, f.message);
        }
        eprintln!("harness-check: blocking findings must be fixed before continuing");
    }
}
