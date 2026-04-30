pub fn sanitize(s: &str) -> String {
    s.replace('"', "'")
}

pub fn parse_f64(value: &str) -> Option<f64> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    trimmed.parse::<f64>().ok()
}

pub fn parse_mpris_length(value: &str) -> Option<f64> {
    let micros = parse_f64(value)?;
    Some(micros / 1_000_000.0)
}

pub fn parse_mpris_position(value: &str) -> Option<f64> {
    let raw = parse_f64(value)?;

    if raw.abs() > 100_000.0 {
        Some(raw / 1_000_000.0)
    } else {
        Some(raw)
    }
}
