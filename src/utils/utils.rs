pub fn format_duration(ms: u64) -> String {
    let s = ms / 1000;
    format!("{:02}:{:02}", s / 60, s % 60)
}

pub fn format_number(num: u64) -> String {
    match num {
        n if n >= 100_000_000 => format!("{:.1}亿", n as f64 / 100_000_000.0),
        n if n >= 10_000 => format!("{:.1}万", n as f64 / 10_000.0),
        _ => num.to_string(),
    }
}
