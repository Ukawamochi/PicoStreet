//! WiFi credentials (kept out of VCS)
//!
//! Fill in your WiFi SSID and PSK before flashing.
//! This file is intentionally excluded from git via `.gitignore`.
//!
//! Safety: Avoid committing real credentials. For multiple networks, you can
//! extend this to select by environment or board variant.

/// WiFi SSID used for connection.
pub const WIFI_SSID: &str = "YOUR_WIFI_SSID";

/// WiFi PSK (password) used for connection.
pub const WIFI_PSK: &str = "YOUR_WIFI_PASSWORD";

