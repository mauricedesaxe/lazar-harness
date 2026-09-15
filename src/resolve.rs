use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde::Deserialize;

use crate::findings::{Finding, Severity};

#[derive(Debug, Clone, Default)]
pub struct Options {
    pub baseline_dir: Option<String>,
    pub timeout: Option<Duration>,
    pub ruff_bin: Option<String>,
    pub oxlint_bin: Option<String>,
}

impl Options {
    pub fn baseline_dir(&self) -> String {
        if let Some(d) = &self.baseline_dir {
            return d.clone();
        }
        if let Ok(d) = std::env::var("HARNESS_LINTERS_DIR")
            && !d.is_empty()
        {
            return d;
        }
        let home = std::env::var("HOME").unwrap_or_default();
        format!("{home}/.config/lazar-harness/linters")
    }

    fn timeout(&self) -> Duration {
        self.timeout.unwrap_or(Duration::from_millis(500))
    }

    pub fn ruff_bin(&self) -> &str {
        self.ruff_bin.as_deref().unwrap_or("ruff")
    }

    pub fn oxlint_bin(&self) -> &str {
        self.oxlint_bin.as_deref().unwrap_or("oxlint")
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct RepoConfigs {
    pub ruff_config: Option<String>,
    pub oxlint_config: Option<String>,
}

/// Detect inspects root for repo-owned lint configs. A pyproject.toml only
/// counts as a ruff config when it declares [tool.ruff], so a repo with an
/// unrelated pyproject still gets the harness baseline.
pub fn detect(root: &str) -> RepoConfigs {
    let mut c = RepoConfigs::default();
    for name in [".ruff.toml", "ruff.toml"] {
        let p = format!("{root}/{name}");
        if Path::new(&p).is_file() {
            c.ruff_config = Some(p);
            break;
        }
    }
    if c.ruff_config.is_none() {
        let py = format!("{root}/pyproject.toml");
        if std::fs::read_to_string(&py).is_ok_and(|s| s.contains("[tool.ruff]")) {
            c.ruff_config = Some(py);
        }
    }
    let ox = format!("{root}/.oxlintrc.json");
    if Path::new(&ox).is_file() {
        c.oxlint_config = Some(ox);
    }
    c
}

/// LinterFiles runs repo linters over files, falling back to the harness
/// baseline when the repo owns no config. Linter findings come back advisory:
/// repos gate them in CI, the harness surfaces them at edit time. A missing
/// or timed-out linter degrades silently.
pub fn linter_files(cfgs: &RepoConfigs, opts: &Options, files: &[String]) -> Vec<Finding> {
    let mut out = vec![];
    let py: Vec<_> = files
        .iter()
        .filter(|f| f.ends_with(".py"))
        .cloned()
        .collect();
    let ts: Vec<_> = files
        .iter()
        .filter(|f| {
            [".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs"]
                .iter()
                .any(|e| f.ends_with(e))
        })
        .cloned()
        .collect();

    if !py.is_empty() && which_bin(opts.ruff_bin()) {
        let cfg = cfgs
            .ruff_config
            .clone()
            .unwrap_or_else(|| format!("{}/ruff.toml", opts.baseline_dir()));
        out.extend(run_ruff(opts, &cfg, &py));
    }
    if !ts.is_empty() && which_bin(opts.oxlint_bin()) {
        let cfg = cfgs
            .oxlint_config
            .clone()
            .unwrap_or_else(|| format!("{}/oxlint.json", opts.baseline_dir()));
        out.extend(run_oxlint(opts, &cfg, &ts));
    }
    out
}

pub fn which_bin(bin: &str) -> bool {
    Command::new("which")
        .arg(bin)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// stdoutOf captures stdout even when the linter exits nonzero, which both
/// ruff and oxlint do when they report findings. A reader thread drains the
/// pipe so a chatty child cannot deadlock; a failed start or a timeout
/// yields None.
fn stdout_of(opts: &Options, bin: &str, args: &[String]) -> Option<String> {
    let mut child = Command::new(bin)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let mut stdout = child.stdout.take()?;
    let reader = std::thread::spawn(move || {
        use std::io::Read;
        let mut buf = String::new();
        let _ = stdout.read_to_string(&mut buf);
        buf
    });
    let deadline = opts.timeout();
    let started = Instant::now();
    let output = loop {
        match child.try_wait() {
            Ok(Some(_)) => break reader.join().ok(),
            Ok(None) if started.elapsed() < deadline => {
                std::thread::sleep(Duration::from_millis(5));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    };
    output.filter(|s| !s.is_empty())
}

#[derive(Deserialize)]
struct RuffDiag {
    code: String,
    message: String,
    filename: String,
    location: RuffRow,
}

#[derive(Deserialize)]
struct RuffRow {
    row: usize,
}

fn run_ruff(opts: &Options, config: &str, files: &[String]) -> Vec<Finding> {
    let mut args: Vec<String> = vec![
        "check".into(),
        "--config".into(),
        config.into(),
        "--output-format".into(),
        "json".into(),
        "--no-cache".into(),
    ];
    args.extend(files.iter().cloned());
    let Some(out) = stdout_of(opts, opts.ruff_bin(), &args) else {
        return vec![];
    };
    let Ok(diags) = serde_json::from_str::<Vec<RuffDiag>>(&out) else {
        return vec![];
    };
    diags
        .into_iter()
        .map(|d| Finding {
            file: d.filename,
            line: d.location.row,
            rule: format!("ruff:{}", d.code),
            severity: Severity::Advisory,
            message: d.message,
            source: "ruff",
        })
        .collect()
}

#[derive(Deserialize)]
struct OxlintOutput {
    diagnostics: Vec<OxlintDiag>,
}

#[derive(Deserialize)]
struct OxlintDiag {
    code: String,
    message: String,
    filename: String,
    #[serde(default)]
    labels: Vec<OxlintLabel>,
}

#[derive(Deserialize)]
struct OxlintLabel {
    span: OxlintSpan,
}

#[derive(Deserialize)]
struct OxlintSpan {
    line: usize,
}

fn run_oxlint(opts: &Options, config: &str, files: &[String]) -> Vec<Finding> {
    let mut args: Vec<String> = vec![
        "-c".into(),
        config.into(),
        "-f".into(),
        "json".into(),
        "--quiet".into(),
    ];
    args.extend(files.iter().cloned());
    let Some(out) = stdout_of(opts, opts.oxlint_bin(), &args) else {
        return vec![];
    };
    let Ok(parsed) = serde_json::from_str::<OxlintOutput>(&out) else {
        return vec![];
    };
    let cwd = std::env::current_dir().unwrap_or_default();
    parsed
        .diagnostics
        .into_iter()
        .map(|d| {
            let line = d.labels.first().map(|l| l.span.line).unwrap_or(0);
            let file = if Path::new(&d.filename).is_absolute() {
                d.filename
            } else {
                cwd.join(&d.filename).to_string_lossy().into_owned()
            };
            Finding {
                file,
                line,
                rule: format!("oxlint:{}", d.code),
                severity: Severity::Advisory,
                message: d.message,
                source: "oxlint",
            }
        })
        .collect()
}

/// ValidateBaseline errors when the baseline directory is missing a config.
pub fn validate_baseline(opts: &Options) -> Result<(), String> {
    for name in ["ruff.toml", "oxlint.json"] {
        let p = format!("{}/{}", opts.baseline_dir(), name);
        if !Path::new(&p).is_file() {
            return Err(format!(
                "baseline {name} not found in {} (set HARNESS_LINTERS_DIR or run install.sh)",
                opts.baseline_dir()
            ));
        }
    }
    Ok(())
}

pub fn baseline_path(opts: &Options, name: &str) -> String {
    format!("{}/{}", opts.baseline_dir(), name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const RUFF_OUTPUT: &str = r#"[
  {
    "code": "B006",
    "message": "Mutable default argument of type `list` is forbidden",
    "filename": "/repo/src/bad.py",
    "location": {"row": 3, "column": 9}
  }
]"#;

    const OXLINT_OUTPUT: &str = r#"{
  "diagnostics": [
    {
      "code": "eslint(no-unused-vars)",
      "message": "Variable 'unused' is declared but never used.",
      "severity": "warning",
      "filename": "src/bad.ts",
      "labels": [{"label": "'unused' is declared here", "span": {"offset": 21, "length": 6, "line": 1, "column": 22}}]
    }
  ]
}"#;

    fn fake_linter(name: &str, output: &str) -> (String, String) {
        let bin_dir = std::env::temp_dir().join(format!("hc-{}-{}", name, std::process::id()));
        fs::create_dir_all(&bin_dir).unwrap();
        let record = bin_dir.join("args.log");
        let script = format!(
            "#!/bin/sh\necho \"$@\" >> {}\ncat <<'JSON'\n{output}\nJSON\nexit 0\n",
            record.display()
        );
        let bin = bin_dir.join(name);
        fs::write(&bin, script).unwrap();
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).unwrap();
        (
            bin.to_string_lossy().into(),
            record.to_string_lossy().into(),
        )
    }

    fn read_record(record: &str) -> String {
        fs::read_to_string(record).expect("linter was never invoked")
    }

    #[test]
    fn repo_config_wins_over_baseline() {
        let repo = std::env::temp_dir().join("hc-resolve-repo");
        fs::create_dir_all(repo.join("src")).unwrap();
        fs::write(repo.join("ruff.toml"), "line-length = 100\n").unwrap();
        fs::write(repo.join("src/bad.py"), "def f(x=[]):\n    pass\n").unwrap();
        let (bin, record) = fake_linter("ruff", RUFF_OUTPUT);
        let opts = Options {
            ruff_bin: Some(bin),
            baseline_dir: Some(std::env::temp_dir().to_string_lossy().into()),
            ..Default::default()
        };
        let got = linter_files(
            &detect(repo.to_str().unwrap()),
            &opts,
            &[repo.join("src/bad.py").to_string_lossy().into()],
        );
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].rule, "ruff:B006");
        assert_eq!(got[0].severity, Severity::Advisory);
        assert!(
            read_record(&record).contains(&format!("--config {}/ruff.toml", repo.display())),
            "repo config must win"
        );
    }

    #[test]
    fn baseline_used_when_repo_has_no_config() {
        let repo = std::env::temp_dir().join("hc-resolve-baseline");
        fs::create_dir_all(&repo).unwrap();
        fs::write(repo.join("bad.py"), "def f(x=[]):\n    pass\n").unwrap();
        let baseline = std::env::temp_dir().join("hc-resolve-baseline-dir");
        fs::create_dir_all(&baseline).unwrap();
        fs::write(baseline.join("ruff.toml"), "").unwrap();
        let (bin, record) = fake_linter("ruff", RUFF_OUTPUT);
        let opts = Options {
            ruff_bin: Some(bin),
            baseline_dir: Some(baseline.to_string_lossy().into()),
            ..Default::default()
        };
        let got = linter_files(
            &detect(repo.to_str().unwrap()),
            &opts,
            &[repo.join("bad.py").to_string_lossy().into()],
        );
        assert_eq!(got.len(), 1);
        assert!(
            read_record(&record).contains(&format!("{}/ruff.toml", baseline.display())),
            "baseline must apply"
        );
    }

    #[test]
    fn pyproject_without_ruff_section_is_not_config() {
        let repo = std::env::temp_dir().join("hc-resolve-pyproj");
        fs::create_dir_all(&repo).unwrap();
        fs::write(repo.join("pyproject.toml"), "[project]\nname = \"x\"\n").unwrap();
        let baseline = std::env::temp_dir().join("hc-resolve-pyproj-dir");
        fs::create_dir_all(&baseline).unwrap();
        fs::write(baseline.join("ruff.toml"), "").unwrap();
        let (bin, record) = fake_linter("ruff", "[]");
        let opts = Options {
            ruff_bin: Some(bin),
            baseline_dir: Some(baseline.to_string_lossy().into()),
            ..Default::default()
        };
        linter_files(
            &detect(repo.to_str().unwrap()),
            &opts,
            &[repo.join("main.py").to_string_lossy().into()],
        );
        assert!(read_record(&record).contains(&format!("{}/ruff.toml", baseline.display())));
    }

    #[test]
    fn missing_tool_degrades_silently() {
        let repo = std::env::temp_dir().join("hc-resolve-notool");
        fs::create_dir_all(&repo).unwrap();
        fs::write(repo.join("bad.py"), "x = 1\n").unwrap();
        let opts = Options {
            ruff_bin: Some("/no/such/ruff".into()),
            ..Default::default()
        };
        let got = linter_files(
            &detect(repo.to_str().unwrap()),
            &opts,
            &[repo.join("bad.py").to_string_lossy().into()],
        );
        assert!(got.is_empty());
    }

    #[test]
    fn oxlint_diagnostics_parsed() {
        let repo = std::env::temp_dir().join("hc-resolve-ox");
        fs::create_dir_all(repo.join("src")).unwrap();
        let bad = repo.join("src/bad.ts");
        fs::write(&bad, "function f() { const unused = 1; }\nf();\n").unwrap();
        let (bin, _) = fake_linter("oxlint", OXLINT_OUTPUT);
        let prev_cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(&repo).unwrap();
        let opts = Options {
            oxlint_bin: Some(bin),
            baseline_dir: Some(std::env::temp_dir().to_string_lossy().into()),
            ..Default::default()
        };
        let got = linter_files(
            &detect(repo.to_str().unwrap()),
            &opts,
            &[bad.to_string_lossy().into()],
        );
        std::env::set_current_dir(prev_cwd).unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].rule, "oxlint:eslint(no-unused-vars)");
        assert_eq!(got[0].line, 1);
        assert_eq!(got[0].file, bad.to_string_lossy());
    }

    #[test]
    fn detect_finds_oxlint_config() {
        let repo = std::env::temp_dir().join("hc-resolve-detect");
        fs::create_dir_all(&repo).unwrap();
        fs::write(repo.join(".oxlintrc.json"), "{\"rules\":{}}").unwrap();
        assert!(detect(repo.to_str().unwrap()).oxlint_config.is_some());
    }
}
