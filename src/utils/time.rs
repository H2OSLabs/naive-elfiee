/// Time utility functions for Elfiee
///
/// This module provides timezone-aware timestamp utilities.
/// **IMPORTANT**: This does NOT handle Vector Clock timestamps (Event.timestamp),
/// which are used for conflict resolution and remain as HashMap<String, i64>.
///
/// All business-level timestamps should use these functions to ensure:
/// - Consistent RFC 3339 (ISO 8601 with timezone) formatting
/// - Proper timezone handling across different environments
/// - Easy parsing and conversion between timezones
use chrono::{DateTime, Local, Utc};
use std::time::SystemTime;

/// Generate current UTC timestamp in RFC 3339 format.
///
/// This is the **primary function** for getting current UTC timestamps.
/// All business logic should use this function to ensure consistent formatting.
///
/// # Format
/// RFC 3339: "2025-12-17T02:30:00Z"
///
/// # Example
/// ```
/// use elfiee_lib::utils::time;
/// let timestamp = time::now_utc();
/// assert!(timestamp.ends_with('Z'));
/// ```
pub fn now_utc() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// Get current UTC DateTime object (internal use only).
///
/// **CAUTION**: This function is for internal use where DateTime operations are required.
/// For most cases, use `now_utc()` to get RFC 3339 string directly.
///
/// Only exposed for `models/command.rs` where Command.timestamp is DateTime<Utc>.
pub(crate) fn now_utc_datetime() -> DateTime<Utc> {
    Utc::now()
}

/// Generate current local timezone timestamp in RFC 3339 format (internal use).
///
/// **CAUTION**: Only for testing. Production code should use UTC timestamps.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn now_local() -> String {
    Local::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// Parse RFC 3339 timestamp and convert to UTC DateTime (internal use).
///
/// **CAUTION**: Only for testing. Production code should work with string timestamps.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn parse_to_utc(timestamp: &str) -> Result<DateTime<Utc>, String> {
    DateTime::parse_from_rfc3339(timestamp)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| format!("Invalid timestamp '{}': {}", timestamp, e))
}

/// Parse RFC 3339 timestamp and convert to local timezone DateTime (internal use).
///
/// **CAUTION**: Only for testing.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn parse_to_local(timestamp: &str) -> Result<DateTime<Local>, String> {
    DateTime::parse_from_rfc3339(timestamp)
        .map(|dt| dt.with_timezone(&Local))
        .map_err(|e| format!("Invalid timestamp '{}': {}", timestamp, e))
}

/// Convert SystemTime to RFC 3339 UTC timestamp string (internal use).
///
/// Used by `commands/file.rs` for converting filesystem timestamps.
/// Not exposed to external crates to enforce using `now_utc()` as the primary API.
pub(crate) fn system_time_to_utc(system_time: SystemTime) -> Result<String, String> {
    let datetime: DateTime<Utc> = system_time.into();
    Ok(datetime.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
}

/// Convert SystemTime to RFC 3339 local timezone timestamp string (internal use).
///
/// **CAUTION**: Only for testing. Production code should use UTC timestamps.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn system_time_to_local(system_time: SystemTime) -> Result<String, String> {
    let datetime: DateTime<Local> = system_time.into();
    Ok(datetime.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
}

/// Format DateTime<Utc> to RFC 3339 string (internal use only).
///
/// **CAUTION**: This is an internal helper. Use `now_utc()` or `system_time_to_utc()` instead.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn format_utc(datetime: DateTime<Utc>) -> String {
    datetime.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// Format DateTime<Local> to RFC 3339 string (internal use only).
///
/// **CAUTION**: This is an internal helper. Use `now_local()` or `system_time_to_local()` instead.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn format_local(datetime: DateTime<Local>) -> String {
    datetime.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;

    #[test]
    fn test_now_utc() {
        let timestamp = now_utc();
        assert!(timestamp.ends_with('Z'), "UTC timestamp should end with Z");

        // Should be parseable
        let parsed = parse_to_utc(&timestamp);
        assert!(
            parsed.is_ok(),
            "Generated UTC timestamp should be parseable"
        );
    }

    #[test]
    fn test_now_local() {
        let timestamp = now_local();

        // Should contain timezone offset (either Z or +/-HH:MM)
        assert!(
            timestamp.ends_with('Z') || timestamp.contains('+') || timestamp.contains('-'),
            "Local timestamp should include timezone info"
        );

        // Should be parseable
        let parsed = parse_to_local(&timestamp);
        assert!(
            parsed.is_ok(),
            "Generated local timestamp should be parseable"
        );
    }

    #[test]
    fn test_parse_to_utc() {
        // Test UTC timestamp
        let result = parse_to_utc("2025-12-17T02:30:00Z");
        assert!(result.is_ok());
        let dt = result.unwrap();
        assert_eq!(dt.hour(), 2);
        assert_eq!(dt.minute(), 30);

        // Test timezone conversion
        let result = parse_to_utc("2025-12-17T10:30:00+08:00");
        assert!(result.is_ok());
        let dt = result.unwrap();
        assert_eq!(dt.hour(), 2); // 10:00 +08:00 = 02:00 UTC
    }

    #[test]
    fn test_parse_to_local() {
        let result = parse_to_local("2025-12-17T02:30:00Z");
        assert!(result.is_ok());
        // Actual local time depends on system timezone
    }

    #[test]
    fn test_parse_invalid_timestamp() {
        let result = parse_to_utc("invalid-timestamp");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid timestamp"));
    }

    #[test]
    fn test_system_time_conversion() {
        let now = SystemTime::now();

        // Test UTC conversion
        let utc_result = system_time_to_utc(now);
        assert!(utc_result.is_ok());
        let utc_str = utc_result.unwrap();
        assert!(utc_str.ends_with('Z') || utc_str.contains('+'));

        // Test local conversion
        let local_result = system_time_to_local(now);
        assert!(local_result.is_ok());
    }

    #[test]
    fn test_format_functions() {
        let utc_dt = Utc::now();
        let formatted = format_utc(utc_dt);
        assert!(formatted.ends_with('Z'));

        let local_dt = Local::now();
        let formatted = format_local(local_dt);
        assert!(formatted.contains('+') || formatted.contains('-') || formatted.ends_with('Z'));
    }

    #[test]
    fn test_roundtrip_utc() {
        // Generate -> Parse -> Format should be consistent
        let original = now_utc();
        let parsed = parse_to_utc(&original).unwrap();
        let formatted = format_utc(parsed);

        // Timestamps should be equivalent (within same second)
        let original_parsed = parse_to_utc(&original).unwrap();
        let formatted_parsed = parse_to_utc(&formatted).unwrap();
        assert_eq!(
            original_parsed.timestamp(),
            formatted_parsed.timestamp(),
            "Roundtrip should preserve timestamp"
        );
    }
}
