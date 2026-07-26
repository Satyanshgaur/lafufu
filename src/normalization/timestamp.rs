use chrono::{DateTime, TimeZone, Utc};
use tracing::debug;

/// Flexible timestamp parser for normalizing raw log timestamps into DateTime<Utc>
pub fn parse_timestamp(raw: &str) -> Option<DateTime<Utc>> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    // 1. Try ISO8601 / RFC3339
    if let Ok(dt) = DateTime::parse_from_rfc3339(trimmed) {
        return Some(dt.with_timezone(&Utc));
    }

    // 2. Try standard ISO-like formats without timezone (assume Utc)
    let formats = [
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S",
        "%d/%b/%Y:%H:%M:%S %z", // Apache/Nginx format
    ];

    for fmt in &formats {
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(trimmed, fmt) {
            return Some(Utc.from_utc_datetime(&dt));
        }
    }

    // 3. Try Syslog format (e.g. "Jul 26 18:00:00" or "Jul 26 18:00:00.123")
    // Prepend current year since syslog timestamps lack year
    let current_year = Utc::now().format("%Y").to_string();
    let syslog_str = format!("{} {}", current_year, trimmed);
    let syslog_formats = [
        "%Y %b %d %H:%M:%S%.f",
        "%Y %b %d %H:%M:%S",
        "%Y %b %e %H:%M:%S%.f",
        "%Y %b %e %H:%M:%S",
    ];

    for fmt in &syslog_formats {
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&syslog_str, fmt) {
            return Some(Utc.from_utc_datetime(&dt));
        }
    }

    // 4. Try Unix Epoch (seconds or milliseconds)
    if let Ok(val) = trimmed.parse::<i64>() {
        if val > 1_000_000_000_000 {
            // Milliseconds
            if let Some(dt) = DateTime::from_timestamp_millis(val) {
                return Some(dt);
            }
        } else if val > 0 {
            // Seconds
            if let Some(dt) = DateTime::from_timestamp(val, 0) {
                return Some(dt);
            }
        }
    }

    // Float epoch seconds e.g. "1719829200.123"
    if let Ok(val) = trimmed.parse::<f64>() {
        let secs = val.trunc() as i64;
        let nsecs = (val.fract() * 1_000_000_000.0) as u32;
        if secs > 0 {
            if let Some(dt) = DateTime::from_timestamp(secs, nsecs) {
                return Some(dt);
            }
        }
    }

    debug!("Failed to parse timestamp from raw string: '{}'", raw);
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rfc3339() {
        let ts = parse_timestamp("2026-07-26T18:00:00Z").unwrap();
        assert_eq!(ts.format("%Y-%m-%d %H:%M:%S").to_string(), "2026-07-26 18:00:00");
    }

    #[test]
    fn test_parse_syslog() {
        let ts = parse_timestamp("Jul 26 18:00:00").unwrap();
        assert_eq!(ts.format("%b %d %H:%M:%S").to_string(), "Jul 26 18:00:00");
    }

    #[test]
    fn test_parse_epoch() {
        let ts = parse_timestamp("1719829200").unwrap();
        assert_eq!(ts.timestamp(), 1719829200);
    }
}
