//! Bridge module — receives and parses WebUI companion snapshots.
//!
//! Listens for HTTP POST snapshots from the WebUI companion-adapter.js
//! and converts them into animation-compatible companion state.

use serde::Deserialize;

use crate::animation::{AttentionItem, AttentionStatus, CompanionSnapshot, CompanionState};

// ---------------------------------------------------------------------------
// Raw WebUI snapshot format
// ---------------------------------------------------------------------------

/// Raw snapshot shape as received from the WebUI companion-adapter.js.
#[derive(Debug, Deserialize, PartialEq)]
pub struct WebuiSnapshot {
    #[allow(dead_code)]
    pub source: Option<String>,
    #[allow(dead_code)]
    pub timestamp: Option<String>,
    pub companion: Option<CompanionData>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct CompanionData {
    pub state: Option<String>,
    pub attention: Option<Vec<RawAttentionItem>>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct RawAttentionItem {
    pub status: Option<String>,
    #[allow(dead_code)]
    pub title: Option<String>,
    pub text: Option<String>,
    pub session_id: Option<String>,
    pub action_required_type: Option<String>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

/// Parse a raw WebUI snapshot into an animation-ready `CompanionSnapshot`.
pub fn parse_snapshot(raw: &WebuiSnapshot) -> CompanionSnapshot {
    let state = match raw.companion.as_ref().and_then(|c| c.state.as_deref()) {
        Some("running") => CompanionState::Running,
        Some("ready") => CompanionState::Ready,
        _ => CompanionState::Idle,
    };

    let attention = raw
        .companion
        .as_ref()
        .and_then(|c| c.attention.as_ref())
        .map(|items| {
            items
                .iter()
                .filter_map(|a| {
                    let status = match a.status.as_deref() {
                        Some("approval") => AttentionStatus::Approval,
                        Some("clarify") => AttentionStatus::Clarify,
                        Some("action_required") => {
                            match a.action_required_type.as_deref() {
                                Some("approval") => AttentionStatus::Approval,
                                Some("clarify") => AttentionStatus::Clarify,
                                _ => AttentionStatus::Approval, // default
                            }
                        },
                        Some("running") => AttentionStatus::Running,
                        Some("ready") => AttentionStatus::Ready,
                        _ => return None,
                    };
                    Some(AttentionItem {
                        status,
                        text: a.text.clone().filter(|t| !t.is_empty()),
                        session_id: a.session_id.clone().filter(|t| !t.is_empty()),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    CompanionSnapshot { state, attention }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_idle_snapshot() {
        let raw: WebuiSnapshot =
            serde_json::from_str(r#"{"companion":{"state":"idle","attention":[]}}"#).unwrap();
        let snap = parse_snapshot(&raw);
        assert_eq!(snap.state, CompanionState::Idle);
        assert!(snap.attention.is_empty());
    }

    #[test]
    fn parse_running_snapshot() {
        let raw: WebuiSnapshot =
            serde_json::from_str(r#"{"companion":{"state":"running"}}"#).unwrap();
        let snap = parse_snapshot(&raw);
        assert_eq!(snap.state, CompanionState::Running);
    }

    #[test]
    fn parse_ready_snapshot() {
        let raw: WebuiSnapshot =
            serde_json::from_str(r#"{"companion":{"state":"ready"}}"#).unwrap();
        let snap = parse_snapshot(&raw);
        assert_eq!(snap.state, CompanionState::Ready);
    }

    #[test]
    fn parse_approval_attention() {
        let raw: WebuiSnapshot = serde_json::from_str(
            r#"{"companion":{"state":"idle","attention":[{"status":"approval","title":"test","session_id":"abc"}]}}"#,
        )
        .unwrap();
        let snap = parse_snapshot(&raw);
        assert_eq!(snap.state, CompanionState::Idle);
        assert_eq!(snap.attention.len(), 1);
        assert_eq!(snap.attention[0].status, AttentionStatus::Approval);
    }

    #[test]
    fn parse_clarify_attention() {
        let raw: WebuiSnapshot = serde_json::from_str(
            r#"{"companion":{"state":"running","attention":[{"status":"clarify"}]}}"#,
        )
        .unwrap();
        let snap = parse_snapshot(&raw);
        assert_eq!(snap.attention[0].status, AttentionStatus::Clarify);
    }

    #[test]
    fn parse_missing_companion_field() {
        let raw: WebuiSnapshot = serde_json::from_str(r#"{}"#).unwrap();
        let snap = parse_snapshot(&raw);
        assert_eq!(snap.state, CompanionState::Idle);
        assert!(snap.attention.is_empty());
    }

    #[test]
    fn unknown_status_skipped() {
        let raw: WebuiSnapshot = serde_json::from_str(
            r#"{"companion":{"attention":[{"status":"weird"},{"status":"approval"}]}}"#,
        )
        .unwrap();
        let snap = parse_snapshot(&raw);
        assert_eq!(snap.attention.len(), 1);
        assert_eq!(snap.attention[0].status, AttentionStatus::Approval);
    }
}
