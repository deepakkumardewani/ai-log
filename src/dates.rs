//! Natural-language date parsing for --from-date / --to-date.
//!
//! Accepts: `today`, `yesterday`, `last week`, `last month`, ISO dates (YYYY-MM-DD).

use chrono::{DateTime, Days, NaiveDate, Utc};

/// Parse a user-supplied date string into a [`DateTime<Utc>`].
///
/// Supported formats:
/// - `today` — start of today (UTC)
/// - `yesterday` — start of yesterday (UTC)
/// - `last week` — 7 days ago at start of day
/// - `last month` — 30 days ago at start of day
/// - `YYYY-MM-DD` — start of that day (UTC)
pub fn parse_date(input: &str) -> anyhow::Result<DateTime<Utc>> {
    let trimmed = input.trim().to_lowercase();

    let today = Utc::now().date_naive();

    match trimmed.as_str() {
        "today" => Ok(today.and_hms_opt(0, 0, 0).unwrap().and_local_timezone(Utc).unwrap()),

        "yesterday" => {
            let d = today
                .pred_opt()
                .ok_or_else(|| anyhow::anyhow!("date underflow computing yesterday"))?;
            Ok(d.and_hms_opt(0, 0, 0).unwrap().and_local_timezone(Utc).unwrap())
        }

        "last week" => {
            let d = today - Days::new(7);
            Ok(d.and_hms_opt(0, 0, 0).unwrap().and_local_timezone(Utc).unwrap())
        }

        "last month" => {
            let d = today - Days::new(30);
            Ok(d.and_hms_opt(0, 0, 0).unwrap().and_local_timezone(Utc).unwrap())
        }

        // ISO date: YYYY-MM-DD
        _ => {
            if let Ok(naive) = NaiveDate::parse_from_str(trimmed.as_str(), "%Y-%m-%d") {
                Ok(naive.and_hms_opt(0, 0, 0).unwrap().and_local_timezone(Utc).unwrap())
            } else {
                anyhow::bail!(
                    "Cannot parse date '{}'. Expected: today, yesterday, last week, last month, or YYYY-MM-DD",
                    input
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_today() {
        let result = parse_date("today").unwrap();
        let now = Utc::now().date_naive();
        assert_eq!(result.date_naive(), now);
    }

    #[test]
    fn parse_yesterday() {
        let result = parse_date("yesterday").unwrap();
        let expected = Utc::now().date_naive().pred_opt().unwrap();
        assert_eq!(result.date_naive(), expected);
    }

    #[test]
    fn parse_last_week() {
        let result = parse_date("last week").unwrap();
        let expected = Utc::now().date_naive() - Days::new(7);
        assert_eq!(result.date_naive(), expected);
    }

    #[test]
    fn parse_last_month() {
        let result = parse_date("last month").unwrap();
        let expected = Utc::now().date_naive() - Days::new(30);
        assert_eq!(result.date_naive(), expected);
    }

    #[test]
    fn parse_iso_date() {
        let result = parse_date("2025-06-15").unwrap();
        assert_eq!(result.date_naive().to_string(), "2025-06-15");
    }

    #[test]
    fn parse_invalid_returns_error() {
        let err = parse_date("not-a-date").unwrap_err();
        assert!(err.to_string().contains("Cannot parse date"));
    }

    #[test]
    fn parse_case_insensitive() {
        let r1 = parse_date("Today").unwrap();
        let r2 = parse_date("TODAY").unwrap();
        assert_eq!(r1.date_naive(), r2.date_naive());
    }
}
