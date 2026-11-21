//! 為替通知 (docs/spec-net.md §3): 91 振込、92 返却、93 組戻依頼、94 組戻不能。

use crate::charcode::Code;
use crate::record::{FileKind, Rec, fmt};
use crate::util::{DateTime, Result, proto};

/// 1億円以上は取引ごとに即時決済 (RTGS)、未満は差額決済 (netting)。
pub const RTGS_THRESHOLD: u64 = 100_000_000;
/// 全銀ネットの銀行番号 (為替通知ファイルの送信元/送信先)
pub const NET_BANK_NO: u16 = 0;

// 返却事由
pub const REASON_NO_ACCOUNT: u8 = 1;
pub const REASON_RECALL: u8 = 3;
pub const REASON_UNKNOWN_BANK: u8 = 10;
pub const REASON_LIMIT: u8 = 11;
pub const REASON_BAD_MESSAGE: u8 = 12;
// 組戻不能の事由
pub const RECALL_NO_FUNDS: u8 = 1;
pub const RECALL_NOT_FOUND: u8 = 2;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transfer {
    pub handling_no: u64,
    pub src_bank: u16,
    pub src_branch: u16,
    pub dst_bank: u16,
    pub dst_branch: u16,
    pub acct_kind: u8,
    pub acct_no: String,
    pub name: String,
    pub amount: u64,
    pub client_code: u64,
    pub client_name: String,
    pub edi: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Return {
    pub handling_no: u64,
    /// 元の振込の仕向銀行 (返却を受け取る)
    pub src_bank: u16,
    /// 元の振込の被仕向銀行
    pub dst_bank: u16,
    pub reason: u8,
    pub amount: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recall {
    pub handling_no: u64,
    pub src_bank: u16,
    pub dst_bank: u16,
    pub amount: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecallNg {
    pub handling_no: u64,
    pub src_bank: u16,
    pub dst_bank: u16,
    pub reason: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Item {
    Transfer(Transfer),
    Return(Return),
    Recall(Recall),
    RecallNg(RecallNg),
}

impl Item {
    pub fn file_kind(&self) -> FileKind {
        match self {
            Item::Transfer(_) => FileKind::Ex91,
            Item::Return(_) => FileKind::Ex92,
            Item::Recall(_) => FileKind::Ex93,
            Item::RecallNg(_) => FileKind::Ex94,
        }
    }

    /// 種別コード 91〜94
    pub fn type_code(&self) -> u8 {
        fmt(self.file_kind()).type_code as u8
    }

    pub fn handling_no(&self) -> u64 {
        match self {
            Item::Transfer(x) => x.handling_no,
            Item::Return(x) => x.handling_no,
            Item::Recall(x) => x.handling_no,
            Item::RecallNg(x) => x.handling_no,
        }
    }

    pub fn src_bank(&self) -> u16 {
        match self {
            Item::Transfer(x) => x.src_bank,
            Item::Return(x) => x.src_bank,
            Item::Recall(x) => x.src_bank,
            Item::RecallNg(x) => x.src_bank,
        }
    }

    pub fn dst_bank(&self) -> u16 {
        match self {
            Item::Transfer(x) => x.dst_bank,
            Item::Return(x) => x.dst_bank,
            Item::Recall(x) => x.dst_bank,
            Item::RecallNg(x) => x.dst_bank,
        }
    }

    pub fn amount(&self) -> u64 {
        match self {
            Item::Transfer(x) => x.amount,
            Item::Return(x) => x.amount,
            Item::Recall(x) => x.amount,
            Item::RecallNg(_) => 0,
        }
    }

    /// この為替通知を作って送り出す銀行。
    pub fn origin(&self) -> u16 {
        match self {
            Item::Transfer(x) => x.src_bank,
            Item::Return(x) => x.dst_bank,
            Item::Recall(x) => x.src_bank,
            Item::RecallNg(x) => x.dst_bank,
        }
    }

    /// この為替通知の届け先の銀行。
    pub fn destination(&self) -> u16 {
        match self {
            Item::Transfer(x) => x.dst_bank,
            Item::Return(x) => x.src_bank,
            Item::Recall(x) => x.dst_bank,
            Item::RecallNg(x) => x.src_bank,
        }
    }

    /// 重複判定のキー: (仕向銀行, 取扱番号, 種別)
    pub fn key(&self) -> (u16, u64, u8) {
        (self.src_bank(), self.handling_no(), self.type_code())
    }

    pub fn to_rec(&self) -> Rec {
        let f = fmt(self.file_kind());
        let r = f
            .new_data()
            .set_num("handling_no", self.handling_no())
            .set_num("src_bank", self.src_bank() as u64)
            .set_num("dst_bank", self.dst_bank() as u64);
        match self {
            Item::Transfer(x) => r
                .set_num("src_branch", x.src_branch as u64)
                .set_num("dst_branch", x.dst_branch as u64)
                .set_num("acct_kind", x.acct_kind as u64)
                .set("acct_no", x.acct_no.clone())
                .set("name", x.name.clone())
                .set_num("amount", x.amount)
                .set_num("client_code", x.client_code)
                .set("client_name", x.client_name.clone())
                .set("edi", x.edi.clone()),
            Item::Return(x) => r.set_num("reason", x.reason as u64).set_num("amount", x.amount),
            Item::Recall(x) => r.set_num("req_amount", x.amount),
            Item::RecallNg(x) => r.set_num("reason", x.reason as u64),
        }
    }

    pub fn from_rec(kind: FileKind, r: &Rec) -> Result<Item> {
        let (h, s, d) = (r.num("handling_no"), r.num("src_bank") as u16, r.num("dst_bank") as u16);
        Ok(match kind {
            FileKind::Ex91 => Item::Transfer(Transfer {
                handling_no: h,
                src_bank: s,
                src_branch: r.num("src_branch") as u16,
                dst_bank: d,
                dst_branch: r.num("dst_branch") as u16,
                acct_kind: r.num("acct_kind") as u8,
                acct_no: r.get("acct_no").to_string(),
                name: r.get("name").to_string(),
                amount: r.num("amount"),
                client_code: r.num("client_code"),
                client_name: r.get("client_name").to_string(),
                edi: r.get("edi").to_string(),
            }),
            FileKind::Ex92 => Item::Return(Return {
                handling_no: h,
                src_bank: s,
                dst_bank: d,
                reason: r.num("reason") as u8,
                amount: r.num("amount"),
            }),
            FileKind::Ex93 => Item::Recall(Recall {
                handling_no: h,
                src_bank: s,
                dst_bank: d,
                amount: r.num("req_amount"),
            }),
            FileKind::Ex94 => Item::RecallNg(RecallNg {
                handling_no: h,
                src_bank: s,
                dst_bank: d,
                reason: r.num("reason") as u8,
            }),
            _ => return proto("為替通知のファイル種別でない"),
        })
    }

    /// 状態ファイル用の1行 (タブ区切り)。
    pub fn to_line(&self) -> String {
        let t = self.type_code();
        match self {
            Item::Transfer(x) => format!(
                "{t}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                x.handling_no,
                x.src_bank,
                x.src_branch,
                x.dst_bank,
                x.dst_branch,
                x.acct_kind,
                x.acct_no,
                x.name,
                x.amount,
                x.client_code,
                x.client_name,
                x.edi
            ),
            Item::Return(x) => format!("{t}\t{}\t{}\t{}\t{}\t{}", x.handling_no, x.src_bank, x.dst_bank, x.reason, x.amount),
            Item::Recall(x) => format!("{t}\t{}\t{}\t{}\t{}", x.handling_no, x.src_bank, x.dst_bank, x.amount),
            Item::RecallNg(x) => format!("{t}\t{}\t{}\t{}\t{}", x.handling_no, x.src_bank, x.dst_bank, x.reason),
        }
    }

    pub fn from_line(line: &str) -> Result<Item> {
        let f: Vec<&str> = line.split('\t').collect();
        fn bad<T>(line: &str) -> Result<T> {
            proto(format!("状態ファイルの行が不正: {line:?}"))
        }
        let n = |i: usize| -> Result<u64> { f.get(i).and_then(|s| s.parse().ok()).map_or_else(|| bad(line), Ok) };
        let s = |i: usize| f.get(i).copied().unwrap_or("").to_string();
        Ok(match f.first().copied() {
            Some("91") if f.len() >= 12 => Item::Transfer(Transfer {
                handling_no: n(1)?,
                src_bank: n(2)? as u16,
                src_branch: n(3)? as u16,
                dst_bank: n(4)? as u16,
                dst_branch: n(5)? as u16,
                acct_kind: n(6)? as u8,
                acct_no: s(7),
                name: s(8),
                amount: n(9)?,
                client_code: n(10)?,
                client_name: s(11),
                edi: s(12),
            }),
            Some("92") if f.len() == 6 => Item::Return(Return {
                handling_no: n(1)?,
                src_bank: n(2)? as u16,
                dst_bank: n(3)? as u16,
                reason: n(4)? as u8,
                amount: n(5)?,
            }),
            Some("93") if f.len() == 5 => Item::Recall(Recall {
                handling_no: n(1)?,
                src_bank: n(2)? as u16,
                dst_bank: n(3)? as u16,
                amount: n(4)?,
            }),
            Some("94") if f.len() == 5 => Item::RecallNg(RecallNg {
                handling_no: n(1)?,
                src_bank: n(2)? as u16,
                dst_bank: n(3)? as u16,
                reason: n(4)? as u8,
            }),
            _ => return bad(line),
        })
    }
}

/// 同じ種別の為替通知から、ファイルを作る。
pub fn build_file(kind: FileKind, from: u16, to: u16, items: &[Item], now: &DateTime, code: Code) -> Result<Vec<u8>> {
    if items.iter().any(|i| i.file_kind() != kind) {
        return proto("種別の違う為替通知が混ざっている");
    }
    let f = fmt(kind);
    let h = f
        .new_header()
        .set_num("from", from as u64)
        .set_num("to", to as u64)
        .set("date", now.ymd())
        .set("time", now.hms());
    let recs: Vec<Rec> = items.iter().map(|i| i.to_rec()).collect();
    f.build(code, h, &recs)
}

/// 1ファイルに入れる為替通知の件数と合計金額の上限。受信側の大きさの上限と、トレーラの桁 (N12) に収めるため。
pub const MAX_ITEMS_PER_FILE: usize = 5000;
pub const MAX_AMOUNT_PER_FILE: u64 = 999_999_999_999;

/// 同じ種別の為替通知を、ファイルに収まる大きさに分けて作る。
/// 戻り値は (ファイルの中身と、そこに入れた為替通知) の列と、作れなかった為替通知 (理由つき)。
/// 作れない為替通知があっても、残りは送れるようにするため、1件ずつ先に検査する。
#[allow(clippy::type_complexity)]
pub fn build_files(
    kind: FileKind,
    from: u16,
    to: u16,
    items: &[Item],
    now: &DateTime,
    code: Code,
) -> (Vec<(Vec<u8>, Vec<Item>)>, Vec<(Item, String)>) {
    let mut files = Vec::new();
    let mut bad = Vec::new();
    let mut cur: Vec<Item> = Vec::new();
    let mut sum = 0u64;
    for it in items {
        if let Err(e) = build_file(kind, from, to, std::slice::from_ref(it), now, code) {
            bad.push((it.clone(), e.to_string()));
            continue;
        }
        if !cur.is_empty() && (cur.len() >= MAX_ITEMS_PER_FILE || sum + it.amount() > MAX_AMOUNT_PER_FILE) {
            let data = build_file(kind, from, to, &cur, now, code).expect("1件ずつ検査済み");
            files.push((data, std::mem::take(&mut cur)));
            sum = 0;
        }
        sum += it.amount();
        cur.push(it.clone());
    }
    if !cur.is_empty() {
        let data = build_file(kind, from, to, &cur, now, code).expect("1件ずつ検査済み");
        files.push((data, cur));
    }
    (files, bad)
}

/// ファイルを解析して (送信元, 送信先, 為替通知) を返す。
pub fn parse_file(kind: FileKind, bytes: &[u8]) -> Result<(u16, u16, Vec<Item>)> {
    let p = fmt(kind).parse(bytes)?;
    let items = p.data.iter().map(|r| Item::from_rec(kind, r)).collect::<Result<Vec<_>>>()?;
    Ok((p.header.num("from") as u16, p.header.num("to") as u16, items))
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn sample() -> Vec<Item> {
        vec![
            Item::Transfer(Transfer {
                handling_no: 1121000001,
                src_bank: 1,
                src_branch: 11,
                dst_bank: 2,
                dst_branch: 22,
                acct_kind: 1,
                acct_no: "0000042".into(),
                name: "ﾔﾏﾀﾞ ﾀﾛｳ".into(),
                amount: 12345,
                client_code: 77,
                client_name: "ｶ)ﾃｽﾄ".into(),
                edi: "ﾁﾕｳﾓﾝ 1".into(),
            }),
            Item::Return(Return {
                handling_no: 1121000002,
                src_bank: 1,
                dst_bank: 2,
                reason: REASON_NO_ACCOUNT,
                amount: 500,
            }),
            Item::Recall(Recall {
                handling_no: 1121000003,
                src_bank: 1,
                dst_bank: 2,
                amount: 900,
            }),
            Item::RecallNg(RecallNg {
                handling_no: 1121000004,
                src_bank: 1,
                dst_bank: 2,
                reason: RECALL_NO_FUNDS,
            }),
        ]
    }

    fn now() -> DateTime {
        DateTime {
            y: 2026,
            mo: 11,
            d: 21,
            h: 10,
            mi: 30,
            s: 15,
        }
    }

    #[test]
    fn files_roundtrip_for_every_kind() {
        for item in sample() {
            for code in [Code::Jis, Code::Ebcdic] {
                let b = build_file(item.file_kind(), 1, 0, &[item.clone(), item.clone()], &now(), code).unwrap();
                assert_eq!(b.len(), 200 * 5);
                let (from, to, items) = parse_file(item.file_kind(), &b).unwrap();
                assert_eq!((from, to), (1, 0));
                assert_eq!(items, vec![item.clone(), item.clone()]);
            }
        }
    }

    #[test]
    fn lines_roundtrip() {
        for item in sample() {
            assert_eq!(Item::from_line(&item.to_line()).unwrap(), item);
        }
        assert!(Item::from_line("99\t1").is_err());
        assert!(Item::from_line("92\tx\t1\t2\t3\t4").is_err());
    }

    #[test]
    fn routing_helpers() {
        let s = sample();
        assert_eq!((s[0].origin(), s[0].destination()), (1, 2));
        assert_eq!((s[1].origin(), s[1].destination()), (2, 1)); // 返却は被仕向 → 仕向
        assert_eq!((s[2].origin(), s[2].destination()), (1, 2));
        assert_eq!((s[3].origin(), s[3].destination()), (2, 1));
        assert_eq!(s[1].key(), (1, 1121000002, 92));
    }

    #[test]
    fn build_files_splits_by_count_and_amount_and_isolates_bad_items() {
        let mk = |i: u64, amount: u64| {
            let Item::Transfer(mut t) = sample().remove(0) else {
                unreachable!()
            };
            t.handling_no = i;
            t.amount = amount;
            Item::Transfer(t)
        };
        // 件数: 5000 件ごとに分かれる
        let items: Vec<Item> = (0..12_000).map(|i| mk(i, 1)).collect();
        let (files, bad) = build_files(FileKind::Ex91, 1, 0, &items, &now(), Code::Jis);
        assert!(bad.is_empty());
        assert_eq!(files.iter().map(|(_, v)| v.len()).collect::<Vec<_>>(), [5000, 5000, 2000]);
        // 金額: 合計が N12 を超えないように分かれる。1件の最大額は 9,999,999,999 円
        let items: Vec<Item> = (0..150).map(|i| mk(i, 9_999_999_999)).collect();
        let (files, bad) = build_files(FileKind::Ex91, 1, 0, &items, &now(), Code::Jis);
        assert!(bad.is_empty());
        assert_eq!(files.iter().map(|(_, v)| v.len()).sum::<usize>(), 150);
        assert!(files.len() >= 2);
        for (data, v) in &files {
            let (_, _, back) = parse_file(FileKind::Ex91, data).unwrap();
            assert_eq!(&back, v);
        }
        // 作れない 1 件 (使えない文字) は、別に返り、残りは送れる
        let Item::Transfer(mut t) = mk(1, 5) else { unreachable!() };
        t.name = "abc".into();
        let items = vec![mk(0, 1), Item::Transfer(t), mk(2, 1)];
        let (files, bad) = build_files(FileKind::Ex91, 1, 0, &items, &now(), Code::Jis);
        assert_eq!(bad.len(), 1);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].1.len(), 2);
    }

    #[test]
    fn mixed_kinds_rejected() {
        let s = sample();
        assert!(build_file(FileKind::Ex91, 1, 0, &s, &now(), Code::Jis).is_err());
    }
}
