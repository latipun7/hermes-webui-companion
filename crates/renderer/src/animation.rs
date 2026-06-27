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
// State machine
// ---------------------------------------------------------------------------

/// Priority: Approval > Clarify > agent state.
///
/// - Any `Approval` attention item → `Waiting`
/// - Any `Clarify` attention item → `Review`
/// - `CompanionState::Running` → `Running`
/// - `CompanionState::Ready` → `Waving`
/// - `CompanionState::Idle` → `Idle`
pub fn resolve_animation_state(snapshot: &CompanionSnapshot) -> AnimationState {
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
}
