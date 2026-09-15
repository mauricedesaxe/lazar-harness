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
            id: "rust-dbg".into(),
            exts: exts(&[".rs"]),
            pattern: r"\bdbg!\(".into(),
            skip_tests: true,
            severity: Severity::Blocking,
            message: "dbg! is leftover debugging. Print deliberately or remove it.".into(),
            compiled: None,
        },
        Rule {
            id: "trailing-whitespace".into(),
            exts: exts(&[".py", ".ts", ".tsx", ".js", ".jsx", ".go", ".rs"]),
            pattern: r"[ \t]+$".into(),
            skip_tests: false,
            severity: Severity::Advisory,
            message: "Trailing whitespace.".into(),
            compiled: None,
        },
        Rule {
            id: "todo-no-ref".into(),
            exts: exts(&[".py", ".ts", ".tsx", ".js", ".jsx", ".go", ".rs"]),
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

const CODE_EXTS: &[&str] = &[".py", ".ts", ".tsx", ".js", ".jsx", ".go", ".rs"];

pub struct Engine {
    pub rules: Vec<Rule>,
    /// File length where the nudge starts. Tests are exempt.
    pub advisory_len: usize,
    /// File length that blocks. The hard limit, also test-exempt.
    pub blocking_len: usize,
    /// Function length tiers. Tests are exempt.
    pub fn_advisory: usize,
    pub fn_blocking: usize,
    /// Class-like (struct, impl, class, interface) length tiers. Tests exempt.
    pub class_advisory: usize,
    pub class_blocking: usize,
}

impl Default for Engine {
    fn default() -> Self {
        Engine {
            rules: default_rules(),
            advisory_len: 500,
            blocking_len: 1000,
            fn_advisory: 60,
            fn_blocking: 100,
            class_advisory: 200,
            class_blocking: 400,
        }
    }
}

/// The severity and limit a span of `lines` crosses, or None when it is
/// within both tiers. Shared by every length rule so the encourage/enforce
/// split stays one decision.
fn tier(lines: usize, advisory: usize, blocking: usize) -> Option<(Severity, usize)> {
    if blocking > 0 && lines > blocking {
        Some((Severity::Blocking, blocking))
    } else if advisory > 0 && lines > advisory {
        Some((Severity::Advisory, advisory))
    } else {
        None
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
        let lines = text.lines().count();
        if lines > 0 && !testing && CODE_EXTS.contains(&ext.as_str()) {
            if let Some((severity, limit)) = tier(lines, self.advisory_len, self.blocking_len) {
                out.push(Finding {
                    file: file.into(),
                    line: lines,
                    rule: FILE_LENGTH_RULE.into(),
                    severity,
                    message: format!("File is over {limit} lines. Consider splitting it."),
                    source: "harness",
                });
            }
            out.extend(self.span_lengths(&text, &ext, file));
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

const FUNCTION_LENGTH_RULE: &str = "function-length";
const CLASS_LENGTH_RULE: &str = "class-length";

#[derive(Clone, Copy)]
enum SpanKind {
    Function,
    Class,
}

struct Span {
    decl_line: usize,
    end_line: usize,
    kind: SpanKind,
}

impl Span {
    fn length(&self) -> usize {
        self.end_line - self.decl_line + 1
    }

    fn rule(&self) -> &'static str {
        match self.kind {
            SpanKind::Function => FUNCTION_LENGTH_RULE,
            SpanKind::Class => CLASS_LENGTH_RULE,
        }
    }

    fn tiers(&self, e: &Engine) -> (usize, usize) {
        match self.kind {
            SpanKind::Function => (e.fn_advisory, e.fn_blocking),
            SpanKind::Class => (e.class_advisory, e.class_blocking),
        }
    }
}

fn function_decl(ext: &str) -> Option<Regex> {
    let pattern = match ext {
        ".rs" => r"^\s*(pub(\(\w+\))? )?(async )?fn\b",
        ".go" => r"^func\b",
        ".py" => r"^\s*(async )?def\b",
        _ => r"^\s*(export )?(async )?function\s+\w",
    };
    Regex::new(pattern).ok()
}

fn class_decl(ext: &str) -> Option<Regex> {
    let pattern = match ext {
        ".rs" => r"^\s*(pub(\(\w+\))? )?(struct|enum|trait|impl)\b",
        ".go" => r"^type\s+\w+\s+(struct|interface)\b",
        ".py" => r"^\s*class\b",
        _ => r"^\s*(export )?(abstract )?class\s+\w",
    };
    Regex::new(pattern).ok()
}

/// Brace languages share one walker: a declaration opens a span at the depth
/// its body starts, and the span closes when the depth drops back. A
/// declaration whose brace sits on a later line (wrapped signatures) is held
/// open for up to ten lines. Braces inside string literals can wobble the
/// depth mid-span; balanced strings net out, so spans still end correctly.
fn brace_spans(text: &str, fn_re: &Regex, class_re: &Regex) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut open: Vec<(usize, usize, SpanKind)> = Vec::new();
    let mut awaiting: Option<(usize, SpanKind)> = None;
    let mut depth = 0usize;
    for (i, line) in text.lines().enumerate() {
        let line_no = i + 1;
        if let Some(kind) = if fn_re.is_match(line) {
            Some(SpanKind::Function)
        } else if class_re.is_match(line) {
            Some(SpanKind::Class)
        } else {
            None
        } {
            awaiting = Some((line_no, kind));
        }
        for ch in line.chars() {
            match ch {
                '{' => {
                    depth += 1;
                    if let Some((decl, kind)) = awaiting.take() {
                        open.push((decl, depth, kind));
                    }
                }
                '}' => {
                    depth = depth.saturating_sub(1);
                    if let Some(pos) = open.iter().rposition(|(_, d, _)| *d == depth + 1) {
                        let (decl, _, kind) = open.remove(pos);
                        spans.push(Span {
                            decl_line: decl,
                            end_line: line_no,
                            kind,
                        });
                    }
                }
                _ => {}
            }
        }
        if let Some((decl, _)) = awaiting
            && line_no - decl >= 10
        {
            awaiting = None;
        }
    }
    spans
}

/// Python spans close when a non-blank, non-comment line returns to the
/// declaration's indentation or shallower.
fn indent_spans(text: &str, fn_re: &Regex, class_re: &Regex) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut open: Vec<(usize, SpanKind, usize)> = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line_no = i + 1;
        let trimmed = line.trim_end();
        if trimmed.is_empty() || trimmed.trim_start().starts_with('#') {
            continue;
        }
        let indent = trimmed.len() - trimmed.trim_start().len();
        while let Some(&(decl, kind, decl_indent)) = open.last() {
            if indent <= decl_indent {
                spans.push(Span {
                    decl_line: decl,
                    end_line: line_no - 1,
                    kind,
                });
                open.pop();
            } else {
                break;
            }
        }
        let kind = if fn_re.is_match(line) {
            Some(SpanKind::Function)
        } else if class_re.is_match(line) {
            Some(SpanKind::Class)
        } else {
            None
        };
        if let Some(kind) = kind {
            open.push((line_no, kind, indent));
        }
    }
    for (decl, kind, _) in open {
        spans.push(Span {
            decl_line: decl,
            end_line: text.lines().count(),
            kind,
        });
    }
    spans
}

impl Engine {
    fn span_lengths(&self, text: &str, ext: &str, file: &str) -> Vec<Finding> {
        let (Some(fn_re), Some(class_re)) = (function_decl(ext), class_decl(ext)) else {
            return vec![];
        };
        let spans = if ext == ".py" {
            indent_spans(text, &fn_re, &class_re)
        } else {
            brace_spans(text, &fn_re, &class_re)
        };
        let mut out = Vec::new();
        for span in spans {
            let (advisory, blocking) = span.tiers(self);
            if let Some((severity, limit)) = tier(span.length(), advisory, blocking) {
                let kind = match span.kind {
                    SpanKind::Function => "function",
                    SpanKind::Class => "class",
                };
                out.push(Finding {
                    file: file.into(),
                    line: span.decl_line,
                    rule: span.rule().into(),
                    severity,
                    message: format!(
                        "{kind} is {} lines, over the {limit} limit. Extract or split it.",
                        span.length()
                    ),
                    source: "harness",
                });
            }
        }
        out
    }
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
        fs::write(&long, "x = 1\n".repeat(501)).unwrap();
        let hit = findings_for(&[long.to_string_lossy().into()], FILE_LENGTH_RULE);
        assert_eq!(hit.len(), 1);
        assert_eq!(hit[0].severity, Severity::Advisory, "{hit:?}");
    }

    #[test]
    fn file_length_blocking_past_hard_limit() {
        let dir = std::env::temp_dir().join("hc-file-len-block");
        fs::create_dir_all(&dir).unwrap();
        let huge = dir.join("huge.py");
        fs::write(&huge, "x = 1\n".repeat(1001)).unwrap();
        let hit = findings_for(&[huge.to_string_lossy().into()], FILE_LENGTH_RULE);
        assert_eq!(hit.len(), 1);
        assert_eq!(hit[0].severity, Severity::Blocking, "{hit:?}");
    }

    #[test]
    fn rust_dbg_flags_and_println_does_not() {
        let hit = findings_for(&[fixture("bad.rs")], "rust-dbg");
        assert_eq!(hit.len(), 1, "{hit:?}");
        assert_eq!(hit[0].line, 2);
        let prints = findings_for(&[fixture("bad.rs")], "debug-print");
        assert!(
            prints.is_empty(),
            "println is deliberate CLI output: {prints:?}"
        );
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

#[cfg(test)]
mod span_tests {
    use super::*;

    fn span_findings(files: &[String], rule: &str) -> Vec<Finding> {
        Engine::default()
            .run(files)
            .into_iter()
            .filter(|f| f.rule == rule)
            .collect()
    }

    fn temp_rs(name: &str, body: String) -> String {
        let dir = std::env::temp_dir().join("hc-span-tests");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        fs::write(&path, body).unwrap();
        path.to_string_lossy().into()
    }

    #[test]
    fn oversized_rust_function_blocks() {
        let body = format!("fn big() {{\n{}\n}}\n", "    x();\n".repeat(110));
        let file = temp_rs("big_fn.rs", body);
        let hit = span_findings(&[file], FUNCTION_LENGTH_RULE);
        assert_eq!(hit.len(), 1, "{hit:?}");
        assert_eq!(hit[0].severity, Severity::Blocking);
        assert_eq!(hit[0].line, 1);
    }

    #[test]
    fn oversized_ts_function_advises() {
        let body = format!("function medium() {{\n{}\n}}\n", "  step();\n".repeat(70));
        let file = temp_rs("medium.ts", body);
        let hit = span_findings(&[file], FUNCTION_LENGTH_RULE);
        assert_eq!(hit.len(), 1, "{hit:?}");
        assert_eq!(hit[0].severity, Severity::Advisory);
    }

    #[test]
    fn oversized_python_class_blocks() {
        let body = format!(
            "class huge:\n{}\n",
            "    def method(self):\n        pass\n".repeat(250)
        );
        let file = temp_rs("huge.py", body);
        let hit = span_findings(&[file], CLASS_LENGTH_RULE);
        assert_eq!(hit.len(), 1, "{hit:?}");
        assert_eq!(hit[0].severity, Severity::Blocking);
    }

    #[test]
    fn nested_and_sequential_functions_span_correctly() {
        let body =
            "fn a() {\n    if true {\n        step();\n    }\n}\n\nfn b() {\n    step();\n}\n";
        let file = temp_rs("nested.rs", body.into());
        assert!(span_findings(&[file], FUNCTION_LENGTH_RULE).is_empty());
    }

    #[test]
    fn tests_directory_is_exempt() {
        let dir = std::env::temp_dir().join("hc-span-tests-t");
        fs::create_dir_all(&dir).unwrap();
        let subdir = dir.join("tests");
        fs::create_dir_all(&subdir).unwrap();
        let path = subdir.join("long.rs");
        fs::write(
            &path,
            format!("fn big() {{\n{}\n}}\n", "    x();\n".repeat(110)),
        )
        .unwrap();
        assert!(span_findings(&[path.to_string_lossy().into()], FUNCTION_LENGTH_RULE).is_empty());
    }
}
