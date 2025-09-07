//! TLV定義とビルダ/パーサ
//! - Service Data(0x16) の中に格納する自前フレーム（ver/type/flags/rsv + TLVs）
//! - 31B 制限に注意（本PoCは CONTACT_ID のみ）

use core::fmt;

use crate::constants::{CONTACT_ID, SERVICE_UUID_16};

/// TLV 種別
#[repr(u8)]
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum TlvType {
    ContactId = 0x01,
    EventId = 0x02,
    EpochMin = 0x03,
    Flags2 = 0x04,
    TagSig = 0x10,
}

/// 解析結果（今後拡張予定）
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Parsed {
    pub version: u8,
    pub msg_type: u8,
    pub flags: u8,
    pub rsv: u8,
    pub contact_id: [u8; 16],
}

impl fmt::Debug for Parsed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Parsed {{ ver: {}, type: {}, flags: 0x{:02x}, rsv: {}, contact_id: {:02X?} }}",
            self.version, self.msg_type, self.flags, self.rsv, self.contact_id
        )
    }
}

/// Service Data 内に格納する自前フレームを構築
/// buf に書き込み、書き込んだサイズを返す
/// 構造: [ver(1), type(1), flags(1), rsv(1), TLVs...]
pub fn build_adv_payload(buf: &mut [u8]) -> usize {
    let mut w = 0usize;
    // ヘッダ
    if buf.len() < 4 { return 0; }
    buf[w] = 0x01; w += 1; // ver
    buf[w] = 0x01; w += 1; // type = Beacon/Hello
    buf[w] = 0x00; w += 1; // flags
    buf[w] = 0x00; w += 1; // rsv
    // CONTACT_ID TLV
    let need = 2 + CONTACT_ID.len();
    if buf.len() < w + need { return 0; }
    buf[w] = TlvType::ContactId as u8; w += 1;
    buf[w] = CONTACT_ID.len() as u8; w += 1;
    buf[w..w+CONTACT_ID.len()].copy_from_slice(&CONTACT_ID);
    w += CONTACT_ID.len();
    w
}

/// AD全体（[len][type][data]...）を走査して Service Data 0x16 のうち
/// UUID=SERVICE_UUID_16 のペイロードをパース。
/// 見つかったら Parsed を返す。
pub fn parse_service_data(ad: &[u8]) -> Option<Parsed> {
    let mut i = 0usize;
    while i < ad.len() {
        let len = ad[i] as usize; i += 1;
        if len == 0 { continue; }
        if i + len > ad.len() { break; }
        let ty = ad[i];
        let data = &ad[i+1 .. i+len];
        i += len;

        if ty != 0x16 { continue; } // Service Data - 16-bit UUID
        if data.len() < 2 { continue; }
        let uuid = u16::from_le_bytes([data[0], data[1]]);
        if uuid != SERVICE_UUID_16 { continue; }
        let payload = &data[2..];
        // ヘッダ
        if payload.len() < 4 { continue; }
        let ver = payload[0];
        let msg_type = payload[1];
        let flags = payload[2];
        let rsv = payload[3];
        if ver != 0x01 || msg_type != 0x01 { continue; }

        // TLV 反復
        let mut off = 4usize;
        let mut found: Option<[u8; 16]> = None;
        while off + 2 <= payload.len() {
            let t = payload[off];
            let l = payload[off + 1] as usize;
            off += 2;
            if off + l > payload.len() { break; }
            if t == TlvType::ContactId as u8 {
                if l == 16 {
                    let mut id = [0u8; 16];
                    id.copy_from_slice(&payload[off..off+16]);
                    found = Some(id);
                }
            }
            off += l;
        }

        if let Some(contact_id) = found {
            return Some(Parsed { version: ver, msg_type, flags, rsv, contact_id });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn build_service_data_ad(payload: &[u8]) -> heapless::Vec<u8, 64> {
        let mut ad: heapless::Vec<u8, 64> = heapless::Vec::new();
        // Flags 0x01
        ad.extend_from_slice(&[0x02, 0x01, 0x06]).unwrap();
        // Complete 16-bit UUIDs (0x03) with SERVICE_UUID_16 (LE)
        ad.extend_from_slice(&[0x03, 0x03, (SERVICE_UUID_16 & 0xFF) as u8, (SERVICE_UUID_16 >> 8) as u8]).unwrap();
        // Service Data 0x16: len = 1(type) + 2(uuid) + payload.len()
        let len = 1 + 2 + payload.len();
        ad.extend_from_slice(&[len as u8, 0x16, (SERVICE_UUID_16 & 0xFF) as u8, (SERVICE_UUID_16 >> 8) as u8]).unwrap();
        ad.extend_from_slice(payload).unwrap();
        ad
    }

    #[test]
    fn build_and_parse_contact_only() {
        let mut buf = [0u8; 32];
        let n = build_adv_payload(&mut buf);
        assert!(n > 0);
        // 先頭4Bヘッダ検証
        assert_eq!(&buf[..4], &[0x01, 0x01, 0x00, 0x00]);
        // TLV (T=1, L=16)
        assert_eq!(buf[4], TlvType::ContactId as u8);
        assert_eq!(buf[5], 16);
        assert_eq!(&buf[6..6+16], &CONTACT_ID);

        let ad = build_service_data_ad(&buf[..n]);
        let parsed = parse_service_data(&ad).expect("must parse");
        assert_eq!(parsed.version, 0x01);
        assert_eq!(parsed.msg_type, 0x01);
        assert_eq!(parsed.flags, 0x00);
        assert_eq!(parsed.rsv, 0x00);
        assert_eq!(parsed.contact_id, CONTACT_ID);
    }

    #[test]
    fn parse_ignores_other_uuid() {
        // Build Service Data with other UUID
        let mut payload = [0u8; 32];
        let n = build_adv_payload(&mut payload);
        let mut ad: heapless::Vec<u8, 64> = heapless::Vec::new();
        // Service Data 0x16, UUID=0xBEEF
        ad.extend_from_slice(&[2 + 1 + n as u8, 0x16, 0xEF, 0xBE]).unwrap();
        ad.extend_from_slice(&payload[..n]).unwrap();
        assert!(parse_service_data(&ad).is_none());
    }

    #[test]
    fn parse_bounds_checks() {
        // Broken AD
        assert!(parse_service_data(&[0x02, 0x16]).is_none());
        // Correct header but short TLV
        let mut p = [0u8; 8];
        p[..4].copy_from_slice(&[1,1,0,0]);
        p[4..6].copy_from_slice(&[TlvType::ContactId as u8, 15]); // too short
        let mut ad = heapless::Vec::<u8,64>::new();
        ad.extend_from_slice(&[2 + 1 + p.len() as u8, 0x16, (SERVICE_UUID_16 & 0xFF) as u8, (SERVICE_UUID_16 >> 8) as u8]).unwrap();
        ad.extend_from_slice(&p).unwrap();
        assert!(parse_service_data(&ad).is_none());
    }
}

