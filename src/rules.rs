use std::fs;
use std::path::Path;

use regex::Regex;
use serde::Deserialize;

use crate::findings::{Finding, Severity};

#[derive(Debug, Deserialize, Clone)]
pub struct Rule {
    pub id: String,
    #[serde(default)]
    pub exts: Vec<String>,
    pub pattern: String,
    #[serde(default)]
    pub skip_tests: bool,
    pub severity: Severity,
    pub message: String,
    #[serde(skip)]
    compiled: Option<Regex>,
}

impl Rule {
    fn compile(&mut self) {
        self.compiled = Regex::new(&self.pattern).ok();
    }

    fn matches(&self, text: &str) -> bool {
        self.compiled.as_ref().is_some_and(|re| re.is_match(text))
    }
}

pub const FILE_LENGTH_RULE: &str = "file-length";

pub fn default_rules() -> Vec<Rule> {
    let raw = vec![
        Rule {
            id: "debug-print".into(),
            exts: exts(&[".py", ".ts", ".tsx", ".js", ".jsx"]),
            pattern: r"\bprint\(|\bconsole\.log\(".into(),
            skip_tests: true,
            severity: Severity::Blocking,
            message: "Debug print in source. Remove it or switch to a logger.".into(),
            compiled: None,
        },
        Rule {
            id: "bare-except".into(),
            exts: exts(&[".py"]),
            pattern: r"^\s*except\s*:\s*$".into(),
            skip_tests: true,
            severity: Severity::Blocking,
            message: "Bare except swallows every error. Catch the specific exception or let it propagate.".into(),
            compiled: None,
        },
        Rule {
            id: "trailing-whitespace".into(),
            exts: exts(&[".py", ".ts", ".tsx", ".js", ".jsx", ".go"]),
            pattern: r"[ \t]+$".into(),
            skip_tests: false,
            severity: Severity::Advisory,
            message: "Trailing whitespace.".into(),
            compiled: None,
        },
        Rule {
            id: "todo-no-ref".into(),
            exts: exts(&[".py", ".ts", ".tsx", ".js", ".jsx", ".go"]),
            pattern: r"\b(TODO|FIXME)\b".into(),
            skip_tests: false,
            severity: Severity::Advisory,
            message: "TODO or FIXME without an issue reference. Link the work or finish it.".into(),
            compiled: None,
        },
    ];
    let mut rules = raw;
    for r in &mut rules {
        r.compile();
    }
    rules
}

fn exts(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

pub fn excluded(path: &str) -> bool {
    lazy_excluded().is_match(path)
}

fn lazy_excluded() -> &'static Regex {
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(^|/)(node_modules|\.venv|venv|vendor|dist|build|out|__pycache__|__marimo__|\.beads|\.git|\.jj|fixtures|testdata)(/|/\.|$)|\.lock$|\.min\.(js|jsx|css)$|\.d\.ts$|\.snap$|\.pb\.go$|_pb2\.py$",
        )
        .unwrap()
    })
}

fn is_test(path: &str) -> bool {
    lazy_test().is_match(path)
}

fn lazy_test() -> &'static Regex {
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(^|/)(tests?|__tests__)(/|$)|_test\.go$|\.test\.[jt]sx?$|\.spec\.[jt]sx?$|_test\.py$|^test_[^/]*\.py$").unwrap()
    })
}

fn issue_ref() -> &'static Regex {
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| Regex::new(r"#\d+|[A-Za-z][A-Za-z0-9]*-\w*\d[\w-]*|https?://").unwrap())
}

const CODE_EXTS: &[&str] = &[".py", ".ts", ".tsx", ".js", ".jsx", ".go"];

pub struct Engine {
    pub rules: Vec<Rule>,
    pub max_file_len: usize,
}

impl Default for Engine {
    fn default() -> Self {
        Engine {
            rules: default_rules(),
            max_file_len: 800,
        }
    }
}

impl Engine {
    pub fn run(&self, files: &[String]) -> Vec<Finding> {
        files.iter().flat_map(|file| self.scan(file)).collect()
    }

    fn scan(&self, file: &str) -> Vec<Finding> {
        let ext = extension(file);
        let Ok(content) = fs::read(file) else {
            return vec![];
        };
        if is_binary(&content) {
            return vec![];
        }
        let text = String::from_utf8_lossy(&content);
        let testing = is_test(file);

        let mut out = vec![];
        for (i, line) in text.lines().enumerate() {
            for r in &self.rules {
                if !r.exts.is_empty() && !r.exts.iter().any(|e| e == &ext) {
                    continue;
                }
                if r.skip_tests && testing {
                    continue;
                }
                if r.id == "todo-no-ref" && issue_ref().is_match(line) {
                    continue;
                }
                if r.matches(line) {
                    out.push(Finding {
                        file: file.into(),
                        line: i + 1,
                        rule: r.id.clone(),
                        severity: r.severity,
                        message: r.message.clone(),
                        source: "harness",
                    });
                }
            }
        }
        if self.max_file_len > 0
            && text.lines().count() > self.max_file_len
            && !testing
            && CODE_EXTS.contains(&ext.as_str())
        {
            out.push(Finding {
                file: file.into(),
                line: text.lines().count(),
                rule: FILE_LENGTH_RULE.into(),
                severity: Severity::Advisory,
                message: format!(
                    "File is over {} lines. Consider splitting it.",
                    self.max_file_len
                ),
                source: "harness",
            });
        }
        out
    }
}

fn extension(path: &str) -> String {
    Path::new(path)
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy().to_lowercase()))
        .unwrap_or_default()
}

fn is_binary(bytes: &[u8]) -> bool {
    let head = if bytes.len() > 8000 {
        &bytes[..8000]
    } else {
        bytes
    };
    head.contains(&0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::findings::Severity;

    fn fixture(name: &str) -> String {
        format!("src/rules/testdata/{name}")
    }

    fn findings_for(files: &[String], rule: &str) -> Vec<Finding> {
        Engine::default()
            .run(files)
            .into_iter()
            .filter(|f| f.rule == rule)
            .collect()
    }

    #[test]
    fn debug_print_flags_source_not_tests() {
        let hit = findings_for(&[fixture("bad.py"), fixture("bad.ts")], "debug-print");
        assert_eq!(hit.len(), 2, "{hit:?}");
        let miss = findings_for(&[fixture("tests/test_bad.py")], "debug-print");
        assert!(miss.is_empty(), "{miss:?}");
    }

    #[test]
    fn bare_except_flags() {
        let hit = findings_for(&[fixture("bad.py")], "bare-except");
        assert_eq!(hit.len(), 1);
        assert_eq!(hit[0].line, 8);
    }

    #[test]
    fn todo_without_issue_ref_flags() {
        let hit = findings_for(&[fixture("bad.py")], "todo-no-ref");
        assert_eq!(hit.len(), 1);
        assert_eq!(hit[0].line, 12);
        let clean = findings_for(&[fixture("good.py")], "todo-no-ref");
        assert!(clean.is_empty(), "{clean:?}");
    }

    #[test]
    fn excluded_paths_recognized() {
        assert!(excluded("a/vendor/dep.py"));
        assert!(excluded("a/node_modules/b.js"));
        assert!(excluded("pkg/x.lock"));
        assert!(!excluded("pkg/main.go"));
    }

    #[test]
    fn severity_split() {
        let all = Engine::default().run(&[fixture("bad.py"), fixture("bad.ts")]);
        let result = crate::findings::Outcome { findings: all };
        assert!(
            result
                .findings
                .iter()
                .any(|f| f.severity == Severity::Blocking)
        );
        assert_eq!(result.exit_code(), 2);
        let advisory_only = crate::findings::Outcome {
            findings: vec![Finding {
                file: "x".into(),
                line: 1,
                rule: "r".into(),
                severity: Severity::Advisory,
                message: "m".into(),
                source: "harness",
            }],
        };
        assert_eq!(advisory_only.exit_code(), 1);
    }

    #[test]
    fn file_length_advisory() {
        let dir = std::env::temp_dir().join("hc-file-len-test");
        fs::create_dir_all(&dir).unwrap();
        let long = dir.join("long.py");
        fs::write(&long, "x = 1\n".repeat(801)).unwrap();
        let hit = findings_for(&[long.to_string_lossy().into()], FILE_LENGTH_RULE);
        assert_eq!(hit.len(), 1, "{hit:?}");
    }

    #[test]
    fn binary_and_missing_files_skipped() {
        let dir = std::env::temp_dir().join("hc-binary-test");
        fs::create_dir_all(&dir).unwrap();
        let bin = dir.join("bin.py");
        fs::write(&bin, b"print(\x00\x01)").unwrap();
        let missing = dir.join("missing.py").to_string_lossy().into();
        assert!(
            Engine::default()
                .run(&[bin.to_string_lossy().into(), missing])
                .is_empty()
        );
    }
}
