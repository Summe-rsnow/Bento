//! Pure display formatters for file sizes, item counts, and Windows FILETIME timestamps.

/// 100-nanosecond ticks per second.
const TICKS_PER_SEC: i64 = 10_000_000;
/// Seconds between 1601-01-01T00:00:00Z and 1970-01-01T00:00:00Z (`134_774 * 86_400`).
const FILETIME_UNIX_EPOCH_SECS: i64 = 11_644_473_600;
/// Maximum year supported by Windows `SYSTEMTIME`.
const MAX_SYSTEMTIME_YEAR: i64 = 30_827;

const BYTE_UNITS: &[(u64, &str, usize)] = &[
    (1u64 << 60, "EB", 2),
    (1u64 << 50, "PB", 2),
    (1u64 << 40, "TB", 2),
    (1u64 << 30, "GB", 2),
    (1u64 << 20, "MB", 1),
    (1u64 << 10, "KB", 1),
];

/// Formats a byte count into a human-readable IEC-binary size string
/// (`"0 B"`, `"512 B"`, `"1.4 KB"`, `"24.8 MB"`, `"3.12 GB"`, `"1.05 TB"`).
pub fn format_bytes(bytes: u64) -> String {
    for &(threshold, unit, decimals) in BYTE_UNITS {
        if bytes >= threshold {
            let value = bytes as f64 / threshold as f64;
            return format!("{value:.decimals$} {unit}");
        }
    }
    format!("{bytes} B")
}

/// Formats an unsigned integer with ASCII comma thousands separators (e.g. `"1,234,567"`).
pub fn format_count(n: u64) -> String {
    let digits = n.to_string();
    let bytes = digits.as_bytes();
    let len = bytes.len();
    let mut out = String::with_capacity(len + len.saturating_sub(1) / 3);
    for (i, &b) in bytes.iter().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            out.push(',');
        }
        out.push(b as char);
    }
    out
}

/// Gregorian `(year, month, day)` for a day count since `1970-01-01`
/// (Howard Hinnant's `civil_from_days`).
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

fn filetime_to_unix_secs(ft: i64) -> Option<i64> {
    if ft <= 0 {
        return None;
    }
    Some(ft.div_euclid(TICKS_PER_SEC) - FILETIME_UNIX_EPOCH_SECS)
}

/// Formats a Windows `FILETIME` (100-ns intervals since 1601-01-01 UTC) in UTC
/// as `"YYYY-MM-DD HH:MM"`. Returns `None` for non-positive or out-of-range timestamps.
pub fn format_filetime_utc(ft: i64) -> Option<String> {
    let unix_secs = filetime_to_unix_secs(ft)?;
    let days = unix_secs.div_euclid(86_400);
    let secs_of_day = unix_secs.rem_euclid(86_400);
    let hour = secs_of_day / 3_600;
    let minute = (secs_of_day % 3_600) / 60;
    let (year, month, day) = civil_from_days(days);
    if !(1601..=MAX_SYSTEMTIME_YEAR).contains(&year) {
        return None;
    }
    Some(format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}"))
}

/// Formats a Windows `FILETIME` (100-ns intervals since 1601-01-01 UTC) in the
/// system's local timezone (respecting historical DST rules) as `"YYYY-MM-DD HH:MM"`.
pub fn format_filetime_local(ft: i64) -> Option<String> {
    if ft <= 0 {
        return None;
    }

    #[cfg(target_os = "windows")]
    {
        #[repr(C)]
        struct FileTime {
            dw_low_date_time: u32,
            dw_high_date_time: u32,
        }

        #[repr(C)]
        #[derive(Default)]
        struct SystemTime {
            w_year: u16,
            w_month: u16,
            w_day_of_week: u16,
            w_day: u16,
            w_hour: u16,
            w_minute: u16,
            w_second: u16,
            w_milliseconds: u16,
        }

        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn FileTimeToSystemTime(
                lp_file_time: *const FileTime,
                lp_system_time: *mut SystemTime,
            ) -> i32;
            fn SystemTimeToTzSpecificLocalTime(
                lp_time_zone_information: *const std::ffi::c_void,
                lp_universal_time: *const SystemTime,
                lp_local_time: *mut SystemTime,
            ) -> i32;
        }

        let raw = ft as u64;
        let file_time = FileTime {
            dw_low_date_time: raw as u32,
            dw_high_date_time: (raw >> 32) as u32,
        };
        let mut utc = SystemTime::default();
        let mut local = SystemTime::default();
        // SAFETY: Both pointers reference valid initialized stack structs of exact Win32 ABI layout.
        let ok = unsafe {
            FileTimeToSystemTime(&file_time, &mut utc) != 0
                && SystemTimeToTzSpecificLocalTime(std::ptr::null(), &utc, &mut local) != 0
        };
        if !ok {
            return None;
        }
        Some(format!(
            "{:04}-{:02}-{:02} {:02}:{:02}",
            local.w_year, local.w_month, local.w_day, local.w_hour, local.w_minute
        ))
    }

    #[cfg(all(unix, not(target_os = "windows")))]
    {
        use std::ffi::c_int;

        #[repr(C)]
        struct Tm {
            tm_sec: c_int,
            tm_min: c_int,
            tm_hour: c_int,
            tm_mday: c_int,
            tm_mon: c_int,
            tm_year: c_int,
            tm_wday: c_int,
            tm_yday: c_int,
            tm_isdst: c_int,
            // Trailing space for glibc / BSD / macOS `tm_gmtoff` & `tm_zone`.
            _extra: [usize; 4],
        }

        unsafe extern "C" {
            fn localtime_r(timep: *const i64, result: *mut Tm) -> *mut Tm;
        }

        let unix_secs = filetime_to_unix_secs(ft)?;
        let (utc_year, _, _) = civil_from_days(unix_secs.div_euclid(86_400));
        if !(1601..=MAX_SYSTEMTIME_YEAR).contains(&utc_year) {
            return None;
        }
        let mut tm = Tm {
            tm_sec: 0,
            tm_min: 0,
            tm_hour: 0,
            tm_mday: 0,
            tm_mon: 0,
            tm_year: 0,
            tm_wday: 0,
            tm_yday: 0,
            tm_isdst: 0,
            _extra: [0; 4],
        };
        // SAFETY: `unix_secs` and `tm` are valid aligned stack values; `localtime_r` initializes `tm`.
        let ptr = unsafe { localtime_r(&unix_secs, &mut tm) };
        if ptr.is_null() {
            return None;
        }
        let year = tm.tm_year + 1900;
        let month = tm.tm_mon + 1;
        let day = tm.tm_mday;
        Some(format!(
            "{year:04}-{month:02}-{day:02} {:02}:{:02}",
            tm.tm_hour, tm.tm_min
        ))
    }

    #[cfg(not(any(target_os = "windows", unix)))]
    {
        format_filetime_utc(ft)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filetime_from_unix_secs(unix_secs: i64) -> i64 {
        (unix_secs + FILETIME_UNIX_EPOCH_SECS) * TICKS_PER_SEC
    }

    #[test]
    fn formats_byte_sizes_across_scales() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1023), "1023 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1434), "1.4 KB");
        assert_eq!(format_bytes((24.8 * 1024.0 * 1024.0) as u64), "24.8 MB");
        assert_eq!(
            format_bytes((3.12 * 1024.0 * 1024.0 * 1024.0) as u64),
            "3.12 GB"
        );
        assert_eq!(
            format_bytes((1.05 * 1024.0 * 1024.0 * 1024.0 * 1024.0) as u64),
            "1.05 TB"
        );
        assert_eq!(format_bytes(2 * (1u64 << 50)), "2.00 PB");
    }

    #[test]
    fn formats_counts_with_thousands_separators() {
        assert_eq!(format_count(0), "0");
        assert_eq!(format_count(9), "9");
        assert_eq!(format_count(999), "999");
        assert_eq!(format_count(1_000), "1,000");
        assert_eq!(format_count(12_345), "12,345");
        assert_eq!(format_count(1_234_567), "1,234,567");
        assert_eq!(format_count(u64::MAX), "18,446,744,073,709,551,615");
    }

    #[test]
    fn formats_filetime_utc_dates_and_rejects_invalid() {
        assert_eq!(format_filetime_utc(0), None);
        assert_eq!(format_filetime_utc(-100), None);
        assert_eq!(format_filetime_utc(i64::MAX), None);

        // 1970-01-01 00:00:00 UTC
        assert_eq!(
            format_filetime_utc(filetime_from_unix_secs(0)),
            Some("1970-01-01 00:00".to_owned())
        );

        // 2026-11-23 12:34:00 UTC (epoch seconds verified against Gregorian calendar)
        // Days 1970-01-01 .. 2026-11-23: 20,780 days -> 20_780 * 86_400 + 12*3600 + 34*60
        let ts = 20_780 * 86_400 + 12 * 3_600 + 34 * 60;
        assert_eq!(
            format_filetime_utc(filetime_from_unix_secs(ts)),
            Some("2026-11-23 12:34".to_owned())
        );

        // Leap day 2024-02-29 23:59 UTC (19,782 days from 1970-01-01)
        let leap_ts = 19_782 * 86_400 + 23 * 3_600 + 59 * 60;
        assert_eq!(
            format_filetime_utc(filetime_from_unix_secs(leap_ts)),
            Some("2024-02-29 23:59".to_owned())
        );

        // Pre-1970 valid Windows FILETIME: 1601-01-01 00:00:01 UTC
        assert_eq!(
            format_filetime_utc(TICKS_PER_SEC),
            Some("1601-01-01 00:00".to_owned())
        );
    }

    #[test]
    fn formats_filetime_local_dates_and_rejects_invalid() {
        assert_eq!(format_filetime_local(0), None);
        assert_eq!(format_filetime_local(-1), None);
        assert_eq!(format_filetime_local(i64::MAX), None);

        let ft = filetime_from_unix_secs(1_700_000_000); // 2023-11-14 22:13:20 UTC
        let formatted = format_filetime_local(ft).expect("valid local filetime");
        assert_eq!(formatted.len(), 16);
        assert_eq!(&formatted[4..5], "-");
        assert_eq!(&formatted[7..8], "-");
        assert_eq!(&formatted[10..11], " ");
        assert_eq!(&formatted[13..14], ":");
        assert!(formatted[0..4].chars().all(|c| c.is_ascii_digit()));
        assert!(formatted[5..7].chars().all(|c| c.is_ascii_digit()));
        assert!(formatted[8..10].chars().all(|c| c.is_ascii_digit()));
        assert!(formatted[11..13].chars().all(|c| c.is_ascii_digit()));
        assert!(formatted[14..16].chars().all(|c| c.is_ascii_digit()));
    }
}
