use std::fmt;
use std::io;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

/// プロトコルと業務処理で共通のエラー。
#[derive(Debug)]
pub enum ZError {
    Io(io::Error),
    /// 規定外の電文、検査エラーなど。TCP を切断する。
    Protocol(String),
    /// 相手が処理結果 NG (コード) を返した。
    Rejected(u8, String),
}

pub type Result<T> = std::result::Result<T, ZError>;

impl fmt::Display for ZError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ZError::Io(e) => write!(f, "I/O エラー: {e}"),
            ZError::Protocol(s) => write!(f, "プロトコルエラー: {s}"),
            ZError::Rejected(c, s) => write!(f, "相手が拒否 (処理結果 {c:02X}): {s}"),
        }
    }
}

impl std::error::Error for ZError {}

impl From<io::Error> for ZError {
    fn from(e: io::Error) -> Self {
        ZError::Io(e)
    }
}

/// 別スレッドが panic して Mutex が poisoned になっても、状態をそのまま使い続ける。
pub fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// 設定の数値を、範囲を確かめて型に入れる。
pub fn small<T: TryFrom<u64>>(v: u64, what: &str) -> Result<T> {
    T::try_from(v).or_else(|_| proto(format!("{what} が範囲外: {v}")))
}

pub fn proto<T>(msg: impl Into<String>) -> Result<T> {
    Err(ZError::Protocol(msg.into()))
}

/// 西暦の年月日と時分秒 (JST)。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DateTime {
    pub y: u16,
    pub mo: u8,
    pub d: u8,
    pub h: u8,
    pub mi: u8,
    pub s: u8,
}

impl DateTime {
    pub fn ymd(&self) -> String {
        format!("{:04}{:02}{:02}", self.y, self.mo, self.d)
    }
    pub fn hms(&self) -> String {
        format!("{:02}{:02}{:02}", self.h, self.mi, self.s)
    }
    pub fn mmdd(&self) -> String {
        format!("{:02}{:02}", self.mo, self.d)
    }
    /// 和暦 (令和) の YYMMDD。2019-05-01 以降のみ対応。
    pub fn reiwa_yymmdd(&self) -> String {
        format!("{:02}{:02}{:02}", self.y.saturating_sub(2018), self.mo, self.d)
    }
}

/// 日時の取得元。試験では固定できる。
#[derive(Clone, Default)]
pub struct Clock(Arc<Mutex<Option<DateTime>>>);

impl Clock {
    pub fn fixed(dt: DateTime) -> Self {
        Clock(Arc::new(Mutex::new(Some(dt))))
    }
    pub fn set(&self, dt: DateTime) {
        *lock(&self.0) = Some(dt);
    }
    pub fn now(&self) -> DateTime {
        if let Some(dt) = *lock(&self.0) {
            return dt;
        }
        let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) as i64 + 9 * 3600;
        let days = secs.div_euclid(86400);
        let rem = secs.rem_euclid(86400);
        let (y, mo, d) = civil_from_days(days);
        DateTime {
            y: y as u16,
            mo,
            d,
            h: (rem / 3600) as u8,
            mi: (rem % 3600 / 60) as u8,
            s: (rem % 60) as u8,
        }
    }
}

/// 西暦 YYYYMMDD を、1970-01-01 からの日数にする (Howard Hinnant の days_from_civil)。
pub fn days_from_ymd(ymd: &str) -> Option<i64> {
    let y: i64 = ymd.get(..4)?.parse().ok()?;
    let m: i64 = ymd.get(4..6)?.parse().ok()?;
    let d: i64 = ymd.get(6..8)?.parse().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146097 + doe - 719468)
}

// Howard Hinnant の civil_from_days。
fn civil_from_days(z: i64) -> (i64, u8, u8) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u8;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u8;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// 10進の数字列を BCD に詰める (2桁/バイト)。桁数は偶数。
pub fn bcd_encode(digits: &str) -> Option<Vec<u8>> {
    if digits.len() % 2 != 0 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some(digits.as_bytes().chunks(2).map(|p| ((p[0] - b'0') << 4) | (p[1] - b'0')).collect())
}

pub fn bcd_decode(b: &[u8]) -> Option<String> {
    let mut s = String::new();
    for &x in b {
        if x >> 4 > 9 || x & 0xF > 9 {
            return None;
        }
        s.push((b'0' + (x >> 4)) as char);
        s.push((b'0' + (x & 0xF)) as char);
    }
    Some(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bcd_roundtrip() {
        let b = bcd_encode("261121103015").unwrap();
        assert_eq!(b, [0x26, 0x11, 0x21, 0x10, 0x30, 0x15]);
        assert_eq!(bcd_decode(&b).unwrap(), "261121103015");
        assert!(bcd_encode("123").is_none());
        assert!(bcd_decode(&[0x1A]).is_none());
    }

    #[test]
    fn days_from_ymd_is_the_inverse() {
        for z in [-1, 0, 1, 19_000, 20_000, 20_778, 20_779, 20_819] {
            let (y, m, d) = civil_from_days(z);
            assert_eq!(days_from_ymd(&format!("{y:04}{m:02}{d:02}")), Some(z));
        }
        assert_eq!(days_from_ymd("20261122").unwrap() - days_from_ymd("20261121").unwrap(), 1);
        assert_eq!(days_from_ymd("2026x121"), None);
    }

    #[test]
    fn civil_dates() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(20453), (2025, 12, 31));
        assert_eq!(civil_from_days(19811), (2024, 3, 29));
    }
}
