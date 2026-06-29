//! Animation state machine for the desktop companion renderer.
//!
//! Maps WebUI companion snapshots to animation states, resolving priority
//! between attention items (approval > clarify > agent state).

use crate::sprite::AnimationState;
use serde::Serialize;

// ---------------------------------------------------------------------------
// Input types — mirror the WebUI companion-adapter.js snapshot shape
// ---------------------------------------------------------------------------

/// The overall companion state from the WebUI bridge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CompanionState {
    Idle,
    Running,
    Ready,
    /// Sidecar unreachable or startup failure — no pet data available.
    Failed,
}

/// Status of a single attention item in the snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AttentionStatus {
    Running,
    Ready,
    Approval,
    Clarify,
}

/// A single session attention entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AttentionItem {
    /// The attention status for this session.
    pub status: AttentionStatus,
    /// Preview text from the session (last message, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Session ID for opening in WebUI.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

/// Snapshot received from the WebUI companion bridge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CompanionSnapshot {
    /// Overall companion state.
    pub state: CompanionState,
    /// Individual session attention items.
    pub attention: Vec<AttentionItem>,
}

// ---------------------------------------------------------------------------
// Animation state helpers
// ---------------------------------------------------------------------------

impl AnimationState {
    /// Return the lowercase string representation for frontend consumption.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::RunningRight => "running-right",
            Self::RunningLeft => "running-left",
            Self::Waving => "waving",
            Self::Jumping => "jumping",
            Self::Failed => "failed",
            Self::Waiting => "waiting",
            Self::Running => "running",
            Self::Review => "review",
        }
    }
}

// ---------------------------------------------------------------------------
// API response types
// ---------------------------------------------------------------------------

/// Wraps a `CompanionSnapshot` with the resolved animation state so frontends
/// don't need to re-implement priority logic.
#[derive(Debug, Serialize)]
pub struct StateResponse {
    pub state: CompanionState,
    pub attention: Vec<AttentionItem>,
    /// Resolved animation state string (e.g. "idle", "waiting", "failed").
    pub resolved_animation: String,
}

impl From<&CompanionSnapshot> for StateResponse {
    fn from(snap: &CompanionSnapshot) -> Self {
        Self {
            state: snap.state.clone(),
            attention: snap.attention.clone(),
            resolved_animation: resolve_animation_state(snap).as_str().to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// State machine
// ---------------------------------------------------------------------------

/// Priority: Failed > Approval > Clarify > agent state.
///
/// - `CompanionState::Failed` → `Failed` (sidecar unreachable, no pet data)
/// - Any `Approval` attention item → `Waiting`
/// - Any `Clarify` attention item → `Review`
/// - `CompanionState::Running` → `Running`
/// - `CompanionState::Ready` → `Waving`
/// - `CompanionState::Idle` → `Idle`
pub fn resolve_animation_state(snapshot: &CompanionSnapshot) -> AnimationState {
    // Failed takes highest priority — sidecar unreachable means no pet data
    if snapshot.state == CompanionState::Failed {
        return AnimationState::Failed;
    }

    // Approval has highest priority — scan first
    if snapshot
        .attention
        .iter()
        .any(|a| a.status == AttentionStatus::Approval)
    {
        return AnimationState::Waiting;
    }

    // Clarify is second priority
    if snapshot
        .attention
        .iter()
        .any(|a| a.status == AttentionStatus::Clarify)
    {
        return AnimationState::Review;
    }

    // Fall through to companion state
    match snapshot.state {
        CompanionState::Running => AnimationState::Running,
        CompanionState::Ready => AnimationState::Waving,
        CompanionState::Failed => AnimationState::Failed,
        CompanionState::Idle => AnimationState::Idle,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(state: CompanionState, attention: Vec<AttentionStatus>) -> CompanionSnapshot {
        CompanionSnapshot {
            state,
            attention: attention
                .into_iter()
                .map(|s| AttentionItem {
                    status: s,
                    text: None,
                    session_id: None,
                })
                .collect(),
        }
    }

    #[test]
    fn idle_snapshot_maps_to_idle() {
        let snap = snapshot(CompanionState::Idle, vec![]);
        assert_eq!(resolve_animation_state(&snap), AnimationState::Idle);
    }

    #[test]
    fn running_snapshot_maps_to_running() {
        let snap = snapshot(CompanionState::Running, vec![]);
        assert_eq!(resolve_animation_state(&snap), AnimationState::Running);
    }

    #[test]
    fn ready_snapshot_maps_to_waving() {
        let snap = snapshot(CompanionState::Ready, vec![]);
        assert_eq!(resolve_animation_state(&snap), AnimationState::Waving);
    }

    #[test]
    fn approval_takes_priority_over_state() {
        let snap = snapshot(
            CompanionState::Running,
            vec![AttentionStatus::Approval],
        );
        assert_eq!(resolve_animation_state(&snap), AnimationState::Waiting);
    }

    #[test]
    fn clarify_takes_priority_over_ready() {
        let snap = snapshot(
            CompanionState::Ready,
            vec![AttentionStatus::Clarify],
        );
        assert_eq!(resolve_animation_state(&snap), AnimationState::Review);
    }

    #[test]
    fn approval_beats_clarify() {
        let snap = snapshot(
            CompanionState::Idle,
            vec![AttentionStatus::Clarify, AttentionStatus::Approval],
        );
        assert_eq!(resolve_animation_state(&snap), AnimationState::Waiting);
    }

    #[test]
    fn ignores_running_and_ready_attention() {
        let snap = snapshot(
            CompanionState::Running,
            vec![AttentionStatus::Running, AttentionStatus::Ready],
        );
        assert_eq!(resolve_animation_state(&snap), AnimationState::Running);
    }

    #[test]
    fn empty_attention_falls_through_to_state() {
        let snap = snapshot(CompanionState::Idle, vec![]);
        assert_eq!(resolve_animation_state(&snap), AnimationState::Idle);
    }

    #[test]
    fn failed_state_maps_to_failed_animation() {
        let snap = snapshot(CompanionState::Failed, vec![]);
        assert_eq!(resolve_animation_state(&snap), AnimationState::Failed);
    }

    #[test]
    fn failed_beats_approval() {
        let snap = snapshot(
            CompanionState::Failed,
            vec![AttentionStatus::Approval, AttentionStatus::Clarify],
        );
        assert_eq!(resolve_animation_state(&snap), AnimationState::Failed);
    }

    #[test]
    fn as_str_returns_lowercase() {
        assert_eq!(AnimationState::Idle.as_str(), "idle");
        assert_eq!(AnimationState::Running.as_str(), "running");
        assert_eq!(AnimationState::Waving.as_str(), "waving");
        assert_eq!(AnimationState::Waiting.as_str(), "waiting");
        assert_eq!(AnimationState::Review.as_str(), "review");
        assert_eq!(AnimationState::Failed.as_str(), "failed");
    }

    #[test]
    fn state_response_includes_resolved_animation() {
        let snap = snapshot(
            CompanionState::Running,
            vec![AttentionStatus::Approval],
        );
        let resp = StateResponse::from(&snap);
        assert_eq!(resp.resolved_animation, "waiting"); // Approval beats Running
        assert_eq!(resp.state, CompanionState::Running);
        assert_eq!(resp.attention.len(), 1);
    }

    #[test]
    fn state_response_idle() {
        let snap = snapshot(CompanionState::Idle, vec![]);
        let resp = StateResponse::from(&snap);
        assert_eq!(resp.resolved_animation, "idle");
    }
}
