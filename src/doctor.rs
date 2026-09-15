use serde::Serialize;

use crate::resolve;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Ok,
    Warn,
    Info,
}

#[derive(Debug, Serialize)]
pub struct Check {
    pub name: String,
    pub status: Status,
    pub detail: String,
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub root: String,
    pub checks: Vec<Check>,
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::Ok => write!(f, "ok"),
            Status::Warn => write!(f, "warn"),
            Status::Info => write!(f, "info"),
        }
    }
}

impl std::fmt::Display for Report {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "harness-check doctor: {}", self.root)?;
        for c in &self.checks {
            writeln!(
                f,
                "{:<4} {}: {}",
                format!("{}", c.status).to_uppercase(),
                c.name,
                c.detail
            )?;
        }
        Ok(())
    }
}

pub fn run(root: &str, opts: &resolve::Options) -> Report {
    let cfgs = resolve::detect(root);
    let mut checks = vec![];
    checks.push(baseline_installed(opts));
    checks.extend(ruff_checks(&cfgs, opts));
    checks.extend(oxlint_checks(&cfgs, opts));
    checks.push(tool_check("ruff", opts.ruff_bin().to_string()));
    checks.push(tool_check("oxlint", opts.oxlint_bin().to_string()));
    Report {
        root: root.into(),
        checks,
    }
}

fn baseline_installed(opts: &resolve::Options) -> Check {
    match resolve::validate_baseline(opts) {
        Ok(()) => Check {
            name: "harness-baseline".into(),
            status: Status::Ok,
            detail: opts.baseline_dir(),
        },
        Err(e) => Check {
            name: "harness-baseline".into(),
            status: Status::Warn,
            detail: e,
        },
    }
}

fn ruff_checks(cfgs: &resolve::RepoConfigs, opts: &resolve::Options) -> Vec<Check> {
    let Some(config) = &cfgs.ruff_config else {
        return vec![Check {
            name: "ruff-config".into(),
            status: Status::Warn,
            detail: "no repo ruff config found; the harness baseline applies at edit time. Extend it to own this repo's policy.".into(),
        }];
    };
    let mut out = vec![];
    if extends_baseline(config) {
        out.push(Check {
            name: "ruff-config".into(),
            status: Status::Ok,
            detail: "extends the harness baseline".into(),
        });
    } else {
        out.push(Check {
            name: "ruff-config".into(),
            status: Status::Warn,
            detail: "does not extend the harness baseline; add extend = \"<baseline>/ruff.toml\" to inherit updates".into(),
        });
    }
    let missing = missing_families(config, opts);
    if missing.is_empty() {
        out.push(Check {
            name: "ruff-families".into(),
            status: Status::Ok,
            detail: "covers all baseline families".into(),
        });
    } else {
        out.push(Check {
            name: "ruff-families".into(),
            status: Status::Warn,
            detail: format!(
                "baseline families missing from repo config: {}",
                missing.join(", ")
            ),
        });
    }
    out
}

fn oxlint_checks(cfgs: &resolve::RepoConfigs, _opts: &resolve::Options) -> Vec<Check> {
    let Some(config) = &cfgs.oxlint_config else {
        return vec![Check {
            name: "oxlint-config".into(),
            status: Status::Warn,
            detail: "no .oxlintrc.json found; the harness baseline applies at edit time. Extend it to own this repo's policy.".into(),
        }];
    };
    if extends_baseline(config) {
        vec![Check {
            name: "oxlint-config".into(),
            status: Status::Ok,
            detail: "extends the harness baseline".into(),
        }]
    } else {
        vec![Check {
            name: "oxlint-config".into(),
            status: Status::Warn,
            detail: "does not extend the harness baseline; add it under extends to inherit updates"
                .into(),
        }]
    }
}

fn tool_check(name: &str, bin: String) -> Check {
    if !crate::resolve::which_bin(&bin) {
        return Check {
            name: name.into(),
            status: Status::Info,
            detail: format!("{bin} not on PATH; edit-time {name} checks are skipped"),
        };
    }
    let detail = std::process::Command::new(&bin)
        .arg("--version")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| bin.clone());
    Check {
        name: name.into(),
        status: Status::Ok,
        detail,
    }
}

fn extends_baseline(config_path: &str) -> bool {
    std::fs::read_to_string(config_path)
        .map(|s| s.contains("lazar-harness/linters") || s.contains("lint-baselines"))
        .unwrap_or(false)
}

// parseSelects is a best-effort reader for the ruff `select` list. It strips
// TOML line comments first, because real configs annotate every family, then
// reads the plain quoted-name subset.
fn parse_selects(config_path: &str) -> Vec<String> {
    let Ok(content) = std::fs::read_to_string(config_path) else {
        return vec![];
    };
    let no_comments: String = content
        .lines()
        .map(|l| match l.find('#') {
            Some(idx) => &l[..idx],
            None => l,
        })
        .collect::<Vec<_>>()
        .join(" ");
    let Some(start) = no_comments.find("select") else {
        return vec![];
    };
    let rest = &no_comments[start..];
    let Some(open) = rest.find('[') else {
        return vec![];
    };
    let Some(close) = rest[open..].find(']') else {
        return vec![];
    };
    rest[open + 1..open + close]
        .split(',')
        .map(|p| p.trim().trim_matches(|c| c == '"' || c == '\'').to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn missing_families(repo_config: &str, opts: &resolve::Options) -> Vec<String> {
    let repo = parse_selects(repo_config);
    if repo.is_empty() {
        return vec![];
    }
    let baseline = parse_selects(&resolve::baseline_path(opts, "ruff.toml"));
    baseline.into_iter().filter(|f| !repo.contains(f)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    const BASELINE: &str =
        "select = [\"E\", \"F\", \"I\", \"UP\", \"B\", \"SIM\", \"RET\", \"S\", \"RUF\"]\n";

    fn write(path: &str, content: &str) {
        fs::create_dir_all(Path::new(path).parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    fn unique(tag: &str) -> String {
        let thread = std::thread::current()
            .name()
            .unwrap_or("t")
            .replace("::", "-");
        std::env::temp_dir()
            .join(format!("hc-{}-{}-{}", tag, std::process::id(), thread))
            .to_string_lossy()
            .into_owned()
    }

    fn baseline_dir() -> String {
        let dir = std::path::PathBuf::from(unique("doctor-baseline"));
        write(&format!("{}/ruff.toml", dir.display()), BASELINE);
        write(&format!("{}/oxlint.json", dir.display()), "{}");
        dir.to_string_lossy().into()
    }

    fn has(report: &Report, name: &str, status: Status, part: &str) -> bool {
        report
            .checks
            .iter()
            .any(|c| c.name == name && c.status == status && c.detail.contains(part))
    }

    #[test]
    fn no_config_warns_with_baseline_message() {
        let repo = std::env::temp_dir().join("hc-doctor-noconfig");
        fs::create_dir_all(&repo).unwrap();
        let report = run(
            repo.to_str().unwrap(),
            &resolve::Options {
                baseline_dir: Some(baseline_dir()),
                ..Default::default()
            },
        );
        assert!(
            has(&report, "ruff-config", Status::Warn, "baseline applies"),
            "{report:?}"
        );
    }

    #[test]
    fn extending_repo_passes() {
        let repo = std::env::temp_dir().join("hc-doctor-extends");
        write(
            &format!("{}/ruff.toml", repo.display()),
            &format!("extend = \"/home/x/.config/lazar-harness/linters/ruff.toml\"\n{BASELINE}"),
        );
        let report = run(
            repo.to_str().unwrap(),
            &resolve::Options {
                baseline_dir: Some(baseline_dir()),
                ..Default::default()
            },
        );
        assert!(
            has(&report, "ruff-config", Status::Ok, "extends"),
            "{report:?}"
        );
        assert!(
            has(&report, "ruff-families", Status::Ok, "covers all"),
            "{report:?}"
        );
    }

    #[test]
    fn missing_families_reported() {
        let repo = std::env::temp_dir().join("hc-doctor-missing");
        write(
            &format!("{}/ruff.toml", repo.display()),
            &format!(
                "extend = \"~/.config/lazar-harness/linters/ruff.toml\"\nselect = [\"E\", \"F\"]\n"
            ),
        );
        let report = run(
            repo.to_str().unwrap(),
            &resolve::Options {
                baseline_dir: Some(baseline_dir()),
                ..Default::default()
            },
        );
        assert!(
            has(
                &report,
                "ruff-families",
                Status::Warn,
                "UP, B, SIM, RET, S, RUF"
            ),
            "{report:?}"
        );
    }

    #[test]
    fn own_config_not_extending_warns() {
        let repo = std::env::temp_dir().join("hc-doctor-own");
        write(&format!("{}/ruff.toml", repo.display()), BASELINE);
        let report = run(
            repo.to_str().unwrap(),
            &resolve::Options {
                baseline_dir: Some(baseline_dir()),
                ..Default::default()
            },
        );
        assert!(
            has(&report, "ruff-config", Status::Warn, "does not extend"),
            "{report:?}"
        );
    }
}
