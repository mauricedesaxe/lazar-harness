use std::io::{BufRead, Write};
use std::process::Command;

use regex::Regex;
use serde::Serialize;

use crate::rules;

#[derive(Debug, Clone, Default)]
pub struct Options {
    pub limit: usize,
    pub base: Option<String>,
    pub force: bool,
}

#[derive(Debug, Serialize)]
pub struct FileCount {
    pub path: String,
    pub lines: usize,
}

#[derive(Debug, Default, Serialize)]
pub struct Summary {
    pub total: usize,
    pub limit: usize,
    pub base: String,
    pub files: Vec<FileCount>,
}

impl std::fmt::Display for Summary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "PR size: {} changed lines (limit {}) against {}",
            self.total, self.limit, self.base
        )?;
        for (i, file) in self.files.iter().enumerate() {
            if i == 10 {
                writeln!(f, "  ... and {} more files", self.files.len() - 10)?;
                break;
            }
            writeln!(f, "  {:>5} {}", file.lines, file.path)?;
        }
        Ok(())
    }
}

// notCounted marks paths that never reach the budget: docs and data files.
// The owner policy is logic plus tests only.
fn not_counted() -> &'static Regex {
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"\.(json|md|markdown|mdx|rst|txt|adoc|lock|snap)$|(^|/)(docs?|changelog)(/|$)")
            .unwrap()
    })
}

pub fn run(
    dir: &str,
    opts: Options,
    stdin: &mut impl BufRead,
    stdout: &mut impl Write,
    stdin_is_tty: bool,
) -> (Summary, i32) {
    let opts = if opts.limit == 0 {
        Options {
            limit: 1000,
            ..opts
        }
    } else {
        opts
    };
    let Some(base) = detect_base(dir, opts.base.clone()) else {
        let _ = writeln!(stdout, "could not detect a base branch; pass --base <ref>");
        return (
            Summary {
                total: 0,
                limit: opts.limit,
                base: String::new(),
                files: vec![],
            },
            3,
        );
    };
    let Some(summary) = measure(dir, &base, opts.limit) else {
        let _ = writeln!(stdout, "diff against {base} failed");
        return (
            Summary {
                total: 0,
                limit: opts.limit,
                base,
                files: vec![],
            },
            3,
        );
    };

    if summary.total <= opts.limit {
        let _ = write!(stdout, "{}", summary);
        return (summary, 0);
    }

    let _ = write!(stdout, "{}", summary);
    let _ = writeln!(
        stdout,
        "PR size {} exceeds the limit of {}. Exceeding it requires explicit human acceptance.",
        summary.total, opts.limit
    );

    if !opts.force {
        let _ = writeln!(
            stdout,
            "Ask the human to accept, then have them run: harness-check pr-size --force-size"
        );
        return (summary, 2);
    }
    if !stdin_is_tty {
        let _ = writeln!(
            stdout,
            "--force-size requires an interactive terminal. Agents cannot accept on the human's behalf."
        );
        return (summary, 2);
    }
    let _ = write!(stdout, "Exceed the PR size limit of {}? [y/N] ", opts.limit);
    let _ = stdout.flush();
    let mut answer = String::new();
    let _ = stdin.read_line(&mut answer);
    let answer = answer.trim().to_lowercase();
    if answer == "y" || answer == "yes" {
        let _ = writeln!(
            stdout,
            "Human accepted PR size {} over {}.",
            summary.total, opts.limit
        );
        return (summary, 0);
    }
    let _ = writeln!(stdout, "Not accepted. Split the change.");
    (summary, 2)
}

fn stdin_is_tty_now() -> bool {
    std::io::IsTerminal::is_terminal(&std::io::stdin())
}

pub fn run_main(dir: &str, opts: Options) -> i32 {
    let tty = stdin_is_tty_now();
    let mut stdin = std::io::stdin().lock();
    let mut stdout = std::io::stdout();
    let (_, code) = run(dir, opts, &mut stdin, &mut stdout, tty);
    code
}

fn git(dir: &str, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .ok()?;
    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        None
    }
}

fn detect_base(dir: &str, flag: Option<String>) -> Option<String> {
    if let Some(b) = flag {
        return Some(b);
    }
    if let Some(out) = git(dir, &["symbolic-ref", "refs/remotes/origin/HEAD"]) {
        return Some(out.trim().to_string());
    }
    ["origin/main", "origin/master", "main", "master"]
        .iter()
        .find(|c| git(dir, &["rev-parse", "--verify", "--quiet", c]).is_some())
        .map(|c| c.to_string())
}

fn measure(dir: &str, base: &str, limit: usize) -> Option<Summary> {
    let mb = git(dir, &["merge-base", base, "HEAD"])?;
    let mb = mb.trim();
    let diff = git(dir, &["diff", "--numstat", mb, "HEAD"])?;
    let mut summary = Summary {
        limit,
        base: base.into(),
        ..Default::default()
    };
    for line in diff.lines() {
        let mut parts = line.splitn(3, '\t');
        let (Some(adds), Some(dels), Some(path)) = (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        if adds == "-" || rules::excluded(path) || not_counted().is_match(path) {
            continue;
        }
        let n: usize = adds.parse::<usize>().ok()? + dels.parse::<usize>().ok()?;
        if n == 0 {
            continue;
        }
        summary.total += n;
        summary.files.push(FileCount {
            path: path.into(),
            lines: n,
        });
    }
    summary.files.sort_by_key(|f| std::cmp::Reverse(f.lines));
    Some(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Cursor;
    use std::path::Path;
    use std::process::Command;

    fn git(dir: &str, args: &[&str]) {
        let out = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .env("GIT_AUTHOR_NAME", "test")
            .env("GIT_AUTHOR_EMAIL", "t@t")
            .env("GIT_COMMITTER_NAME", "test")
            .env("GIT_COMMITTER_EMAIL", "t@t")
            .output()
            .expect("git");
        assert!(
            out.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    fn write_file(dir: &str, name: &str, content: &str, msg: &str) {
        let path = format!("{dir}/{name}");
        fs::create_dir_all(Path::new(&path).parent().unwrap()).unwrap();
        fs::write(&path, content).unwrap();
        git(dir, &["add", name]);
        git(dir, &["commit", "-m", msg]);
    }

    fn fixture_repo() -> String {
        let name = std::thread::current()
            .name()
            .unwrap_or("t")
            .replace("::", "-");
        let dir = std::env::temp_dir().join(format!("hc-prsize-{}-{}", std::process::id(), name));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let dir = dir.to_string_lossy().into_owned();
        git(&dir, &["init", "-b", "main"]);
        write_file(&dir, "src/keep.py", "x = 1\n", "initial");
        git(&dir, &["checkout", "-b", "feature"]);
        dir
    }

    #[test]
    fn under_budget_exits_clean() {
        let dir = fixture_repo();
        write_file(&dir, "src/small.py", &"x = 1\n".repeat(10), "small");
        let mut out = Vec::new();
        let (summary, code) = run(
            &dir,
            Options {
                base: Some("main".into()),
                ..Default::default()
            },
            &mut Cursor::new(""),
            &mut out,
            false,
        );
        assert_eq!(code, 0, "{}", String::from_utf8_lossy(&out));
        assert_eq!(summary.total, 10);
    }

    #[test]
    fn over_budget_exits_two_without_force() {
        let dir = fixture_repo();
        write_file(&dir, "src/big.py", &"x = 1\n".repeat(50), "big");
        let mut out = Vec::new();
        let (_, code) = run(
            &dir,
            Options {
                base: Some("main".into()),
                limit: 10,
                ..Default::default()
            },
            &mut Cursor::new(""),
            &mut out,
            false,
        );
        assert_eq!(code, 2, "{}", String::from_utf8_lossy(&out));
        let text = String::from_utf8_lossy(&out);
        assert!(text.contains("explicit human acceptance"), "{text}");
        assert!(text.contains("Ask the human"), "{text}");
    }

    #[test]
    fn force_without_tty_is_refused() {
        let dir = fixture_repo();
        write_file(&dir, "src/big.py", &"x = 1\n".repeat(50), "big");
        let mut out = Vec::new();
        let (_, code) = run(
            &dir,
            Options {
                base: Some("main".into()),
                limit: 10,
                force: true,
            },
            &mut Cursor::new("y\n"),
            &mut out,
            false,
        );
        assert_eq!(code, 2, "{}", String::from_utf8_lossy(&out));
        assert!(String::from_utf8_lossy(&out).contains("interactive terminal"));
    }

    #[test]
    fn force_with_tty_and_human_yes_accepts() {
        let dir = fixture_repo();
        write_file(&dir, "src/big.py", &"x = 1\n".repeat(50), "big");
        let mut out = Vec::new();
        let (_, code) = run(
            &dir,
            Options {
                base: Some("main".into()),
                limit: 10,
                force: true,
            },
            &mut Cursor::new("y\n"),
            &mut out,
            true,
        );
        assert_eq!(code, 0, "{}", String::from_utf8_lossy(&out));
        assert!(String::from_utf8_lossy(&out).contains("Human accepted"));
    }

    #[test]
    fn force_with_tty_and_human_no_refuses() {
        let dir = fixture_repo();
        write_file(&dir, "src/big.py", &"x = 1\n".repeat(50), "big");
        let mut out = Vec::new();
        let (_, code) = run(
            &dir,
            Options {
                base: Some("main".into()),
                limit: 10,
                force: true,
            },
            &mut Cursor::new("n\n"),
            &mut out,
            true,
        );
        assert_eq!(code, 2);
    }

    #[test]
    fn docs_and_json_do_not_count() {
        let dir = fixture_repo();
        write_file(&dir, "README.md", &"words\n".repeat(100), "docs");
        write_file(&dir, "pkg/data.json", &"{\"k\": 1}\n".repeat(100), "data");
        write_file(&dir, "src/logic.py", &"x = 1\n".repeat(5), "logic");
        let mut out = Vec::new();
        let (summary, code) = run(
            &dir,
            Options {
                base: Some("main".into()),
                limit: 10,
                ..Default::default()
            },
            &mut Cursor::new(""),
            &mut out,
            false,
        );
        assert_eq!(code, 0, "{}", String::from_utf8_lossy(&out));
        assert_eq!(
            summary.total,
            5,
            "only logic counts: {}",
            String::from_utf8_lossy(&out)
        );
    }

    #[test]
    fn base_detection_falls_back_to_main() {
        let dir = fixture_repo();
        write_file(&dir, "src/tiny.py", "x = 1\n", "tiny");
        assert_eq!(detect_base(&dir, None).as_deref(), Some("main"));
    }
}
