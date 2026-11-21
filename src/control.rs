//! 制御電文 (通信制御電文とファイル制御電文、各64バイト)。docs/spec-protocol.md §3.3, §3.4。

use crate::charcode::{Code, decode, encode};
use crate::util::{Clock, Result, bcd_decode, bcd_encode, proto};
use std::fmt;

// 通信制御電文の電文区分
pub const OPEN_REQ: u8 = 0x00;
pub const OPEN_RSP: u8 = 0x01;
pub const CLOSE_REQ: u8 = 0x02;
pub const CLOSE_RSP: u8 = 0x03;
pub const MODE_REQ: u8 = 0x04;
pub const MODE_RSP: u8 = 0x05;
// ファイル制御電文の電文区分
pub const START_REQ: u8 = 0x10;
pub const START_RSP: u8 = 0x11;
pub const END_REQ: u8 = 0x12;
pub const END_RSP: u8 = 0x13;
pub const RETRY_REQ: u8 = 0x14;

// 処理結果
pub const OK: u8 = 0x00;
pub const COMM_KIND_ERR: u8 = 0x10;
pub const PEER_CODE_ERR: u8 = 0x11;
pub const OWN_CODE_ERR: u8 = 0x12;
pub const PASSWORD_ERR: u8 = 0x14;
pub const APP_ERR: u8 = 0x15;
pub const MODE_ERR: u8 = 0x16;
pub const MODE_CHANGE_NG: u8 = 0x17;
pub const FILE_NAME_ERR: u8 = 0x11;
pub const FILE_KEY_ERR: u8 = 0x12;
pub const TEXT_COUNT_ERR: u8 = 0x13;
pub const RECORD_COUNT_ERR: u8 = 0x14;
pub const RECORD_LEN_ERR: u8 = 0x15;
pub const NO_FILE: u8 = 0x17;
pub const RECORD_ID_ERR: u8 = 0x18;
pub const COMPRESS_ERR: u8 = 0x19;
pub const OTHER_ERR: u8 = 0x99;

pub fn is_comm_kind(k: u8) -> bool {
    k <= MODE_RSP
}

/// センター確認コード: センターコード(10桁) + CPU/端末コード(4桁)。BCD で 7 バイト。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CenterCode {
    pub center: u64,
    pub cpu: u16,
}

impl CenterCode {
    pub fn parse(s: &str) -> Result<Self> {
        if s.len() != 14 || !s.bytes().all(|b| b.is_ascii_digit()) {
            return proto(format!("センター確認コードは14桁の数字: {s:?}"));
        }
        Ok(CenterCode {
            center: s[..10].parse().unwrap(),
            cpu: s[10..].parse().unwrap(),
        })
    }
    pub fn to_bytes(&self) -> [u8; 7] {
        let v = bcd_encode(&self.to_string()).unwrap();
        v.try_into().unwrap()
    }
    pub fn from_bytes(b: &[u8]) -> Result<Self> {
        match bcd_decode(b) {
            Some(s) => CenterCode::parse(&s),
            None => proto("センター確認コードが BCD でない"),
        }
    }
}

impl fmt::Display for CenterCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:010}{:04}", self.center, self.cpu)
    }
}

/// パスワード・アクセスキー (6バイト)。文字列は EBCDIC に変換して使う。
pub fn six(s: &str) -> Result<[u8; 6]> {
    let mut v = encode(Code::Ebcdic, s)?;
    if v.len() > 6 {
        return proto(format!("6文字以内: {s:?}"));
    }
    v.resize(6, 0x40);
    Ok(v.try_into().unwrap())
}

fn digit_byte(d: u8) -> u8 {
    0xF0 + d
}

fn byte_digit(b: u8) -> Result<u8> {
    if (0xF0..=0xF9).contains(&b) {
        Ok(b - 0xF0)
    } else {
        proto(format!("EBCDIC の数字でない: 0x{b:02X}"))
    }
}

/// 通信年月日時分秒 (BCD、西暦下2桁)。
pub fn now_bcd(clock: &Clock) -> [u8; 6] {
    let t = clock.now();
    let s = format!("{:02}{:02}{:02}{:02}{:02}{:02}", t.y % 100, t.mo, t.d, t.h, t.mi, t.s);
    bcd_encode(&s).unwrap().try_into().unwrap()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommCtl {
    pub kind: u8,
    pub result: u8,
    /// 相手センター確認コード (この電文の受信側)
    pub peer: CenterCode,
    /// 当方センター確認コード (この電文の送信側)
    pub own: CenterCode,
    pub time: [u8; 6],
    pub password: [u8; 6],
    /// アプリケーションID。0 = ファイル伝送
    pub app: u8,
    /// 0 = 連絡、1 = 照会
    pub mode: u8,
}

impl CommCtl {
    pub fn encode(&self) -> [u8; 64] {
        let mut b = [0u8; 64];
        b[0] = self.kind;
        b[1] = self.result;
        b[2..9].copy_from_slice(&self.peer.to_bytes());
        b[9..16].copy_from_slice(&self.own.to_bytes());
        b[16..22].copy_from_slice(&self.time);
        b[22..28].copy_from_slice(&self.password);
        b[28] = digit_byte(self.app);
        b[29] = digit_byte(self.mode);
        b
    }

    pub fn decode(b: &[u8]) -> Result<Self> {
        if b.len() != 64 {
            return proto("通信制御電文は64バイト");
        }
        Ok(CommCtl {
            kind: b[0],
            result: b[1],
            peer: CenterCode::from_bytes(&b[2..9])?,
            own: CenterCode::from_bytes(&b[9..16])?,
            time: b[16..22].try_into().unwrap(),
            password: b[22..28].try_into().unwrap(),
            app: byte_digit(b[28])?,
            mode: byte_digit(b[29])?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileCtl {
    pub kind: u8,
    pub result: u8,
    pub name: [u8; 12],
    pub key: [u8; 6],
    pub texts: u16,
    pub records: u32,
    pub rec_len: u16,
    pub from: u16,
    pub to: u16,
    pub compress: bool,
}

impl FileCtl {
    pub fn new(kind: u8, name: [u8; 12], key: [u8; 6], rec_len: u16, compress: bool) -> Self {
        FileCtl {
            kind,
            result: OK,
            name,
            key,
            texts: 0,
            records: 0,
            rec_len,
            from: 0,
            to: 0,
            compress,
        }
    }

    pub fn encode(&self) -> [u8; 64] {
        let mut b = [0u8; 64];
        b[0] = self.kind;
        b[1] = self.result;
        b[2..14].copy_from_slice(&self.name);
        b[14..20].copy_from_slice(&self.key);
        b[20..22].copy_from_slice(&self.texts.to_be_bytes());
        b[22..25].copy_from_slice(&self.records.to_be_bytes()[1..]);
        b[25] = digit_byte(0); // レコードID: 固定長
        b[26..28].copy_from_slice(&self.rec_len.to_be_bytes());
        b[28..30].copy_from_slice(&self.from.to_be_bytes());
        b[30..32].copy_from_slice(&self.to.to_be_bytes());
        b[32] = digit_byte(self.compress as u8);
        b
    }

    pub fn decode(b: &[u8]) -> Result<Self> {
        if b.len() != 64 {
            return proto("ファイル制御電文は64バイト");
        }
        if byte_digit(b[25])? != 0 {
            return proto("レコードID が固定長でない");
        }
        let compress = match byte_digit(b[32])? {
            0 => false,
            1 => true,
            _ => return proto("データ圧縮ID が不正"),
        };
        Ok(FileCtl {
            kind: b[0],
            result: b[1],
            name: b[2..14].try_into().unwrap(),
            key: b[14..20].try_into().unwrap(),
            texts: u16::from_be_bytes([b[20], b[21]]),
            records: u32::from_be_bytes([0, b[22], b[23], b[24]]),
            rec_len: u16::from_be_bytes([b[26], b[27]]),
            from: u16::from_be_bytes([b[28], b[29]]),
            to: u16::from_be_bytes([b[30], b[31]]),
            compress,
        })
    }
}

/// ファイル名 (12バイト、EBCDIC): 産業別コード(4) + データコード(4) + サイクル(2) + 補助情報ID(1)=0 + ダミー(1)=0。
pub fn make_name(industry: &str, data: &str, cycle: u8) -> [u8; 12] {
    let s = format!("{industry}{data}{cycle:02}00");
    assert_eq!(s.len(), 12);
    encode(Code::Ebcdic, &s).expect("数字のみ").try_into().unwrap()
}

/// 12バイトのファイル名を文字列にする。表にない文字は `?`。
pub fn name_str(n: &[u8; 12]) -> String {
    n.iter()
        .map(|&b| decode(Code::Ebcdic, &[b]).ok().and_then(|s| s.chars().next()).unwrap_or('?'))
        .collect()
}

/// ファイル名からサイクルを除いた識別部 (産業別コード+データコード、8文字)。
pub fn name_kind(n: &[u8; 12]) -> String {
    name_str(n).chars().take(8).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cc(s: &str) -> CenterCode {
        CenterCode::parse(s).unwrap()
    }

    #[test]
    fn center_code_bcd() {
        let c = cc("00312345670001");
        assert_eq!(c.to_bytes(), [0x00, 0x31, 0x23, 0x45, 0x67, 0x00, 0x01]);
        assert_eq!(CenterCode::from_bytes(&c.to_bytes()).unwrap(), c);
        assert_eq!(c.to_string(), "00312345670001");
        assert!(CenterCode::parse("123").is_err());
    }

    #[test]
    fn comm_ctl_layout() {
        let m = CommCtl {
            kind: OPEN_REQ,
            result: 0,
            peer: cc("00000000010001"),
            own: cc("00000000020002"),
            time: [0x26, 0x11, 0x21, 0x10, 0x30, 0x15],
            password: six("ABC123").unwrap(),
            app: 0,
            mode: 1,
        };
        let b = m.encode();
        assert_eq!(b.len(), 64);
        assert_eq!(&b[..2], [0, 0]);
        assert_eq!(&b[2..9], [0, 0, 0, 0, 0x01, 0x00, 0x01]);
        assert_eq!(&b[9..16], [0, 0, 0, 0, 0x02, 0x00, 0x02]);
        assert_eq!(&b[16..22], [0x26, 0x11, 0x21, 0x10, 0x30, 0x15]);
        assert_eq!(&b[22..28], [0xC1, 0xC2, 0xC3, 0xF1, 0xF2, 0xF3]);
        assert_eq!(&b[28..30], [0xF0, 0xF1]);
        assert!(b[30..].iter().all(|&x| x == 0));
        assert_eq!(CommCtl::decode(&b).unwrap(), m);
    }

    #[test]
    fn file_ctl_layout() {
        let mut m = FileCtl::new(END_REQ, make_name("5020", "0121", 3), six("KEY001").unwrap(), 120, true);
        m.texts = 0x0102;
        m.records = 0x030405;
        m.from = 1;
        m.to = 0xFFFF;
        let b = m.encode();
        assert_eq!(b[0], 0x12);
        assert_eq!(&b[2..14], [0xF5, 0xF0, 0xF2, 0xF0, 0xF0, 0xF1, 0xF2, 0xF1, 0xF0, 0xF3, 0xF0, 0xF0]);
        assert_eq!(&b[20..22], [1, 2]);
        assert_eq!(&b[22..25], [3, 4, 5]);
        assert_eq!(b[25], 0xF0);
        assert_eq!(&b[26..28], [0, 120]);
        assert_eq!(&b[28..32], [0, 1, 0xFF, 0xFF]);
        assert_eq!(b[32], 0xF1);
        assert!(b[33..].iter().all(|&x| x == 0)); // 補助情報(17B) + 拡張(14B)
        assert_eq!(FileCtl::decode(&b).unwrap(), m);
        assert_eq!(name_str(&m.name), "502001210300");
        assert_eq!(name_kind(&m.name), "50200121");
    }

    #[test]
    fn name_kind_never_panics_on_kana_bytes() {
        // EBCDIC の 0x81 などは半角カナ (UTF-8 で3バイト) になる。バイト位置で切ると panic するので、文字で切る
        let mut n = [0x81u8; 12];
        n[8..].copy_from_slice(&[0xF0; 4]);
        assert_eq!(name_kind(&n).chars().count(), 8);
        assert_eq!(name_kind(&[0xFF; 12]).chars().count(), 8); // 表にない文字は ?
    }

    #[test]
    fn rejects_bad_lengths_and_ids() {
        assert!(CommCtl::decode(&[0u8; 63]).is_err());
        let mut b = FileCtl::new(START_REQ, make_name("5020", "0001", 1), six("K").unwrap(), 200, false).encode();
        b[25] = 0xF1;
        assert!(FileCtl::decode(&b).is_err());
    }
}
