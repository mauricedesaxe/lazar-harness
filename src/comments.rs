use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use regex::Regex;

use crate::findings::{Finding, Severity};

#[derive(Clone, Copy, PartialEq)]
enum Lang {
    CStyle,
    JavaScript,
    Python,
}

fn lang_of(file: &str) -> Option<Lang> {
    let ext = Path::new(file)
        .extension()?
        .to_string_lossy()
        .to_lowercase();
    Some(match ext.as_str() {
        "rs" => Lang::CStyle,
        "ts" | "tsx" | "mts" | "cts" | "js" | "jsx" | "mjs" | "cjs" => Lang::JavaScript,
        "go" => Lang::CStyle,
        "py" | "pyi" => Lang::Python,
        _ => return None,
    })
}

#[derive(Debug)]
struct Violation {
    line: usize,
    snippet: String,
}

/// lintPair reports prose comments present in the new text but not in the
/// old one, as a multiset difference: a comment that moved or repeats is not
/// re-flagged.
fn lint_pair(file: &str, old_text: &str, new_text: &str) -> Vec<Violation> {
    let old = tokens_for(file, old_text);
    let new = tokens_for(file, new_text);
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for token in &old {
        *counts.entry(token).or_default() += 1;
    }
    new.into_iter()
        .filter(|token| match counts.get_mut(token.as_str()) {
            Some(count) if *count > 0 => {
                *count -= 1;
                false
            }
            _ => true,
        })
        .map(|token| token_violation(new_text, &token))
        .collect()
}

fn token_violation(text: &str, token: &str) -> Violation {
    let needle = normalized_first_line(token);
    let line = text
        .lines()
        .position(|l| normalized_first_line(l).contains(&needle))
        .map(|i| i + 1)
        .unwrap_or(1);
    Violation {
        line,
        snippet: token.chars().take(100).collect(),
    }
}

fn normalized_first_line(line: &str) -> String {
    line.trim().to_string()
}

struct Comment {
    line: usize,
    end_line: usize,
    snippet: String,
    token: String,
    exempt: bool,
}

fn directive() -> &'static Regex {
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?i)^(?:eslint-disable\b|eslint-enable\b|prettier-ignore\b|biome-ignore\b|@ts-expect-error\b|@ts-ignore\b|@ts-nocheck\b|@ts-check\b|tslint:|deno-lint-ignore\b|v8\s+ignore\b|c8\s+ignore\b|istanbul\s+ignore\b|type:\s*ignore\b|noqa\b|pylint:|pyright:\s*ignore\b|ruff:|mypy:|fmt:\s*(?:on|off)\b|go:[a-z]+\b|@license\b|SPDX-License-Identifier\b|#pragma\b|pragma\s*:|allow\(.*\)|deny\(.*\)|cfg\(.*\)|!\[(allow|deny|cfg|warn|expect))",
        )
        .unwrap()
    })
}

fn license_re() -> &'static Regex {
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?i)(copyright|licen[cs]e|SPDX-License-Identifier|@license|all rights reserved)",
        )
        .unwrap()
    })
}

const LICENSE_HEADER_SCAN_LINES: usize = 5;

fn make_comment(line: usize, raw: &str, end_line: usize, leading: bool) -> Comment {
    let token = normalized_token(raw);
    let exempt = (line == 1 && raw.trim_start().starts_with("#!"))
        || directive().is_match(&token)
        || (leading && line <= LICENSE_HEADER_SCAN_LINES && license_re().is_match(raw));
    Comment {
        line,
        end_line,
        snippet: raw.trim().to_string(),
        token,
        exempt,
    }
}

fn normalized_token(text: &str) -> String {
    let stripped = text
        .lines()
        .map(|l| {
            let l = l.trim();
            let l = l
                .strip_prefix("///")
                .or_else(|| l.strip_prefix("//"))
                .unwrap_or(l);
            let l = l.strip_prefix('#').unwrap_or(l);
            let l = l.strip_prefix('*').unwrap_or(l);
            l.trim()
        })
        .collect::<Vec<_>>()
        .join(" ");
    collapse_spaces(&stripped)
}

fn collapse_spaces(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn scan_c_style(text: &str) -> Vec<Comment> {
    let mut comments = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0usize;
    let mut line = 1usize;
    let mut quote: Option<u8> = None;
    while i < bytes.len() {
        let ch = bytes[i];
        let next = bytes.get(i + 1).copied();
        if let Some(q) = quote {
            if ch == b'\\' && q != b'`' {
                i += 2;
                continue;
            }
            if ch == q {
                quote = None;
            }
            if ch == b'\n' {
                line += 1;
            }
            i += 1;
            continue;
        }
        match ch {
            b'\n' => line += 1,
            b'"' | b'\'' | b'`' => quote = Some(ch),
            b'/' if next == Some(b'/') => {
                let start = i;
                let start_line = line;
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
                let raw = &text[start..i.min(text.len())];
                let leading = text[..start].trim().is_empty();
                comments.push(make_comment(start_line, raw, start_line, leading));
                continue;
            }
            b'/' if next == Some(b'*') => {
                let start = i;
                let start_line = line;
                let close = text[i + 2..].find("*/").map(|p| i + 2 + p);
                i = match close {
                    Some(c) => c + 2,
                    None => text.len(),
                };
                let raw = &text[start..i.min(text.len())];
                let end_line = start_line + raw.matches('\n').count();
                let leading = text[..start].trim().is_empty();
                comments.push(make_comment(start_line, raw, end_line, leading));
                line = end_line;
            }
            _ => {}
        }
        i += 1;
    }
    comments
}

fn scan_python(text: &str) -> Vec<Comment> {
    let mut comments = Vec::new();
    let mut triple: Option<String> = None;
    for (i, raw) in text.lines().enumerate() {
        let line_no = i + 1;
        let mut chars = raw.char_indices().peekable();
        let mut in_triple = triple.clone();
        let mut hit: Option<usize> = None;
        let mut idx = 0usize;
        while idx < raw.len() {
            if let Some(t) = &in_triple {
                if let Some(close) = raw[idx..].find(t.as_str()) {
                    idx = idx + close + 3;
                    in_triple = None;
                    continue;
                }
                break;
            }
            let ch = raw[idx..].chars().next().unwrap();
            if ch == '"' || ch == '\'' {
                let trip = &raw[idx..(idx + 3).min(raw.len())];
                if trip == "\"\"\"" || trip == "'''" {
                    match raw[idx + 3..].find(trip) {
                        Some(close) => idx = idx + 3 + close + 3,
                        None => {
                            in_triple = Some(trip.to_string());
                            break;
                        }
                    }
                    continue;
                }
                idx += 1;
                while idx < raw.len() {
                    if raw.as_bytes()[idx] == b'\\' {
                        idx += 2;
                        continue;
                    }
                    if raw.as_bytes()[idx] == ch as u8 {
                        idx += 1;
                        break;
                    }
                    idx += 1;
                }
                continue;
            }
            if ch == '#' {
                hit = Some(idx);
                break;
            }
            idx += ch.len_utf8();
        }
        triple = in_triple;
        if let Some(start) = hit {
            let token_text = &raw[start..];
            let exempt = (line_no == 1 && raw.starts_with("#!"))
                || directive().is_match(&normalized_token(token_text));
            comments.push(Comment {
                line: line_no,
                end_line: line_no,
                snippet: token_text.trim().to_string(),
                token: normalized_token(token_text),
                exempt,
            });
        }
        let _ = &mut chars;
    }
    comments
}

fn tokens_for(file: &str, text: &str) -> Vec<String> {
    let Some(lang) = lang_of(file) else {
        return vec![];
    };
    let license_end = leading_license_end(text, lang);
    let mut comments = match lang {
        Lang::Python => scan_python(text),
        Lang::CStyle | Lang::JavaScript => scan_c_style(text),
    };
    comments.retain(|c| {
        !c.exempt && c.line > license_end && !c.token.is_empty() && !allowed_comment(file, text, c)
    });
    comments.into_iter().map(|c| c.token).collect()
}

/// Native symbol docs are the allowed forms: an explicit `Why:` rationale,
/// JS `/**`/`///` docs attached to a following declaration, Go line docs
/// naming their symbol, and in Rust every `///` or `//!`, which only exist
/// as doc syntax.
fn allowed_comment(file: &str, text: &str, item: &Comment) -> bool {
    if is_why(&item.token) {
        return true;
    }
    match lang_of(file) {
        Some(Lang::JavaScript) => is_js_doc(text, item),
        Some(Lang::CStyle) if file.ends_with(".go") => is_go_doc(text, item),
        Some(Lang::CStyle) => item.snippet.starts_with("///") || item.snippet.starts_with("//!"),
        _ => false,
    }
}

fn is_why(token: &str) -> bool {
    let lower = token.to_lowercase();
    lower.starts_with("why:") && token.len() > "why:".len()
}

fn is_js_doc(text: &str, item: &Comment) -> bool {
    if !item.snippet.starts_with("/**") && !item.snippet.starts_with("///") {
        return false;
    }
    let lines: Vec<&str> = text.lines().collect();
    let declaration = lines.get(item.end_line).copied().unwrap_or("");
    let decl_re = Regex::new(
        r"^\s*(?:(?:export|declare)\s+)*(?:default\s+)?(?:async\s+)?(?:function|class|interface|type|enum|namespace|const|let|var)\s+[A-Za-z_$]",
    )
    .unwrap();
    let member_re = Regex::new(
        r"^\s+(?:(?:public|private|protected|static|readonly|abstract|async|get|set)\s+)*[#A-Za-z_$][\w$#]*\s*(?:[(:=])",
    )
    .unwrap();
    decl_re.is_match(declaration) || member_re.is_match(declaration)
}

fn is_go_doc(text: &str, item: &Comment) -> bool {
    if !item.snippet.starts_with("//") {
        return false;
    }
    let lines: Vec<&str> = text.lines().collect();
    let mut start = item.line - 1;
    while start > 0 && lines[start - 1].trim_start().starts_with("//") {
        start -= 1;
    }
    let mut index = item.end_line;
    while index < lines.len() && lines[index].trim_start().starts_with("//") {
        index += 1;
    }
    let Some(declaration) = lines.get(index).and_then(|l| {
        Regex::new(r"^\s*(?:func\s+(?:\([^)]*\)\s*)?|type\s+|var\s+|const\s+)([A-Za-z_]\w*)")
            .ok()?
            .captures(l)
    }) else {
        return false;
    };
    let name = declaration.get(1).map(|m| m.as_str()).unwrap_or("");
    normalized_token(lines[start])
        .to_lowercase()
        .starts_with(&name.to_lowercase())
}

fn leading_license_end(text: &str, lang: Lang) -> usize {
    let lines: Vec<&str> = text.lines().collect();
    let mut index = if lines.first().is_some_and(|l| l.starts_with("#!")) {
        1
    } else {
        0
    };
    while index < lines.len() && lines[index].trim().is_empty() {
        index += 1;
    }
    let start = index;
    let prefix_re = Regex::new(if lang == Lang::Python {
        r"^\s*#"
    } else {
        r"^\s*//"
    })
    .unwrap();
    while index < lines.len() && prefix_re.is_match(lines[index]) {
        index += 1;
    }
    if index == start {
        return 0;
    }
    if license_re().is_match(&lines[start..index].join("\n")) {
        index
    } else {
        0
    }
}

/// CheckFiles lints each file against its committed HEAD version, so only
/// prose comments newly added relative to the last commit are violations.
/// Untracked or new files compare against empty, meaning every prose comment
/// in them counts.
pub fn check_files(files: &[String]) -> Vec<Finding> {
    let mut out = Vec::new();
    for file in files {
        let Some(_) = lang_of(file) else { continue };
        let Ok(new_text) = fs::read_to_string(file) else {
            continue;
        };
        let old_text = head_version(file).unwrap_or_default();
        for v in lint_pair(file, &old_text, &new_text) {
            out.push(Finding {
                file: file.into(),
                line: v.line,
                rule: "comment-lint".into(),
                severity: Severity::Blocking,
                message: format!(
                    "New prose comment: {}. Keep only non-obvious invariants, use Why: for rationale, or native docs for symbols.",
                    v.snippet
                ),
                source: "comment-lint",
            });
        }
    }
    out
}

fn head_version(file: &str) -> Option<String> {
    let out = Command::new("git")
        .args(["show", &format!("HEAD:{file}")])
        .output()
        .ok()?;
    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn violations(file: &str, old: &str, new: &str) -> Vec<Violation> {
        lint_pair(file, old, new)
    }

    #[test]
    fn rust_plain_prose_is_flagged() {
        let got = violations("a.rs", "", "// move the card\nfn a() {}\n");
        assert_eq!(got.len(), 1);
        assert!(got[0].snippet.contains("move the card"));
    }

    #[test]
    fn rust_native_docs_are_allowed() {
        let got = violations(
            "a.rs",
            "",
            "/// Formats the payout.\nfn fmt() {}\n//! module doc\n// Why: the ordering is load-bearing\n",
        );
        assert!(got.is_empty(), "{got:?}");
    }

    #[test]
    fn rust_inner_attribute_directives_allowed() {
        let got = violations("a.rs", "", "#![allow(dead_code)]\nfn a() {}\n");
        assert!(got.is_empty(), "{got:?}");
    }

    #[test]
    fn python_prose_flagged_and_directives_allowed() {
        let got = violations("a.py", "", "x = 1  # noqa\n# step one\n");
        assert_eq!(got.len(), 1);
        assert!(got[0].snippet.contains("step one"));
    }

    #[test]
    fn python_strings_do_not_produce_comments() {
        let got = violations("a.py", "", "s = \"# not a comment\"\n");
        assert!(got.is_empty(), "{got:?}");
    }

    #[test]
    fn why_rationale_is_allowed() {
        let got = violations(
            "a.go",
            "",
            "// Why: the loop must run before flush\nflush();\n",
        );
        assert!(got.is_empty(), "{got:?}");
    }

    #[test]
    fn go_symbol_doc_is_allowed() {
        let got = violations(
            "a.go",
            "",
            "// Helper drains the queue.\nfunc helper() {}\n",
        );
        assert!(got.is_empty(), "{got:?}");
    }

    #[test]
    fn js_doc_is_allowed() {
        let got = violations(
            "a.ts",
            "",
            "/** Parses the payload. */\nfunction parse() {}\n",
        );
        assert!(got.is_empty(), "{got:?}");
    }

    #[test]
    fn moved_comment_is_not_reflagged() {
        let old = "fn a() {\n    // step one\n}\n";
        let new = "fn a() {\n\n}\n\nfn b() {\n    // step one\n}\n";
        assert!(violations("a.rs", old, new).is_empty());
    }

    #[test]
    fn unchanged_comment_not_flagged_again() {
        let old = "// existing prose\nfn a() {}\n";
        let new = "// existing prose\nfn a() {}\nfn b() {}\n";
        assert!(violations("a.rs", old, new).is_empty());
    }

    #[test]
    fn license_header_allowed() {
        let body = "// Copyright 2026 someone\n// All rights reserved.\n\nfn a() {}\n";
        assert!(
            violations("a.rs", "", body).is_empty(),
            "{:?}",
            violations("a.rs", "", body)
        );
    }

    #[test]
    fn shebang_allowed() {
        assert!(violations("a.py", "", "#!/usr/bin/env python3\nx = 1\n").is_empty());
    }
}
