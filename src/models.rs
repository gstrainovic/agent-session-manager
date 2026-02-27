use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::LazyLock;

static RE_CAVEAT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)<local-command-caveat>.*?</local-command-caveat>\n?").unwrap()
});
static RE_TASK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)<task-notification>(.*?)</task-notification>").unwrap()
});
static RE_CMD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)<command-name>(.*?)</command-name>.*?<command-args>(.*?)</command-args>")
        .unwrap()
});
static RE_STDOUT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)<local-command-stdout>(.*?)</local-command-stdout>").unwrap()
});
static RE_XML: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^>]+>").unwrap());
static RE_ANSI: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\x1b\[[0-9;]*[mGKHFABCDsuJr]|\[[0-9;]*m)").unwrap());
static RE_BLANK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\n{3,}").unwrap());

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub project_path: String,
    pub project_name: String,
    pub created_at: String,
    pub updated_at: String,
    pub size: u64,
    pub total_entries: usize,
    pub messages: Vec<Message>,
    #[serde(skip)]
    pub jsonl_path: PathBuf,
    #[serde(skip)]
    pub slug: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

impl Session {
    #[cfg(test)]
    pub fn new(id: String, project_path: String) -> Self {
        let project_name = project_path
            .rsplit('/')
            .next()
            .unwrap_or(&project_path)
            .to_string();
        Self {
            id,
            project_name,
            project_path,
            created_at: chrono::Local::now().to_rfc3339(),
            updated_at: chrono::Local::now().to_rfc3339(),
            size: 0,
            total_entries: 0,
            messages: Vec::new(),
            jsonl_path: PathBuf::new(),
            slug: None,
        }
    }

    pub fn display_project_name(&self) -> &str {
        self.project_name.trim_matches('-')
    }

    pub fn display_name(&self) -> String {
        let short_id = if self.id.len() > 8 {
            &self.id[..8]
        } else {
            &self.id
        };
        if let Some(slug) = &self.slug {
            format!("{} [{}] ({})", self.project_name, slug, short_id)
        } else {
            format!("{} ({})", self.project_name, short_id)
        }
    }
}

/// Extrahiert den customTitle aus JSONL-Inhalt (manueller Rename via Claude Code /rename).
pub fn extract_custom_title(content: &str) -> Option<String> {
    for line in content.lines() {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
            if json.get("type").and_then(|t| t.as_str()) == Some("custom-title") {
                if let Some(title) = json.get("customTitle").and_then(|s| s.as_str()) {
                    if !title.is_empty() {
                        return Some(title.to_string());
                    }
                }
            }
        }
    }
    None
}

pub fn count_jsonl_entries(content: &str) -> usize {
    content
        .lines()
        .filter(|line| serde_json::from_str::<serde_json::Value>(line).is_ok())
        .count()
}

fn clean_message_content(content: &str) -> String {
    let mut text = content.to_string();

    text = RE_CAVEAT.replace_all(&text, "").into_owned();

    text = RE_TASK
        .replace_all(&text, |caps: &regex::Captures| {
            let inner = &caps[1];
            let summary = extract_xml_inner(inner, "summary").unwrap_or("?");
            let status = extract_xml_inner(inner, "status").unwrap_or("?");
            format!("> **[Task {}]** {}", status, summary)
        })
        .into_owned();

    text = RE_CMD
        .replace_all(&text, |caps: &regex::Captures| {
            let name = caps[1].trim();
            let args = caps[2].trim();
            if args.is_empty() {
                format!("`{}`", name)
            } else {
                format!("`{} {}`", name, args)
            }
        })
        .into_owned();

    text = RE_STDOUT
        .replace_all(&text, |caps: &regex::Captures| {
            let inner = strip_ansi(&caps[1]).trim().to_string();
            if inner.is_empty() {
                String::new()
            } else {
                format!("\n```\n{}\n```\n", inner)
            }
        })
        .into_owned();

    text = RE_XML.replace_all(&text, "").into_owned();
    text = strip_ansi(&text);
    text = RE_BLANK.replace_all(&text, "\n\n").into_owned();

    text.trim().to_string()
}

fn strip_ansi(s: &str) -> String {
    RE_ANSI.replace_all(s, "").into_owned()
}

/// Erkennt Messages die nur Slash-Commands oder reinen Stdout enthalten — kein echter Gesprächsinhalt.
fn is_noise_message(text: &str) -> bool {
    let t = text.trim();
    // Slash-Command: `/model`, `/exit`, `/context args` etc.
    if t.starts_with('`') && t.ends_with('`') && t.len() > 2 {
        let inner = &t[1..t.len() - 1];
        if inner.starts_with('/') && !inner.contains('\n') {
            return true;
        }
    }
    // Reiner Code-Block (Stdout-Output ohne begleitenden Text)
    if t.starts_with("```") && t.ends_with("```") && t.len() > 6 {
        return true;
    }
    false
}

fn extract_xml_inner<'a>(text: &'a str, tag: &str) -> Option<&'a str> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let start = text.find(&open)? + open.len();
    let end = text.find(&close)?;
    Some(text[start..end].trim())
}

pub fn parse_jsonl_messages(content: &str) -> Vec<Message> {
    let mut messages = Vec::new();

    for line in content.lines() {
        let json: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let entry_type = json.get("type").and_then(|t| t.as_str()).unwrap_or("");

        if entry_type != "user" && entry_type != "assistant" {
            continue;
        }

        let msg = match json.get("message") {
            Some(m) => m,
            None => continue,
        };

        let role = msg
            .get("role")
            .and_then(|r| r.as_str())
            .unwrap_or("unknown");

        let content_val = match msg.get("content") {
            Some(c) => c,
            None => continue,
        };

        let text = if let Some(s) = content_val.as_str() {
            s.to_string()
        } else if let Some(arr) = content_val.as_array() {
            arr.iter()
                .filter_map(|item| {
                    if item.get("type").and_then(|t| t.as_str()) == Some("text") {
                        item.get("text").and_then(|t| t.as_str()).map(String::from)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            continue;
        };

        let text = clean_message_content(&text);

        if text.is_empty() || is_noise_message(&text) {
            continue;
        }

        messages.push(Message {
            role: role.to_string(),
            content: text,
        });
    }

    messages
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = Session::new("test-id".to_string(), "/home/g/my-project".to_string());
        assert_eq!(session.id, "test-id");
        assert_eq!(session.project_name, "my-project");
        assert_eq!(session.messages.len(), 0);
        assert_eq!(session.total_entries, 0);
    }

    #[test]
    fn test_display_name() {
        let session = Session::new(
            "abcdef12-3456-7890-abcd-ef1234567890".to_string(),
            "/home/g/auto-service".to_string(),
        );
        assert_eq!(session.display_name(), "auto-service (abcdef12)");
    }

    #[test]
    fn test_display_name_with_slug() {
        let mut session = Session::new(
            "abcdef12-3456-7890-abcd-ef1234567890".to_string(),
            "/home/g/auto-service".to_string(),
        );
        session.slug = Some("mein-label".to_string());
        assert_eq!(session.display_name(), "auto-service [mein-label] (abcdef12)");
    }

    #[test]
    fn test_extract_custom_title_finds_title() {
        let content = r#"{"type":"user","message":{"role":"user","content":"hello"},"uuid":"x"}
{"type":"custom-title","customTitle":"my-name","sessionId":"abc"}
"#;
        assert_eq!(extract_custom_title(content), Some("my-name".to_string()));
    }

    #[test]
    fn test_extract_custom_title_returns_none_when_absent() {
        let content = r#"{"type":"user","message":{"role":"user","content":"hello"},"uuid":"x"}
"#;
        assert_eq!(extract_custom_title(content), None);
    }

    #[test]
    fn test_extract_custom_title_skips_empty() {
        let content = r#"{"type":"custom-title","customTitle":"","sessionId":"abc"}
"#;
        assert_eq!(extract_custom_title(content), None);
    }

    #[test]
    fn test_parse_jsonl_user_message_string_content() {
        let line =
            r#"{"type":"user","message":{"role":"user","content":"hello world"},"uuid":"abc"}"#;
        let messages = parse_jsonl_messages(line);
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[0].content, "hello world");
    }

    #[test]
    fn test_parse_jsonl_assistant_message_array_content() {
        let line = r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"I will help"},{"type":"thinking","thinking":"hmm"}]},"uuid":"def"}"#;
        let messages = parse_jsonl_messages(line);
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].role, "assistant");
        assert_eq!(messages[0].content, "I will help");
    }

    #[test]
    fn test_parse_jsonl_skips_non_message_types() {
        let lines = r#"{"type":"file-history-snapshot","messageId":"abc","snapshot":{}}
{"type":"progress","data":{"type":"hook_progress"}}
{"type":"user","message":{"role":"user","content":"actual message"},"uuid":"xyz"}"#;
        let messages = parse_jsonl_messages(lines);
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "actual message");
    }

    #[test]
    fn test_parse_jsonl_skips_empty_text() {
        let line = r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":""}]},"uuid":"abc"}"#;
        let messages = parse_jsonl_messages(line);
        assert_eq!(messages.len(), 0);
    }

    #[test]
    fn test_parse_jsonl_multiple_text_blocks() {
        let line = r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"part one"},{"type":"text","text":"part two"}]},"uuid":"abc"}"#;
        let messages = parse_jsonl_messages(line);
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "part one\npart two");
    }

    #[test]
    fn test_parse_jsonl_invalid_json_skipped() {
        let lines = "not valid json\n{\"type\":\"user\",\"message\":{\"role\":\"user\",\"content\":\"ok\"},\"uuid\":\"x\"}";
        let messages = parse_jsonl_messages(lines);
        assert_eq!(messages.len(), 1);
    }

    #[test]
    fn test_count_jsonl_entries() {
        let content = r#"{"type":"file-history-snapshot","messageId":"abc","snapshot":{}}
{"type":"progress","data":{"type":"hook_progress"}}
{"type":"user","message":{"role":"user","content":"hello"},"uuid":"x"}
{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"hi"}]},"uuid":"y"}
{"type":"queue-operation","operation":"dequeue","timestamp":"2026-01-01T00:00:00Z"}"#;
        let total = count_jsonl_entries(content);
        assert_eq!(
            total, 5,
            "Should count all valid JSONL entries, not just user/assistant"
        );
    }

    #[test]
    fn test_count_jsonl_entries_skips_invalid() {
        let content = "not valid json\n{\"type\":\"user\",\"message\":{\"role\":\"user\",\"content\":\"ok\"},\"uuid\":\"x\"}\n";
        let total = count_jsonl_entries(content);
        assert_eq!(total, 1, "Should skip invalid JSON lines");
    }

    #[test]
    fn test_parse_jsonl_missing_message_field() {
        let line = r#"{"type":"user","uuid":"abc"}"#;
        let messages = parse_jsonl_messages(line);
        assert_eq!(messages.len(), 0, "Should skip entry without message field");
    }

    #[test]
    fn test_parse_jsonl_missing_content_field() {
        let line = r#"{"type":"user","message":{"role":"user"},"uuid":"abc"}"#;
        let messages = parse_jsonl_messages(line);
        assert_eq!(messages.len(), 0, "Should skip entry without content field");
    }

    #[test]
    fn test_parse_jsonl_content_neither_string_nor_array() {
        let line = r#"{"type":"user","message":{"role":"user","content":42},"uuid":"abc"}"#;
        let messages = parse_jsonl_messages(line);
        assert_eq!(
            messages.len(),
            0,
            "Should skip entry where content is a number"
        );
    }

    #[test]
    fn test_parse_jsonl_missing_role_uses_unknown() {
        let line = r#"{"type":"user","message":{"content":"hello"},"uuid":"abc"}"#;
        let messages = parse_jsonl_messages(line);
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].role, "unknown");
        assert_eq!(messages[0].content, "hello");
    }

    #[test]
    fn test_display_project_name_strips_leading_dash() {
        let mut s = Session::new("id".to_string(), "/home/g".to_string());
        s.project_name = "-home-g".to_string();
        assert_eq!(s.display_project_name(), "home-g");
    }

    #[test]
    fn test_display_project_name_no_dash() {
        let s = Session::new("id".to_string(), "/home/g/myproject".to_string());
        assert_eq!(s.display_project_name(), "myproject");
    }

    #[test]
    fn test_noise_message_slash_command_filtered() {
        let line = r#"{"type":"user","message":{"role":"user","content":"<command-name>/model</command-name>\n            <command-message>model</command-message>\n            <command-args></command-args>"},"uuid":"abc"}"#;
        let messages = parse_jsonl_messages(line);
        assert_eq!(messages.len(), 0, "slash command should be filtered");
    }

    #[test]
    fn test_noise_message_local_caveat_filtered() {
        let content = "before <local-command-caveat>DO NOT respond</local-command-caveat> after";
        let line = format!(
            r#"{{"type":"user","message":{{"role":"user","content":"{}"}},"uuid":"abc"}}"#,
            content.replace('"', "\\\"")
        );
        let messages = parse_jsonl_messages(&line);
        // "before  after" is not empty, so it stays — caveat itself is stripped
        assert!(
            messages.is_empty()
                || !messages[0].content.contains("local-command-caveat"),
            "caveat tag should be stripped"
        );
    }

    #[test]
    fn test_noise_message_stdout_only_filtered() {
        // A message that is purely a code block (stdout) should be filtered
        let cleaned = "```\nSet model to sonnet\n```";
        assert!(is_noise_message(cleaned), "stdout-only block should be noise");
    }

    #[test]
    fn test_noise_message_real_text_not_filtered() {
        assert!(!is_noise_message("kannst du mir helfen?"));
        assert!(!is_noise_message("hier ist code:\n```\nfoo\n```\nund mehr text"));
    }

    #[test]
    fn test_ansi_codes_stripped() {
        let line = r#"{"type":"user","message":{"role":"user","content":"\u001b[1mhello\u001b[0m"},"uuid":"abc"}"#;
        let messages = parse_jsonl_messages(line);
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "hello");
    }
}
