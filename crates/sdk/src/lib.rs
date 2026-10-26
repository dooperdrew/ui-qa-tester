#![forbid(unsafe_code)]
//! Optional lightweight telemetry and synthetic input bridge crate for Rust native GUI
//! applications (`iced`, `egui`, `Slint`).

pub fn sdk_protocol_version() -> u32 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_version_is_one() {
        assert_eq!(sdk_protocol_version(), 1);
    }
}
