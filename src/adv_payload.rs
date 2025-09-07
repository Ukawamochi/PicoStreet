//! PicoStreet X交換特化プロトコル
//! - Service Data(0x16) の中に8バイト固定ペイロード
//! - Version(1) + DeviceType(1) + BD_ADDR(6)

use core::fmt;

use crate::constants::SERVICE_UUID_16;

/// 解析結果（簡素化版）
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Parsed {
    pub version: u8,
    pub device_type: u8,
    pub bd_addr: [u8; 6],
}

impl fmt::Debug for Parsed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Parsed {{ ver: {}, dev_type: 0x{:02X}, bd_addr: {:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X} }}",
            self.version, self.device_type,
            self.bd_addr[0], self.bd_addr[1], self.bd_addr[2],
            self.bd_addr[3], self.bd_addr[4], self.bd_addr[5]
        )
    }
}

/// Service Data 内に格納する8バイト固定ペイロードを構築
/// buf に書き込み、書き込んだサイズを返す
/// 構造: [version(1), device_type(1), bd_addr(6)]
pub fn build_adv_payload(buf: &mut [u8], bd_addr: &[u8; 6]) -> usize {
    if buf.len() < 8 { 
        return 0; 
    }
    
    buf[0] = 0x01; // Version
    buf[1] = 0x50; // DeviceType: PicoStreet ('P' = 0x50)
    buf[2..8].copy_from_slice(bd_addr); // BD_ADDR (6 bytes)
    
    8 // 8バイト固定
}

/// AD全体（[len][type][data]...）を走査して Service Data 0x16 のうち
/// UUID=SERVICE_UUID_16 のペイロードをパース。
/// 見つかったら Parsed を返す。
pub fn parse_service_data(ad: &[u8]) -> Option<Parsed> {
    let mut i = 0usize;
    while i < ad.len() {
        let len = ad[i] as usize; 
        i += 1;
        if len == 0 { 
            continue; 
        }
        if i + len > ad.len() { 
            break; 
        }
        
        let ty = ad[i];
        let data = &ad[i+1 .. i+len];
        i += len;

        if ty != 0x16 { 
            continue; // Service Data - 16-bit UUID
        }
        if data.len() < 2 { 
            continue; 
        }
        
        let uuid = u16::from_le_bytes([data[0], data[1]]);
        if uuid != SERVICE_UUID_16 { 
            continue; 
        }
        
        let payload = &data[2..];
        
        // PicoStreetペイロード検証: 8バイト固定
        if payload.len() != 8 { 
            continue; 
        }
        
        let version = payload[0];
        let device_type = payload[1];
        
        // Version=0x01, DeviceType=0x50 を確認
        if version != 0x01 || device_type != 0x50 { 
            continue; 
        }
        
        let mut bd_addr = [0u8; 6];
        bd_addr.copy_from_slice(&payload[2..8]);
        
        return Some(Parsed {
            version,
            device_type,
            bd_addr,
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    // テスト用BD_ADDR
    const TEST_BD_ADDR: [u8; 6] = [0x28, 0xCD, 0xC1, 0x15, 0x26, 0x11];

    fn build_service_data_ad(payload: &[u8]) -> heapless::Vec<u8, 64> {
        let mut ad: heapless::Vec<u8, 64> = heapless::Vec::new();
        // Flags 0x01
        ad.extend_from_slice(&[0x02, 0x01, 0x06]).unwrap();
        // Service Data 0x16: len = 1(type) + 2(uuid) + payload.len()
        let len = 1 + 2 + payload.len();
        ad.extend_from_slice(&[len as u8, 0x16, (SERVICE_UUID_16 & 0xFF) as u8, (SERVICE_UUID_16 >> 8) as u8]).unwrap();
        ad.extend_from_slice(payload).unwrap();
        ad
    }

    #[test]
    fn build_and_parse_picostreet() {
        let mut buf = [0u8; 8];
        let n = build_adv_payload(&mut buf, &TEST_BD_ADDR);
        assert_eq!(n, 8);
        
        // ヘッダ検証
        assert_eq!(buf[0], 0x01); // Version
        assert_eq!(buf[1], 0x50); // DeviceType (PicoStreet)
        assert_eq!(&buf[2..8], &TEST_BD_ADDR); // BD_ADDR

        let ad = build_service_data_ad(&buf[..n]);
        let parsed = parse_service_data(&ad).expect("must parse");
        assert_eq!(parsed.version, 0x01);
        assert_eq!(parsed.device_type, 0x50);
        assert_eq!(parsed.bd_addr, TEST_BD_ADDR);
    }

    #[test]
    fn parse_ignores_other_uuid() {
        // Build Service Data with other UUID
        let mut payload = [0u8; 8];
        let n = build_adv_payload(&mut payload, &TEST_BD_ADDR);
        let mut ad: heapless::Vec<u8, 64> = heapless::Vec::new();
        // Service Data 0x16, UUID=0xBEEF
        ad.extend_from_slice(&[2 + 1 + n as u8, 0x16, 0xEF, 0xBE]).unwrap();
        ad.extend_from_slice(&payload[..n]).unwrap();
        assert!(parse_service_data(&ad).is_none());
    }

    #[test]
    fn parse_ignores_wrong_device_type() {
        let mut ad: heapless::Vec<u8, 64> = heapless::Vec::new();
        // Service Data with wrong DeviceType
        let payload = [0x01, 0x99, 0x28, 0xCD, 0xC1, 0x15, 0x26, 0x11]; // DeviceType=0x99
        ad.extend_from_slice(&[2 + 1 + payload.len() as u8, 0x16, (SERVICE_UUID_16 & 0xFF) as u8, (SERVICE_UUID_16 >> 8) as u8]).unwrap();
        ad.extend_from_slice(&payload).unwrap();
        assert!(parse_service_data(&ad).is_none());
    }

    #[test]
    fn parse_bounds_checks() {
        // Broken AD
        assert!(parse_service_data(&[0x02, 0x16]).is_none());
        
        // Correct header but wrong payload size
        let payload = [0x01, 0x50, 0x28, 0xCD, 0xC1]; // Only 5 bytes instead of 8
        let mut ad = heapless::Vec::<u8,64>::new();
        ad.extend_from_slice(&[2 + 1 + payload.len() as u8, 0x16, (SERVICE_UUID_16 & 0xFF) as u8, (SERVICE_UUID_16 >> 8) as u8]).unwrap();
        ad.extend_from_slice(&payload).unwrap();
        assert!(parse_service_data(&ad).is_none());
    }
}
