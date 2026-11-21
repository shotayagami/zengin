//! 固定長レコードと、ファイル (ヘッダ/データ/トレーラ/エンド) の組み立てと検証。
//! 業務のレイアウトは別冊 (総合振込、給与振込、振込入金通知)。為替通知 (91〜94) は docs/spec-net.md §3。

use crate::charcode::{Code, decode, encode};
use crate::util::{Result, proto};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum K {
    /// 数字。右詰め・前ゼロ
    N,
    /// 文字。左詰め・後ろスペース
    C,
}

#[derive(Debug)]
pub struct F {
    pub name: &'static str,
    pub k: K,
    pub len: usize,
}

const fn n(name: &'static str, len: usize) -> F {
    F { name, k: K::N, len }
}
const fn c(name: &'static str, len: usize) -> F {
    F { name, k: K::C, len }
}

#[derive(Debug)]
pub struct Layout {
    pub fields: &'static [F],
}

#[allow(clippy::len_without_is_empty)]
impl Layout {
    pub fn len(&self) -> usize {
        self.fields.iter().map(|f| f.len).sum()
    }
}

#[derive(Clone, Debug)]
pub struct Rec {
    pub layout: &'static Layout,
    vals: Vec<String>,
}

impl PartialEq for Rec {
    fn eq(&self, o: &Rec) -> bool {
        std::ptr::eq(self.layout, o.layout) && self.vals == o.vals
    }
}

impl Rec {
    pub fn new(layout: &'static Layout) -> Rec {
        Rec {
            layout,
            vals: layout
                .fields
                .iter()
                .map(|f| if f.k == K::N { "0".into() } else { String::new() })
                .collect(),
        }
    }

    fn idx(&self, name: &str) -> usize {
        self.layout
            .fields
            .iter()
            .position(|f| f.name == name)
            .unwrap_or_else(|| panic!("項目がない: {name}"))
    }

    pub fn has(&self, name: &str) -> bool {
        self.layout.fields.iter().any(|f| f.name == name)
    }

    pub fn get(&self, name: &str) -> &str {
        &self.vals[self.idx(name)]
    }

    pub fn num(&self, name: &str) -> u64 {
        self.get(name).parse().unwrap_or(0)
    }

    pub fn set(mut self, name: &str, v: impl Into<String>) -> Rec {
        let i = self.idx(name);
        self.vals[i] = v.into();
        self
    }

    pub fn set_num(self, name: &str, v: u64) -> Rec {
        self.set(name, v.to_string())
    }

    pub fn encode(&self, code: Code) -> Result<Vec<u8>> {
        let mut out = Vec::with_capacity(self.layout.len());
        for (f, v) in self.layout.fields.iter().zip(&self.vals) {
            if v.chars().count() > f.len {
                return proto(format!("{} が {} 桁を超える: {v:?}", f.name, f.len));
            }
            let pad = f.len - v.chars().count();
            let s = match f.k {
                K::N => {
                    if !v.bytes().all(|b| b.is_ascii_digit()) {
                        return proto(format!("{} が数字でない: {v:?}", f.name));
                    }
                    format!("{}{v}", "0".repeat(pad))
                }
                K::C => format!("{v}{}", " ".repeat(pad)),
            };
            out.extend(encode(code, &s)?);
        }
        Ok(out)
    }

    pub fn decode(layout: &'static Layout, b: &[u8], code: Code) -> Result<Rec> {
        if b.len() != layout.len() {
            return proto(format!("レコード長が不正: {} (期待 {})", b.len(), layout.len()));
        }
        let mut vals = Vec::new();
        let mut off = 0;
        for f in layout.fields {
            let s = decode(code, &b[off..off + f.len])?;
            off += f.len;
            vals.push(match f.k {
                K::N => {
                    if !s.bytes().all(|x| x.is_ascii_digit()) {
                        return proto(format!("{} が数字でない: {s:?}", f.name));
                    }
                    s
                }
                K::C => s.trim_end_matches(' ').to_string(),
            });
        }
        Ok(Rec { layout, vals })
    }
}

// ---- 別冊のレイアウト -------------------------------------------------------

/// 総合振込・給与振込のヘッダ (120)。会社コード/振込依頼人コード、会社名/振込依頼人名は同じ位置。
static H120: Layout = Layout {
    fields: &[
        n("data_kind", 1),
        n("type", 2),
        n("code", 1),
        n("client_code", 10),
        c("client_name", 40),
        n("date", 4),
        n("bank_no", 4),
        c("bank_name", 15),
        n("branch_no", 3),
        c("branch_name", 15),
        n("acct_kind", 1),
        n("acct_no", 7),
        c("dummy", 17),
    ],
};
static SOGO_D: Layout = Layout {
    fields: &[
        n("data_kind", 1),
        n("bank_no", 4),
        c("bank_name", 15),
        n("branch_no", 3),
        c("branch_name", 15),
        n("clearing", 4),
        n("acct_kind", 1),
        n("acct_no", 7),
        c("name", 30),
        n("amount", 10),
        n("new_code", 1),
        // 顧客コード1・2。識別表示が Y のときは EDI 情報 (20桁) として使うため、文字として扱う
        c("cust1", 10),
        c("cust2", 10),
        n("transfer_kind", 1),
        c("edi_flag", 1),
        c("dummy", 7),
    ],
};
static KYUYO_D: Layout = Layout {
    fields: &[
        n("data_kind", 1),
        n("bank_no", 4),
        c("bank_name", 15),
        n("branch_no", 3),
        c("branch_name", 15),
        n("clearing", 4),
        n("acct_kind", 1),
        n("acct_no", 7),
        c("name", 30),
        n("amount", 10),
        n("new_code", 1),
        n("emp_no", 10),
        n("dept", 10),
        c("dummy", 9),
    ],
};
static T120: Layout = Layout {
    fields: &[n("data_kind", 1), n("count", 6), n("amount", 12), c("dummy", 101)],
};
static E120: Layout = Layout {
    fields: &[n("data_kind", 1), c("dummy", 119)],
};

static NYUKIN_H: Layout = Layout {
    fields: &[
        n("data_kind", 1),
        n("type", 2),
        n("code", 1),
        n("created", 6),
        n("from", 6),
        n("to", 6),
        n("bank_no", 4),
        c("bank_name", 15),
        n("branch_no", 3),
        c("branch_name", 15),
        n("acct_kind", 1),
        n("acct_no", 7),
        c("acct_name", 40),
        c("dummy", 93),
    ],
};
static NYUKIN_D: Layout = Layout {
    fields: &[
        n("data_kind", 1),
        n("inquiry_no", 6),
        n("acct_date", 6),
        n("start_date", 6),
        n("amount", 10),
        n("other_amount", 10),
        n("client_code", 10),
        c("client_name", 48),
        c("src_bank", 15),
        c("src_branch", 15),
        n("cancel", 1),
        c("edi", 20),
        c("dummy", 52),
    ],
};
static NYUKIN_T: Layout = Layout {
    fields: &[
        n("data_kind", 1),
        n("count", 6),
        n("amount", 12),
        n("cancel_count", 6),
        n("cancel_amount", 12),
        c("dummy", 163),
    ],
};
static E200: Layout = Layout {
    fields: &[n("data_kind", 1), c("dummy", 199)],
};

// ---- 為替通知 (独自設計。docs/spec-net.md §3) --------------------------------------

static EX_H: Layout = Layout {
    fields: &[
        n("data_kind", 1),
        n("type", 2),
        n("code", 1),
        n("from", 4),
        n("to", 4),
        n("date", 8),
        n("time", 6),
        c("dummy", 174),
    ],
};
static EX_T: Layout = Layout {
    fields: &[n("data_kind", 1), n("count", 6), n("amount", 12), c("dummy", 181)],
};
static EX91_D: Layout = Layout {
    fields: &[
        n("data_kind", 1),
        n("handling_no", 10),
        n("src_bank", 4),
        n("src_branch", 3),
        n("dst_bank", 4),
        n("dst_branch", 3),
        n("acct_kind", 1),
        n("acct_no", 7),
        c("name", 30),
        n("amount", 10),
        n("client_code", 10),
        c("client_name", 48),
        c("edi", 20),
        c("dummy", 49),
    ],
};
static EX92_D: Layout = Layout {
    fields: &[
        n("data_kind", 1),
        n("handling_no", 10),
        n("src_bank", 4),
        n("dst_bank", 4),
        n("reason", 2),
        n("amount", 10),
        c("dummy", 169),
    ],
};
static EX93_D: Layout = Layout {
    fields: &[
        n("data_kind", 1),
        n("handling_no", 10),
        n("src_bank", 4),
        n("dst_bank", 4),
        n("req_amount", 10),
        c("dummy", 171),
    ],
};
static EX94_D: Layout = Layout {
    fields: &[
        n("data_kind", 1),
        n("handling_no", 10),
        n("src_bank", 4),
        n("dst_bank", 4),
        n("reason", 2),
        c("dummy", 179),
    ],
};

// ---- ファイル -------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileKind {
    /// 総合振込 (種別21、連絡)
    Sogo,
    /// 給与振込 (種別11、連絡)
    Kyuyo,
    /// 振込入金通知 (種別01、照会)
    Nyukin,
    /// 為替通知: 91 振込、92 返却、93 組戻依頼、94 組戻不能 (連絡)
    Ex91,
    Ex92,
    Ex93,
    Ex94,
}

pub struct Fmt {
    pub kind: FileKind,
    pub rec_len: usize,
    /// ヘッダの種別コード
    pub type_code: u64,
    /// ファイル名の産業別コード + データコード (8文字)
    pub name_kind: &'static str,
    header: &'static Layout,
    data: &'static Layout,
    trailer: &'static Layout,
    end: &'static Layout,
}

static FMTS: [Fmt; 7] = [
    Fmt {
        kind: FileKind::Sogo,
        rec_len: 120,
        type_code: 21,
        name_kind: "50200121",
        header: &H120,
        data: &SOGO_D,
        trailer: &T120,
        end: &E120,
    },
    Fmt {
        kind: FileKind::Kyuyo,
        rec_len: 120,
        type_code: 11,
        name_kind: "50200111",
        header: &H120,
        data: &KYUYO_D,
        trailer: &T120,
        end: &E120,
    },
    Fmt {
        kind: FileKind::Nyukin,
        rec_len: 200,
        type_code: 1,
        name_kind: "50200001",
        header: &NYUKIN_H,
        data: &NYUKIN_D,
        trailer: &NYUKIN_T,
        end: &E200,
    },
    Fmt {
        kind: FileKind::Ex91,
        rec_len: 200,
        type_code: 91,
        name_kind: "90209091",
        header: &EX_H,
        data: &EX91_D,
        trailer: &EX_T,
        end: &E200,
    },
    Fmt {
        kind: FileKind::Ex92,
        rec_len: 200,
        type_code: 92,
        name_kind: "90209092",
        header: &EX_H,
        data: &EX92_D,
        trailer: &EX_T,
        end: &E200,
    },
    Fmt {
        kind: FileKind::Ex93,
        rec_len: 200,
        type_code: 93,
        name_kind: "90209093",
        header: &EX_H,
        data: &EX93_D,
        trailer: &EX_T,
        end: &E200,
    },
    Fmt {
        kind: FileKind::Ex94,
        rec_len: 200,
        type_code: 94,
        name_kind: "90209094",
        header: &EX_H,
        data: &EX94_D,
        trailer: &EX_T,
        end: &E200,
    },
];

pub fn fmt(kind: FileKind) -> &'static Fmt {
    FMTS.iter().find(|f| f.kind == kind).unwrap()
}

/// ファイル名の識別部 (8文字) から業務を引く。
pub fn kind_from_name(name_kind: &str) -> Option<&'static Fmt> {
    FMTS.iter().find(|f| f.name_kind == name_kind)
}

/// データレコードの (件数, 金額, 取消の件数, 取消の金額)。`cancel` 項目が 1 の行は取消として別に数える。
fn tally(data: &[Rec]) -> (u64, u64, u64, u64) {
    let (mut c, mut a, mut cc, mut ca) = (0, 0, 0, 0);
    for d in data {
        let amt = if d.has("amount") { d.num("amount") } else { 0 };
        if d.has("cancel") && d.num("cancel") == 1 {
            cc += 1;
            ca += amt;
        } else {
            c += 1;
            a += amt;
        }
    }
    (c, a, cc, ca)
}

/// 人名・店舗名の欄に使える文字か (別冊 付録1 の注1): カナ(ヲと小文字を除く)、濁点、半濁点、英大文字、数字、スペース、`( ) - .`。
pub fn name_chars_ok(s: &str) -> bool {
    s.chars().all(|c| {
        matches!(c, 'A'..='Z' | '0'..='9' | ' ' | '(' | ')' | '-' | '.' | '\u{ff71}'..='\u{ff9f}')
            && !('\u{ff67}'..='\u{ff6f}').contains(&c)
    })
}

pub struct Parsed {
    pub code: Code,
    pub header: Rec,
    pub data: Vec<Rec>,
    pub trailer: Rec,
}

impl Fmt {
    pub fn new_header(&self) -> Rec {
        Rec::new(self.header)
    }
    pub fn new_data(&self) -> Rec {
        Rec::new(self.data)
    }

    /// ヘッダとデータからファイルを作る。データ区分、種別、コード区分、トレーラの件数・金額、エンドは自動で設定する。
    pub fn build(&self, code: Code, header: Rec, data: &[Rec]) -> Result<Vec<u8>> {
        let mut out = header
            .set_num("data_kind", 1)
            .set_num("type", self.type_code)
            .set_num("code", code.digit() as u64)
            .encode(code)?;
        for d in data {
            if !std::ptr::eq(d.layout, self.data) {
                return proto("データレコードの種類が違う");
            }
            out.extend(d.clone().set_num("data_kind", 2).encode(code)?);
        }
        let (count, amount, cancel_count, cancel_amount) = tally(data);
        let mut t = Rec::new(self.trailer)
            .set_num("data_kind", 8)
            .set_num("count", count)
            .set_num("amount", amount);
        if t.has("cancel_count") {
            t = t.set_num("cancel_count", cancel_count).set_num("cancel_amount", cancel_amount);
        }
        out.extend(t.encode(code)?);
        out.extend(Rec::new(self.end).set_num("data_kind", 9).encode(code)?);
        Ok(out)
    }

    /// ファイルを解析し、順序、種別、件数、合計金額を検査する。
    pub fn parse(&self, b: &[u8]) -> Result<Parsed> {
        if b.is_empty() || b.len() % self.rec_len != 0 || b.len() / self.rec_len < 3 {
            return proto(format!("ファイル長が不正: {} バイト (レコード長 {})", b.len(), self.rec_len));
        }
        let code = match b[0] {
            0x31 => Code::Jis,
            0xF1 => Code::Ebcdic,
            x => return proto(format!("先頭のデータ区分が不正: 0x{x:02X}")),
        };
        let recs: Vec<&[u8]> = b.chunks(self.rec_len).collect();
        let header = Rec::decode(self.header, recs[0], code)?;
        if header.num("data_kind") != 1 || header.num("type") != self.type_code || header.num("code") != code.digit() as u64 {
            return proto("ヘッダの種別/コード区分が不正");
        }
        let last = recs.len() - 1;
        let end = Rec::decode(self.end, recs[last], code)?;
        if end.num("data_kind") != 9 {
            return proto("エンド・レコードがない");
        }
        let trailer = Rec::decode(self.trailer, recs[last - 1], code)?;
        if trailer.num("data_kind") != 8 {
            return proto("トレーラ・レコードがない");
        }
        let mut data = Vec::new();
        for r in &recs[1..last - 1] {
            let d = Rec::decode(self.data, r, code)?;
            if d.num("data_kind") != 2 {
                return proto("データ・レコードのデータ区分が不正");
            }
            data.push(d);
        }
        let (count, amount, cancel_count, cancel_amount) = tally(&data);
        if trailer.num("count") != count {
            return proto(format!("トレーラの合計件数が不一致: {} (実際 {count})", trailer.num("count")));
        }
        if trailer.num("amount") != amount {
            return proto(format!("トレーラの合計金額が不一致: {} (実際 {amount})", trailer.num("amount")));
        }
        if trailer.has("cancel_count") && (trailer.num("cancel_count") != cancel_count || trailer.num("cancel_amount") != cancel_amount) {
            return proto("トレーラの取消の件数/金額が不一致");
        }
        Ok(Parsed {
            code,
            header,
            data,
            trailer,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layouts_have_declared_lengths() {
        for f in &FMTS {
            for l in [f.header, f.data, f.trailer, f.end] {
                assert_eq!(l.len(), f.rec_len, "{:?}", f.kind);
            }
        }
    }

    fn sogo(code: Code) -> Vec<u8> {
        let f = fmt(FileKind::Sogo);
        let h = f
            .new_header()
            .set("client_code", "1234567890")
            .set("client_name", "ｶ)ﾃｽﾄ")
            .set("date", "1121")
            .set_num("bank_no", 1);
        let d = |amt, no| {
            f.new_data()
                .set_num("bank_no", 2)
                .set_num("branch_no", 3)
                .set("acct_kind", "1")
                .set("acct_no", format!("{no:07}"))
                .set("name", "ﾔﾏﾀﾞ ﾀﾛｳ")
                .set_num("amount", amt)
        };
        f.build(code, h, &[d(1000, 1), d(250, 2)]).unwrap()
    }

    #[test]
    fn sogo_roundtrip_both_codes() {
        for code in [Code::Jis, Code::Ebcdic] {
            let b = sogo(code);
            assert_eq!(b.len(), 120 * 5);
            let p = fmt(FileKind::Sogo).parse(&b).unwrap();
            assert_eq!(p.code, code);
            assert_eq!(p.header.get("client_name"), "ｶ)ﾃｽﾄ");
            assert_eq!(p.data.len(), 2);
            assert_eq!(p.data[1].num("amount"), 250);
            assert_eq!(p.trailer.num("count"), 2);
            assert_eq!(p.trailer.num("amount"), 1250);
        }
    }

    #[test]
    fn known_bytes_jis() {
        let b = sogo(Code::Jis);
        // ヘッダ: データ区分 '1'、種別 '21'、コード区分 '0'
        assert_eq!(&b[..4], b"1210");
        // トレーラ (4番目のレコード): '8' + 件数6桁 + 金額12桁
        assert_eq!(&b[120 * 3..120 * 3 + 19], b"8000002000000001250");
        // エンド: '9' + スペース
        assert_eq!(b[120 * 4], b'9');
        assert!(b[120 * 4 + 1..].iter().all(|&x| x == 0x20));
    }

    #[test]
    fn rejects_trailer_mismatch_and_bad_order() {
        let mut b = sogo(Code::Jis);
        b[120 * 3 + 18] = b'9'; // 合計金額を変える
        assert!(fmt(FileKind::Sogo).parse(&b).is_err());
        let mut b = sogo(Code::Jis);
        b[120 * 3 + 6] = b'3'; // 合計件数を変える
        assert!(fmt(FileKind::Sogo).parse(&b).is_err());
        let b = sogo(Code::Jis);
        assert!(fmt(FileKind::Sogo).parse(&b[..120 * 4]).is_err()); // エンドなし
        assert!(fmt(FileKind::Kyuyo).parse(&sogo(Code::Jis)).is_err()); // 種別違い
    }

    #[test]
    fn rejects_bad_fields() {
        let f = fmt(FileKind::Sogo);
        let h = f.new_header().set("client_name", "a"); // 小文字は不可
        assert!(f.build(Code::Jis, h, &[]).is_err());
        let h = f.new_header().set("client_code", "12345678901"); // 桁あふれ
        assert!(f.build(Code::Jis, h, &[]).is_err());
        let h = f.new_header().set("date", "12x4");
        assert!(f.build(Code::Jis, h, &[]).is_err());
    }

    #[test]
    fn nyukin_and_exchange_roundtrip() {
        let f = fmt(FileKind::Nyukin);
        let h = f.new_header().set_num("bank_no", 2).set("acct_name", "ﾃｽﾄ");
        let d = f.new_data().set_num("amount", 777).set("client_name", "ﾌﾘｺﾐ");
        let b = f.build(Code::Ebcdic, h, &[d]).unwrap();
        assert_eq!(b.len(), 800);
        let p = f.parse(&b).unwrap();
        assert_eq!(p.data[0].num("amount"), 777);

        let f = fmt(FileKind::Ex94);
        let d = f.new_data().set_num("handling_no", 1121000001).set_num("reason", 1);
        let b = f.build(Code::Jis, f.new_header().set_num("from", 2), &[d]).unwrap();
        let p = f.parse(&b).unwrap();
        assert_eq!(p.data[0].num("handling_no"), 1121000001);
        assert_eq!(p.trailer.num("amount"), 0);
    }

    #[test]
    fn kind_lookup() {
        assert_eq!(kind_from_name("50200121").unwrap().kind, FileKind::Sogo);
        assert_eq!(kind_from_name("90209092").unwrap().kind, FileKind::Ex92);
        assert!(kind_from_name("12345678").is_none());
    }
}
