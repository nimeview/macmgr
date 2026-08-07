/// Safe UTF-8 prefix truncation (won't panic on char boundaries).
pub fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

pub fn status_severity(msg: &str) -> StatusKind {
    let lower = msg.to_lowercase();
    if lower.starts_with("error") || lower.contains("failed") || lower.contains("!!") {
        StatusKind::Error
    } else if lower.starts_with("ok")
        || lower.contains("enabled")
        || lower.contains("saved")
        || lower.contains("applied")
        || lower.contains("installed")
        || lower.contains("done")
        || lower.contains("reloaded")
        || lower.contains("snapshot")
    {
        StatusKind::Ok
    } else {
        StatusKind::Info
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusKind {
    Ok,
    Error,
    Info,
}
