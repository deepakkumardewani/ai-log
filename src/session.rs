//! Session DAG threading via `parentUuid`.
//!
//! Builds a tree / DAG of [`MessageNode`]s from a flat list of
//! [`TranscriptEntry`]s. Detects forks (multiple children sharing a
//! parent), sidechains (`isSidechain == true`), and session boundaries.

use std::collections::HashMap;

use uuid::Uuid;

use crate::model::entry::TranscriptEntry;

/// A node in the message tree / DAG.
#[derive(Debug, Clone)]
pub struct MessageNode {
    /// The underlying transcript entry.
    pub entry: TranscriptEntry,
    /// UUIDs of direct children (by `parentUuid`).
    pub children: Vec<Uuid>,
    /// Whether this message is on a sidechain.
    pub is_sidechain: bool,
    /// Depth from root (0 for roots).
    pub depth: usize,
}

/// A session composed of threaded messages.
#[derive(Debug, Clone)]
pub struct Session {
    /// Session identifier (from `sessionId` field).
    pub id: String,
    /// UUIDs of root messages (those with no parent in this session).
    pub root_message_ids: Vec<Uuid>,
    /// All messages keyed by UUID.
    pub messages: HashMap<Uuid, MessageNode>,
    /// Sidechain message UUIDs.
    pub sidechains: Vec<Uuid>,
    /// Fork points: UUIDs with more than one child.
    pub forks: Vec<Uuid>,
}

/// Context for building a session tree.
struct BuildContext {
    /// All entries indexed by UUID → entry.
    entry_map: HashMap<Uuid, TranscriptEntry>,
    /// UUID → Vec<child UUID> (from parentUuid).
    children_map: HashMap<Uuid, Vec<Uuid>>,
}

impl BuildContext {
    fn new(entries: &[TranscriptEntry]) -> Self {
        let mut entry_map: HashMap<Uuid, TranscriptEntry> = HashMap::with_capacity(entries.len());
        let mut children_map: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        let mut synthetic_parents: HashMap<Uuid, Vec<Uuid>> = HashMap::new();

        for entry in entries {
            let (uuid, parent_uuid, _is_sidechain) = match entry {
                TranscriptEntry::Unknown { raw, .. } => {
                    // Try to extract common fields from the raw JSON.
                    let uuid = raw
                        .get("uuid")
                        .and_then(|v| v.as_str())
                        .and_then(|s| Uuid::parse_str(s).ok());
                    let parent = raw
                        .get("parentUuid")
                        .and_then(|v| v.as_str())
                        .and_then(|s| Uuid::parse_str(s).ok());
                    let sidechain =
                        raw.get("isSidechain").and_then(|v| v.as_bool()).unwrap_or(false);
                    match uuid {
                        Some(id) => (id, parent, sidechain),
                        None => continue,
                    }
                }
                other => {
                    let common = other.common();
                    (common.uuid, common.parent_uuid, common.is_sidechain)
                }
            };

            entry_map.insert(uuid, entry.clone());

            if let Some(parent) = parent_uuid {
                if entry_map.contains_key(&parent) {
                    children_map.entry(parent).or_default().push(uuid);
                } else if synthetic_parents.contains_key(&parent) {
                    synthetic_parents.get_mut(&parent).unwrap().push(uuid);
                } else {
                    synthetic_parents.entry(parent).or_default().push(uuid);
                }
            }
        }

        // Re-check synthetic parents: if the parent has since appeared in the
        // entry map, move children from synthetic to real.
        let resolved: Vec<Uuid> =
            synthetic_parents.keys().filter(|k| entry_map.contains_key(k)).cloned().collect();
        for parent in &resolved {
            if let Some(children) = synthetic_parents.remove(parent) {
                children_map.entry(*parent).or_default().extend(children);
            }
        }

        BuildContext {
            entry_map,
            children_map,
        }
    }
}

/// Build a [`Session`] from a flat list of entries.
///
/// Orphan messages (those whose `parentUuid` references a UUID not present
/// in the entry list) are attached to a synthetic root.
pub fn build_session(entries: &[TranscriptEntry]) -> Session {
    if entries.is_empty() {
        return Session {
            id: String::new(),
            root_message_ids: Vec::new(),
            messages: HashMap::new(),
            sidechains: Vec::new(),
            forks: Vec::new(),
        };
    }

    let ctx = BuildContext::new(entries);

    // Determine session ID from the first entry with one.
    let session_id = entries.iter().filter_map(extract_session_id).next().unwrap_or_default();

    let mut messages: HashMap<Uuid, MessageNode> = HashMap::with_capacity(ctx.entry_map.len());
    let mut sidechains = Vec::new();
    let mut forks = Vec::new();

    // Compute roots: entries with no parentUuid, or whose parentUuid is NOT
    // in the entry map (synthetic / orphan root).
    let mut root_ids: Vec<Uuid> = Vec::new();

    for (uuid, entry) in &ctx.entry_map {
        let has_parent_in_map =
            extract_parent_uuid(entry).map(|p| ctx.entry_map.contains_key(&p)).unwrap_or(false);

        if !has_parent_in_map {
            root_ids.push(*uuid);
        }

        let children = ctx.children_map.get(uuid).cloned().unwrap_or_default();

        if children.len() > 1 {
            forks.push(*uuid);
        }

        if is_entry_sidechain(entry) {
            sidechains.push(*uuid);
        }
    }

    // Compute depths via BFS from roots.
    let mut depths: HashMap<Uuid, usize> = HashMap::new();
    let mut queue: Vec<(Uuid, usize)> = root_ids.iter().map(|id| (*id, 0)).collect();
    while let Some((uuid, depth)) = queue.pop() {
        depths.insert(uuid, depth);
        if let Some(children) = ctx.children_map.get(&uuid) {
            for child in children {
                queue.push((*child, depth + 1));
            }
        }
    }

    // Build MessageNodes.
    for (uuid, entry) in &ctx.entry_map {
        let children = ctx.children_map.get(uuid).cloned().unwrap_or_default();
        let depth = depths.get(uuid).copied().unwrap_or(0);

        messages.insert(
            *uuid,
            MessageNode {
                entry: entry.clone(),
                children,
                is_sidechain: is_entry_sidechain(entry),
                depth,
            },
        );
    }

    Session {
        id: session_id,
        root_message_ids: root_ids,
        messages,
        sidechains,
        forks,
    }
}

// ---------------------------------------------------------------------------
// Helpers for entries that may be Unknown
// ---------------------------------------------------------------------------

fn extract_session_id(entry: &TranscriptEntry) -> Option<String> {
    match entry {
        TranscriptEntry::Unknown { raw, .. } => {
            raw.get("sessionId").and_then(|v| v.as_str()).map(|s| s.to_string())
        }
        other => Some(other.common().session_id.clone()),
    }
}

fn extract_parent_uuid(entry: &TranscriptEntry) -> Option<Uuid> {
    match entry {
        TranscriptEntry::Unknown { raw, .. } => {
            raw.get("parentUuid").and_then(|v| v.as_str()).and_then(|s| Uuid::parse_str(s).ok())
        }
        other => other.common().parent_uuid,
    }
}

fn is_entry_sidechain(entry: &TranscriptEntry) -> bool {
    match entry {
        TranscriptEntry::Unknown { raw, .. } => {
            raw.get("isSidechain").and_then(|v| v.as_bool()).unwrap_or(false)
        }
        other => other.common().is_sidechain,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_reader;
    use std::io::Cursor;

    fn parse_entries(jsonl: &str) -> Vec<TranscriptEntry> {
        let result = parse_reader(Cursor::new(jsonl)).unwrap();
        result.entries
    }

    #[test]
    fn linear_session_produces_chain() {
        let u1 = "550e8400-e29b-41d4-a716-446655440001";
        let a1 = "550e8400-e29b-41d4-a716-446655440002";
        let u2 = "550e8400-e29b-41d4-a716-446655440003";
        let jsonl = format!(
            r#"
{{"type":"user","uuid":"{u1}","timestamp":"2025-06-15T10:30:00Z","sessionId":"s1","message":{{"role":"user","content":[{{"type":"text","text":"hi"}}]}}}}
{{"type":"assistant","uuid":"{a1}","parentUuid":"{u1}","timestamp":"2025-06-15T10:30:05Z","sessionId":"s1","message":{{"role":"assistant","content":[{{"type":"text","text":"Hello!"}}]}}}}
{{"type":"user","uuid":"{u2}","parentUuid":"{a1}","timestamp":"2025-06-15T10:31:00Z","sessionId":"s1","message":{{"role":"user","content":[{{"type":"text","text":"bye"}}]}}}}
"#
        );
        let entries = parse_entries(&jsonl);
        let session = build_session(&entries);

        assert_eq!(session.id, "s1");
        assert_eq!(session.root_message_ids.len(), 1);
        assert_eq!(session.root_message_ids[0], Uuid::parse_str(u1).unwrap());
        assert_eq!(session.messages.len(), 3);
        assert!(session.forks.is_empty());

        // u1 → a1 → u2
        let u1_node = &session.messages[&Uuid::parse_str(u1).unwrap()];
        assert_eq!(u1_node.children.len(), 1);
        assert_eq!(u1_node.children[0], Uuid::parse_str(a1).unwrap());
        assert_eq!(u1_node.depth, 0);

        let a1_node = &session.messages[&Uuid::parse_str(a1).unwrap()];
        assert_eq!(a1_node.children.len(), 1);
        assert_eq!(a1_node.children[0], Uuid::parse_str(u2).unwrap());
        assert_eq!(a1_node.depth, 1);

        let u2_node = &session.messages[&Uuid::parse_str(u2).unwrap()];
        assert!(u2_node.children.is_empty());
        assert_eq!(u2_node.depth, 2);
    }

    #[test]
    fn forked_session_detects_fork() {
        let u1 = "550e8400-e29b-41d4-a716-446655440001";
        let a1 = "550e8400-e29b-41d4-a716-446655440002";
        let a2 = "550e8400-e29b-41d4-a716-446655440003";
        let jsonl = format!(
            r#"
{{"type":"user","uuid":"{u1}","timestamp":"2025-06-15T10:30:00Z","sessionId":"s1","message":{{"role":"user","content":[{{"type":"text","text":"start"}}]}}}}
{{"type":"assistant","uuid":"{a1}","parentUuid":"{u1}","timestamp":"2025-06-15T10:30:05Z","sessionId":"s1","message":{{"role":"assistant","content":[{{"type":"text","text":"branch 1"}}]}}}}
{{"type":"assistant","uuid":"{a2}","parentUuid":"{u1}","timestamp":"2025-06-15T10:30:06Z","sessionId":"s1","isSidechain":true,"message":{{"role":"assistant","content":[{{"type":"text","text":"branch 2 (sidechain)"}}]}}}}
"#
        );
        let entries = parse_entries(&jsonl);
        let session = build_session(&entries);

        assert_eq!(session.messages.len(), 3);
        assert_eq!(session.forks.len(), 1);
        assert_eq!(session.forks[0], Uuid::parse_str(u1).unwrap());
        assert_eq!(session.sidechains.len(), 1);
        assert_eq!(session.sidechains[0], Uuid::parse_str(a2).unwrap());
    }

    #[test]
    fn orphan_message_attaches_to_synthetic_root() {
        let a1 = "550e8400-e29b-41d4-a716-446655440002";
        let missing = "550e8400-e29b-41d4-a716-446655449999";
        let jsonl = format!(
            r#"
{{"type":"assistant","uuid":"{a1}","parentUuid":"{missing}","timestamp":"2025-06-15T10:30:05Z","sessionId":"s1","message":{{"role":"assistant","content":[{{"type":"text","text":"orphan"}}]}}}}
"#
        );
        let entries = parse_entries(&jsonl);
        let session = build_session(&entries);

        assert_eq!(session.messages.len(), 1);
        assert_eq!(session.root_message_ids.len(), 1);
        assert_eq!(session.root_message_ids[0], Uuid::parse_str(a1).unwrap());
    }

    #[test]
    fn empty_entries_produce_empty_session() {
        let entries: Vec<TranscriptEntry> = vec![];
        let session = build_session(&entries);
        assert!(session.id.is_empty());
        assert!(session.messages.is_empty());
    }
}
