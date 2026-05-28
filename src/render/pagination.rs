//! Pagination support for splitting long sessions across multiple HTML pages.
//!
//! When `--page-size N` is set, messages are batched into chunks of N.
//! Each chunk becomes a separate HTML file: `session-page-1.html`, `session-page-2.html`, etc.
//! The first page includes the full chrome (header, sidebar); subsequent pages are content-only.

use crate::session::Session;

/// A single page of a paginated session.
#[derive(Debug, Clone)]
pub struct Page {
    /// 1-based page number.
    pub number: usize,
    /// Total number of pages.
    pub total: usize,
    /// Indices into the session's messages for this page.
    pub message_range: std::ops::Range<usize>,
    /// Whether this is the first page (includes full chrome).
    pub is_first: bool,
    /// Whether this is the last page.
    pub is_last: bool,
}

/// Split a session into pages of at most `page_size` messages each.
///
/// Returns `None` if `page_size` is 0 or the session fits in one page.
pub fn paginate(session: &Session, page_size: usize) -> Option<Vec<Page>> {
    if page_size == 0 {
        return None;
    }

    let total_messages = session.messages.len();
    if total_messages <= page_size {
        return None;
    }

    let total_pages = total_messages.div_ceil(page_size);
    let pages: Vec<Page> = (0..total_pages)
        .map(|i| {
            let start = i * page_size;
            let end = ((i + 1) * page_size).min(total_messages);
            Page {
                number: i + 1,
                total: total_pages,
                message_range: start..end,
                is_first: i == 0,
                is_last: i == total_pages - 1,
            }
        })
        .collect();

    Some(pages)
}

/// Generate an output filename for a paginated page.
///
/// Page 1 uses the base stem; subsequent pages append `-page-N`.
pub fn page_filename(base_stem: &str, page: &Page) -> String {
    if page.total == 1 {
        format!("{}.html", base_stem)
    } else {
        format!("{}-page-{}.html", base_stem, page.number)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::content::{ContentItem, Message};
    use crate::model::entry::{CommonFields, TranscriptEntry, UserEntry};
    use chrono::{DateTime, Utc};
    use uuid::Uuid;

    fn make_session(msg_count: usize) -> Session {
        let mut entries = Vec::new();
        for i in 0..msg_count {
            entries.push(TranscriptEntry::User(UserEntry {
                common: CommonFields {
                    uuid: Uuid::parse_str(&format!("550e8400-e29b-41d4-a716-{:012}", i)).unwrap(),
                    parent_uuid: None,
                    timestamp: DateTime::parse_from_rfc3339("2025-01-01T00:00:00Z")
                        .unwrap()
                        .with_timezone(&Utc),
                    session_id: "test".to_string(),
                    is_sidechain: false,
                    agent_id: None,
                    cwd: None,
                    git_branch: None,
                    version: None,
                },
                team_name: None,
                request_id: None,
                user_type: None,
                message: Message {
                    role: "user".to_string(),
                    model: None,
                    stop_reason: None,
                    usage: None,
                    content: vec![ContentItem::Text {
                        text: format!("message {}", i),
                    }],
                },
            }));
        }
        crate::session::build_session(&entries)
    }

    #[test]
    fn paginate_small_session_returns_none() {
        let session = make_session(3);
        assert!(paginate(&session, 5).is_none());
    }

    #[test]
    fn paginate_large_session_splits_correctly() {
        let session = make_session(10);
        let pages = paginate(&session, 4).unwrap();
        assert_eq!(pages.len(), 3);
        assert_eq!(pages[0].message_range, 0..4);
        assert_eq!(pages[1].message_range, 4..8);
        assert_eq!(pages[2].message_range, 8..10);
        assert!(pages[0].is_first);
        assert!(!pages[0].is_last);
        assert!(!pages[1].is_first);
        assert!(!pages[1].is_last);
        assert!(!pages[2].is_first);
        assert!(pages[2].is_last);
    }

    #[test]
    fn paginate_exact_multiple_returns_none() {
        let session = make_session(6);
        // 6 messages with page_size 3 → 2 pages (should split)
        let pages = paginate(&session, 3).unwrap();
        assert_eq!(pages.len(), 2);
    }

    #[test]
    fn paginate_zero_page_size_returns_none() {
        let session = make_session(10);
        assert!(paginate(&session, 0).is_none());
    }

    #[test]
    fn page_filename_single() {
        let page = Page {
            number: 1,
            total: 1,
            message_range: 0..10,
            is_first: true,
            is_last: true,
        };
        assert_eq!(page_filename("session", &page), "session.html");
    }

    #[test]
    fn page_filename_multi() {
        let page = Page {
            number: 2,
            total: 3,
            message_range: 10..20,
            is_first: false,
            is_last: false,
        };
        assert_eq!(page_filename("session", &page), "session-page-2.html");
    }
}
