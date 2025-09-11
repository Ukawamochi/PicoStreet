//! ユーザー設定
//! 補足:
//! - デベロッパーモード(true): API送信は30秒ごと。ログ詳細。動作確認に便利。
//! - 通常モード(false): API送信は毎日 午前3時(JST)。実運用向け。
//! - APIエンドポイント: あなたのサーバのURL/ポート/パスに合わせてください。
//! - WiFi情報: `Steps/wifi_config.rs` に SSID/パスワードを設定してください（このファイルでは変更しません）。
//! 
//! このファイル名をsettings_example.rsからsettins.rsに変更して使用してください。
//! 

/// 初心者向け: デベロッパーモードを切り替える（true か false を変えるだけ）
pub const DEVELOPER_MODE: bool = true; // true=30秒毎送信 / false=毎日3時に送信

/// APIサーバのホスト名 or IP
pub const API_HOST: &str = "192.168.1.23"; // 例: "192.168.1.23" や "example.com"

/// APIサーバのポート番号
pub const API_PORT: u16 = 3000; // 例: 80, 3000, 8080 など

/// APIのパス
pub const API_PATH: &str = "/"; // 例: "/api/encounters"

/// デベロッパーモードかどうか（内部用）
#[inline]
pub fn is_developer_mode() -> bool { DEVELOPER_MODE }

/// WiFi アクセスポイントのSSID（ネットワーク名）
pub const WIFI_SSID: &str = "AP_NAME"; // ← WiFiのSSIDを入絵よく

/// WiFi パスワード（PSK）
pub const WIFI_PSK: &str = "PASSWORD"; // ← パスワードを入力