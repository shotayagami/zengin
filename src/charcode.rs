//! JIS / EBCDIC の使用文字表 (別冊 付録1)。表は `src/charcodes.txt`。

use crate::util::{Result, proto};
use std::collections::HashMap;
use std::sync::OnceLock;

/// ヘッダのコード区分。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Code {
    Jis,
    Ebcdic,
}

impl Code {
    pub fn digit(self) -> u8 {
        match self {
            Code::Jis => 0,
            Code::Ebcdic => 1,
        }
    }
}

struct Table {
    enc: HashMap<char, u8>,
    dec: [Option<char>; 256],
}

struct Tables {
    jis: Table,
    ebcdic: Table,
}

fn tables() -> &'static Tables {
    static T: OnceLock<Tables> = OnceLock::new();
    T.get_or_init(|| {
        let mut jis = Table {
            enc: HashMap::new(),
            dec: [None; 256],
        };
        let mut ebcdic = Table {
            enc: HashMap::new(),
            dec: [None; 256],
        };
        let rows: Vec<(&str, u8, char)> = include_str!("charcodes.txt")
            .lines()
            .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
            .map(|l| {
                let mut it = l.split(' ');
                let (name, hex, ch) = (it.next().unwrap(), it.next().unwrap(), it.next().unwrap());
                let b = u8::from_str_radix(hex, 16).expect("charcodes.txt の16進");
                (name, b, if ch == "SP" { ' ' } else { ch.chars().next().unwrap() })
            })
            .collect();
        // 表の原本は全角のカナ。内部では半角カナ (JIS X 0201 の 0xA1〜0xDF は U+FF61〜U+FF9F) で扱う。
        let to_half: HashMap<char, char> = rows
            .iter()
            .filter(|(n, b, _)| *n == "JIS" && *b >= 0xA1)
            .map(|&(_, b, c)| (c, char::from_u32(0xFF61 + (b - 0xA1) as u32).unwrap()))
            .collect();
        for (name, b, c) in rows {
            let c = *to_half.get(&c).unwrap_or(&c);
            let t = if name == "JIS" { &mut jis } else { &mut ebcdic };
            t.enc.insert(c, b);
            t.dec[b as usize] = Some(c);
        }
        Tables { jis, ebcdic }
    })
}

fn table(code: Code) -> &'static Table {
    match code {
        Code::Jis => &tables().jis,
        Code::Ebcdic => &tables().ebcdic,
    }
}

/// 円記号は全角表記に寄せる。
fn norm(c: char) -> char {
    match c {
        '\u{a5}' | '\\' => '\u{ffe5}',
        _ => c,
    }
}

pub fn encode(code: Code, s: &str) -> Result<Vec<u8>> {
    let t = table(code);
    s.chars()
        .map(|c| match t.enc.get(&norm(c)) {
            Some(&b) => Ok(b),
            None => proto(format!("使用できない文字: {c:?}")),
        })
        .collect()
}

pub fn decode(code: Code, b: &[u8]) -> Result<String> {
    let t = table(code);
    b.iter()
        .map(|&x| match t.dec[x as usize] {
            Some(c) => Ok(c),
            None => proto(format!("使用できないコード: 0x{x:02X}")),
        })
        .collect()
}

pub fn is_valid(code: Code, s: &str) -> bool {
    encode(code, s).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ebcdic_basics() {
        assert_eq!(encode(Code::Ebcdic, "A 0").unwrap(), [0xC1, 0x40, 0xF0]);
        assert_eq!(encode(Code::Ebcdic, "ｱ").unwrap(), [0x81]);
        assert_eq!(decode(Code::Ebcdic, &[0xF1, 0xF2]).unwrap(), "12");
    }

    #[test]
    fn jis_basics() {
        assert_eq!(encode(Code::Jis, "A 0").unwrap(), [0x41, 0x20, 0x30]);
        assert_eq!(encode(Code::Jis, "ｱﾝ").unwrap(), [0xB1, 0xDD]);
    }

    #[test]
    fn rejects_unlisted() {
        assert!(encode(Code::Jis, "a").is_err()); // 小文字
        assert!(encode(Code::Jis, "ｯ").is_err()); // 小文字カナ
        assert!(decode(Code::Ebcdic, &[0x00]).is_err());
    }

    #[test]
    fn yen_normalized() {
        assert_eq!(encode(Code::Jis, "\\").unwrap(), [0x5C]);
    }
}
