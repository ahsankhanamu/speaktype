const MIB: f64 = 1024.0 * 1024.0;
const GIB: f64 = MIB * 1024.0;

fn format_scaled(value: f64, unit: &str) -> String {
    if value >= 100.0 || (value - value.round()).abs() < 0.05 {
        format!("{:.0} {}", value.round(), unit)
    } else {
        format!("{:.1} {}", value, unit)
    }
}

/// Human-readable binary size (e.g. `75 MB`, `1.5 GB`).
pub fn format_byte_size(bytes: u64) -> String {
    let bytes_f = bytes as f64;
    if bytes_f >= GIB {
        format_scaled(bytes_f / GIB, "GB")
    } else {
        format_scaled(bytes_f / MIB, "MB")
    }
}

/// Estimated download size for catalog entries (e.g. `~75 MB`).
pub fn format_byte_size_approx(bytes: u64) -> String {
    format!("~{}", format_byte_size(bytes))
}

const MIB_U64: u64 = 1024 * 1024;
const GIB_U64: u64 = MIB_U64 * 1024;

pub fn catalog_model_size_bytes(model_id: &str) -> u64 {
    match model_id {
        "tiny.en" | "tiny" => 75 * MIB_U64,
        "base.en" | "base" => 150 * MIB_U64,
        "small.en" | "small" => 500 * MIB_U64,
        "medium.en" | "medium" | "large-v3-turbo" => GIB_U64 + GIB_U64 / 2,
        "large-v3" => 3 * GIB_U64,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_megabytes() {
        assert_eq!(format_byte_size(75 * 1024 * 1024), "75 MB");
        assert_eq!(format_byte_size(150 * 1024 * 1024), "150 MB");
        assert_eq!(format_byte_size(465 * 1024 * 1024), "465 MB");
    }

    #[test]
    fn formats_gigabytes() {
        assert_eq!(format_byte_size(GIB_U64 + GIB_U64 / 2), "1.5 GB");
        assert_eq!(format_byte_size(3 * GIB_U64), "3 GB");
    }
}
