use std::path::Path;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Payload {
    #[serde(rename = "tool_name")]
    tool_name: String,
    #[serde(rename = "tool_input")]
    tool_input: ToolInput,
}

#[derive(Debug, Deserialize)]
struct ToolInput {
    #[serde(rename = "file_path", default)]
    file_path: Option<String>,
    #[serde(rename = "notebook_path", default)]
    notebook_path: Option<String>,
}

#[derive(Debug)]
pub enum StdinPayload {
    Paths(Vec<String>),
    None,
    Malformed(String),
}

/// PathsFromStdin extracts the files to check from a hook payload. An empty
/// stream or a payload for an unchecked tool passes through with no paths.
pub fn paths_from_stdin(raw: &str) -> StdinPayload {
    let trimmed = raw.trim();
    if trimmed.is_empty() || !trimmed.starts_with('{') {
        return StdinPayload::None;
    }
    let payload: Payload = match serde_json::from_str(trimmed) {
        Ok(p) => p,
        Err(e) => return StdinPayload::Malformed(format!("malformed hook payload: {e}")),
    };
    if !matches!(
        payload.tool_name.as_str(),
        "Edit" | "Write" | "MultiEdit" | "NotebookEdit"
    ) {
        return StdinPayload::None;
    }
    let path = payload
        .tool_input
        .file_path
        .or(payload.tool_input.notebook_path);
    match path {
        Some(p) => StdinPayload::Paths(vec![p]),
        None => StdinPayload::Malformed(format!(
            "payload for {:?} has no file path",
            payload.tool_name
        )),
    }
}

pub fn paths_from_args(args: &[String]) -> Result<Vec<String>, String> {
    let mut paths = Vec::new();
    for a in args {
        if a == "-" {
            continue;
        }
        if !Path::new(a).exists() {
            return Err(format!("file not found: {a}"));
        }
        paths.push(a.clone());
    }
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn edit_payload(path: &str) -> String {
        format!(
            r#"{{"tool_name":"Edit","tool_input":{{"file_path":"{path}","old_string":"a","new_string":"b"}},"session_id":"abc"}}"#
        )
    }

    #[test]
    fn edit_payload_yields_path() {
        match paths_from_stdin(&edit_payload("/repo/main.py")) {
            StdinPayload::Paths(p) => assert_eq!(p, vec!["/repo/main.py"]),
            other => panic!("want paths, got {other:?}"),
        }
    }

    #[test]
    fn notebook_payload_yields_notebook_path() {
        let payload =
            r#"{"tool_name":"NotebookEdit","tool_input":{"notebook_path":"/repo/n.ipynb"}}"#;
        match paths_from_stdin(payload) {
            StdinPayload::Paths(p) => assert_eq!(p, vec!["/repo/n.ipynb"]),
            other => panic!("want paths, got {other:?}"),
        }
    }

    #[test]
    fn unchecked_tool_passes_through() {
        let payload = r#"{"tool_name":"Bash","tool_input":{"command":"ls"}}"#;
        assert!(matches!(paths_from_stdin(payload), StdinPayload::None));
    }

    #[test]
    fn empty_stdin_passes_through() {
        assert!(matches!(paths_from_stdin(""), StdinPayload::None));
    }

    #[test]
    fn malformed_payload_is_usage_error() {
        match paths_from_stdin("{not json") {
            StdinPayload::Malformed(msg) => assert!(msg.contains("malformed hook payload")),
            other => panic!("want malformed, got {other:?}"),
        }
    }

    #[test]
    fn edit_payload_without_path_is_usage_error() {
        let payload = r#"{"tool_name":"Edit","tool_input":{}}"#;
        assert!(matches!(
            paths_from_stdin(payload),
            StdinPayload::Malformed(_)
        ));
    }

    #[test]
    fn args_reject_missing_file() {
        let dir = std::env::temp_dir().join("hook-args-missing-test");
        fs::create_dir_all(&dir).unwrap();
        let err = paths_from_args(&[dir.join("nope.py").to_string_lossy().into()]);
        assert!(err.is_err_and(|e| e.contains("file not found")));
    }
}
