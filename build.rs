use std::time::{SystemTime, UNIX_EPOCH};
use time::OffsetDateTime;

fn main() {
    for path in ["build.rs", "Cargo.toml", "Cargo.lock", "src", "examples"] {
        println!("cargo:rerun-if-changed={path}");
    }
    println!("cargo:rerun-if-env-changed=SOURCE_DATE_EPOCH");
    let epoch = match std::env::var("SOURCE_DATE_EPOCH") {
        Ok(value) => value
            .parse::<u64>()
            .expect("SOURCE_DATE_EPOCH must be Unix seconds"),
        Err(_) => SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before Unix epoch")
            .as_secs(),
    };
    let timestamp = OffsetDateTime::from_unix_timestamp(
        i64::try_from(epoch).expect("build timestamp out of range"),
    )
    .expect("build timestamp out of range");
    println!("cargo:rustc-env=MWT_JV_BUILD_EPOCH={epoch}");
    println!(
        "cargo:rustc-env=MWT_JV_BUILD_TIME={} {:02}:{:02}:{:02} UTC",
        timestamp.date(),
        timestamp.hour(),
        timestamp.minute(),
        timestamp.second()
    );
}
