#![cfg_attr(not(test), no_std)]

/// 共通定数（プロトコル／ID など）
pub mod constants {
    /// Service UUID (16-bit, LEエンディアン)
    pub const SERVICE_UUID_16: u16 = 0xF00D;

    /// PoC用 16バイト CONTACT_ID
    /// 仕様の例は17文字のため、16バイトに収まる ID を採用
    pub const CONTACT_ID: [u8; 16] = *b"DEMO-DEMO-DEMO-1"; // 16B
}

pub mod adv_payload;

