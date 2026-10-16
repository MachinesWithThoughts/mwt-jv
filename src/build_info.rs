use std::time::{SystemTime, UNIX_EPOCH};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const TIMESTAMP: &str = env!("MWT_JV_BUILD_TIME");

fn relative(build: u64, now: u64) -> String {
    let seconds = now.abs_diff(build);
    if seconds < 5 {
        return "just now".into();
    }
    let (count, unit) = match seconds {
        0..60 => (seconds, "s"),
        60..3600 => (seconds / 60, "m"),
        3600..86400 => (seconds / 3600, "h"),
        _ => (seconds / 86400, "d"),
    };
    if now >= build {
        format!("{count}{unit} ago")
    } else {
        format!("in {count}{unit}")
    }
}

pub fn age() -> String {
    let build = env!("MWT_JV_BUILD_EPOCH")
        .parse()
        .expect("valid build epoch");
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    relative(build, now)
}

pub fn header(available: usize) -> String {
    let age = age();
    let full = format!("v{VERSION} · built {TIMESTAMP} ({age})");
    let short = format!("v{VERSION} · {age}");
    if full.chars().count() <= available {
        full
    } else if short.chars().count() <= available {
        short
    } else {
        format!("v{VERSION}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_build_age_handles_unit_boundaries_and_future_clocks() {
        let build = 1_000_000;
        for (elapsed, expected) in [
            (0, "just now"),
            (5, "5s ago"),
            (59, "59s ago"),
            (60, "1m ago"),
            (3599, "59m ago"),
            (3600, "1h ago"),
            (86400, "1d ago"),
        ] {
            assert_eq!(relative(build, build + elapsed), expected);
        }
        assert_eq!(relative(build, build - 120), "in 2m");
    }
}
