//! 全銀ネット (中央): 為替通知の中継、仕向超過額管理、日次清算。docs/spec-net.md §5。

use crate::charcode::Code;
use crate::config::Conf;
use crate::control::{CenterCode, FILE_NAME_ERR, FileCtl, OTHER_ERR, RECORD_LEN_ERR, make_name, name_kind};
use crate::exchange::*;
use crate::handler::{Handler, InFile, OutFile, PeerCfg};
use crate::record::{FileKind, fmt, kind_from_name};
use crate::server::Server;
use crate::session::{Opts, send_file, tune};
use crate::store::Store;
use crate::util::{Clock, Result, lock, proto};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io;
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const WHO: &str = "全銀ネット";

/// 仕向超過額の限度額の上限 (円)。i64 の計算が溢れないようにするため。
const MAX_LIMIT: u64 = 1_000_000_000_000_000;

#[derive(Clone, Debug)]
pub struct BankEntry {
    pub bank_no: u16,
    pub name: String,
    pub addr: String,
    pub peer: PeerCfg,
    /// 仕向超過額の限度額 (円)
    pub limit: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Settle {
    /// 差額決済
    Netting,
    /// 取引ごとの即時決済 (1億円以上)
    Rtgs,
}

/// 仕訳。為替通知の資金の動き (支払側 → 受取側) を1件ごとに記録する。
#[derive(Clone, Debug)]
pub struct Entry {
    pub seq: u64,
    pub date: String,
    pub handling_no: u64,
    /// 91 振込、92 返却
    pub ty: u8,
    pub payer: u16,
    pub payee: u16,
    pub amount: u64,
    pub settle: Settle,
    pub settled: bool,
}

pub struct NetState {
    clock: Clock,
    store: Store,
    pub own: CenterCode,
    pub banks: BTreeMap<u16, BankEntry>,
    pub ledger: Vec<Entry>,
    outq: BTreeMap<u16, Vec<Item>>,
    seen: BTreeSet<(u16, u64, u8)>,
    settle_no: u32,
    cycles: HashMap<(u16, u8), u8>,
    /// 清算前の差額決済の仕向超過額 (支払い − 受取り)。仕訳から作り直せる集計値
    exposure: BTreeMap<u16, i64>,
    /// 振込 (種別91) の仕訳の位置: (仕向, 被仕向, 取扱番号) → ledger の添字
    index: HashMap<(u16, u16, u64), usize>,
}

pub struct Row {
    pub bank_no: u16,
    pub name: String,
    pub paid: u64,
    pub received: u64,
    pub net: i64,
}

pub struct Settlement {
    pub no: u32,
    pub rows: Vec<Row>,
    pub rtgs: Vec<Entry>,
    pub text: String,
}

fn yen(n: i64) -> String {
    let s = n.abs().to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if n < 0 { format!("-{out}") } else { out }
}

fn item_key_transfer(t: &Transfer) -> (u16, u64, u8) {
    (t.src_bank, t.handling_no, 91)
}

fn corrupt<T>(file: &str, line: &str) -> Result<T> {
    proto(format!("状態ファイル {file} の行が壊れている: {line:?}"))
}

impl NetState {
    /// 状態ファイルがあれば読み込む。あるのに読めない、解釈できない場合は Err (黙って初期状態に戻さない)。
    pub fn new(clock: Clock, store: Store, own: CenterCode, banks: Vec<BankEntry>) -> Result<NetState> {
        let mut s = NetState {
            clock,
            store,
            own,
            banks: banks.into_iter().map(|b| (b.bank_no, b)).collect(),
            ledger: Vec::new(),
            outq: BTreeMap::new(),
            seen: BTreeSet::new(),
            settle_no: 0,
            cycles: HashMap::new(),
            exposure: BTreeMap::new(),
            index: HashMap::new(),
        };
        s.load()?;
        for i in 0..s.ledger.len() {
            s.index_entry(i);
        }
        Ok(s)
    }

    /// 設定: 全体の `center`、`[bank 銀行番号]` に `center`、`password`、`key`、`addr`、`limit`、`name`。
    pub fn from_conf(conf: &Conf, clock: Clock, store: Store) -> Result<NetState> {
        let own = CenterCode::parse(conf.global.req("center")?)?;
        let mut banks = Vec::new();
        for sec in conf.of_kind("bank") {
            let bank_no: u16 = match sec.name.parse::<u64>().ok().filter(|n| (1..=9999).contains(n)) {
                Some(n) => n as u16,
                None => return proto(format!("銀行番号は 1〜9999: {}", sec.name)),
            };
            let limit = match sec.req("limit")?.parse::<u64>() {
                Ok(l) if l <= MAX_LIMIT => l,
                _ => return proto(format!("limit は 0〜{MAX_LIMIT} の整数 (円)")),
            };
            banks.push(BankEntry {
                bank_no,
                name: sec.get("name").unwrap_or(&sec.name).to_string(),
                addr: sec.req("addr")?.to_string(),
                peer: PeerCfg::from_section(sec, own)?,
                limit,
            });
        }
        NetState::new(clock, store, own, banks)
    }

    fn log(&self, msg: &str) {
        self.store.journal(&self.clock, WHO, msg);
    }

    // ---- 永続化 ----

    fn load(&mut self) -> Result<()> {
        for l in self.store.lines("ledger.tsv")? {
            let f: Vec<&str> = l.split('\t').collect();
            let p = |i: usize| f.get(i).and_then(|s| s.parse::<u64>().ok());
            match (f.len(), p(0), p(2), p(3), p(4), p(5), p(6)) {
                (9, Some(seq), Some(h), Some(ty), Some(payer), Some(payee), Some(amount)) => self.ledger.push(Entry {
                    seq,
                    date: f[1].to_string(),
                    handling_no: h,
                    ty: ty as u8,
                    payer: payer as u16,
                    payee: payee as u16,
                    amount,
                    settle: if f[7] == "R" { Settle::Rtgs } else { Settle::Netting },
                    settled: f[8] == "1",
                }),
                _ => return corrupt("ledger.tsv", &l),
            }
        }
        for l in self.store.lines("outq.tsv")? {
            match l
                .split_once('\t')
                .and_then(|(b, rest)| Some((b.parse::<u16>().ok()?, Item::from_line(rest).ok()?)))
            {
                Some((b, item)) => self.outq.entry(b).or_default().push(item),
                None => return corrupt("outq.tsv", &l),
            }
        }
        for l in self.store.lines("seen.tsv")? {
            let f: Vec<u64> = l.split('\t').filter_map(|s| s.parse().ok()).collect();
            match f[..] {
                [a, b, c] => {
                    self.seen.insert((a as u16, b, c as u8));
                }
                _ => return corrupt("seen.tsv", &l),
            }
        }
        for l in self.store.lines("meta.tsv")? {
            match l.split('\t').collect::<Vec<_>>()[..] {
                ["settle", n] if n.parse::<u32>().is_ok() => self.settle_no = n.parse().unwrap(),
                _ => return corrupt("meta.tsv", &l),
            }
        }
        Ok(())
    }

    fn save(&self) {
        let ledger: Vec<String> = self
            .ledger
            .iter()
            .map(|e| {
                format!(
                    "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                    e.seq,
                    e.date,
                    e.handling_no,
                    e.ty,
                    e.payer,
                    e.payee,
                    e.amount,
                    if e.settle == Settle::Rtgs { "R" } else { "N" },
                    e.settled as u8
                )
            })
            .collect();
        let outq: Vec<String> = self
            .outq
            .iter()
            .flat_map(|(b, v)| v.iter().map(move |i| format!("{b}\t{}", i.to_line())))
            .collect();
        let seen: Vec<String> = self.seen.iter().map(|(a, b, c)| format!("{a}\t{b}\t{c}")).collect();
        for (name, lines) in [
            ("ledger.tsv", ledger),
            ("outq.tsv", outq),
            ("seen.tsv", seen),
            ("meta.tsv", vec![format!("settle\t{}", self.settle_no)]),
        ] {
            if let Err(e) = self.store.write(name, &lines) {
                eprintln!("[{WHO}] {name} の保存に失敗: {e}");
            }
        }
    }

    // ---- 仕訳と仕向超過額 ----

    /// 清算前の差額決済の仕向超過額 (支払い − 受取り)。
    pub fn exposure(&self, bank: u16) -> i64 {
        self.exposure.get(&bank).copied().unwrap_or(0)
    }

    fn index_entry(&mut self, i: usize) {
        let e = &self.ledger[i];
        if e.ty == 91 {
            self.index.insert((e.payer, e.payee, e.handling_no), i);
        }
        if !e.settled && e.settle == Settle::Netting {
            *self.exposure.entry(e.payer).or_default() += e.amount as i64;
            *self.exposure.entry(e.payee).or_default() -= e.amount as i64;
        }
    }

    fn find_entry(&self, payer: u16, payee: u16, handling_no: u64) -> Option<&Entry> {
        self.index.get(&(payer, payee, handling_no)).map(|&i| &self.ledger[i])
    }

    fn push_entry(&mut self, handling_no: u64, ty: u8, payer: u16, payee: u16, amount: u64, settle: Settle) {
        let seq = self.ledger.len() as u64 + 1;
        self.ledger.push(Entry {
            seq,
            date: self.clock.now().ymd(),
            handling_no,
            ty,
            payer,
            payee,
            amount,
            settle,
            settled: false,
        });
        self.index_entry(self.ledger.len() - 1);
    }

    fn enqueue(&mut self, bank: u16, item: Item) {
        self.outq.entry(bank).or_default().push(item);
    }

    /// 検査で受理できない振込を、仕向銀行へ返却する。
    fn reject(&mut self, t: &Transfer, to: u16, reason: u8) {
        self.log(&format!(
            "振込を返却 (事由 {reason:02}): 銀行{:04} 取扱番号 {} → 銀行{:04} {}円",
            t.src_bank, t.handling_no, t.dst_bank, t.amount
        ));
        let ret = Return {
            handling_no: t.handling_no,
            src_bank: to,
            dst_bank: t.dst_bank,
            reason,
            amount: t.amount,
        };
        self.enqueue(to, Item::Return(ret));
    }

    /// 銀行から受け取った為替通知を検査し、受理したものを宛先の配信待ちに入れる (§5.2、§5.4)。
    pub fn accept(&mut self, from: u16, items: Vec<Item>) {
        for item in items {
            if item.origin() != from {
                self.log(&format!(
                    "送信元が違う為替通知を破棄: 銀行{from:04} から種別{} の取扱番号 {}",
                    item.type_code(),
                    item.handling_no()
                ));
                if let Item::Transfer(t) = &item {
                    self.reject(t, from, REASON_BAD_MESSAGE);
                }
                continue;
            }
            // 重複の記録は、受理したときに付ける (検査で落ちた為替通知が、あとの正しい為替通知を妨げないように)
            if self.seen.contains(&item.key()) {
                self.log(&format!("重複を無視: 種別{} 取扱番号 {}", item.type_code(), item.handling_no()));
                continue;
            }
            match item {
                Item::Transfer(t) => {
                    self.seen.insert(item_key_transfer(&t));
                    self.accept_transfer(t)
                }
                Item::Return(r) => {
                    let Some(orig) = self.find_entry(r.src_bank, r.dst_bank, r.handling_no).cloned() else {
                        self.log(&format!("元の振込がない返却を破棄: 取扱番号 {}", r.handling_no));
                        continue;
                    };
                    if r.amount > orig.amount {
                        self.log(&format!("元の金額を超える返却を破棄: 取扱番号 {}", r.handling_no));
                        continue;
                    }
                    self.seen.insert((r.src_bank, r.handling_no, 92));
                    self.push_entry(r.handling_no, 92, r.dst_bank, r.src_bank, r.amount, orig.settle);
                    self.log(&format!(
                        "返却を受理: 銀行{:04} → 銀行{:04} 取扱番号 {} {}円",
                        r.dst_bank, r.src_bank, r.handling_no, r.amount
                    ));
                    self.enqueue(r.src_bank, Item::Return(r));
                }
                Item::Recall(r) => {
                    self.seen.insert((r.src_bank, r.handling_no, 93));
                    if self.find_entry(r.src_bank, r.dst_bank, r.handling_no).is_some() {
                        self.enqueue(r.dst_bank, Item::Recall(r));
                    } else {
                        self.log(&format!("該当する振込がない組戻依頼: 取扱番号 {}", r.handling_no));
                        let ng = RecallNg {
                            handling_no: r.handling_no,
                            src_bank: r.src_bank,
                            dst_bank: r.dst_bank,
                            reason: RECALL_NOT_FOUND,
                        };
                        self.enqueue(r.src_bank, Item::RecallNg(ng));
                    }
                }
                Item::RecallNg(r) => {
                    // 組戻依頼を中継した振込に対する返事だけを通す
                    let asked = self.seen.contains(&(r.src_bank, r.handling_no, 93));
                    if asked && self.find_entry(r.src_bank, r.dst_bank, r.handling_no).is_some() {
                        self.seen.insert((r.src_bank, r.handling_no, 94));
                        self.enqueue(r.src_bank, Item::RecallNg(r));
                    } else {
                        self.log(&format!("組戻依頼のない組戻不能を破棄: 取扱番号 {}", r.handling_no));
                    }
                }
            }
        }
        self.save();
    }

    fn accept_transfer(&mut self, t: Transfer) {
        let (src, dst) = (t.src_bank, t.dst_bank);
        if !self.banks.contains_key(&dst) {
            return self.reject(&t, src, REASON_UNKNOWN_BANK);
        }
        if src == dst || t.amount == 0 {
            return self.reject(&t, src, REASON_BAD_MESSAGE);
        }
        let settle = if t.amount >= RTGS_THRESHOLD {
            Settle::Rtgs
        } else {
            Settle::Netting
        };
        if settle == Settle::Netting {
            let limit = self.banks[&src].limit as i64;
            if self.exposure(src) + t.amount as i64 > limit {
                return self.reject(&t, src, REASON_LIMIT);
            }
        }
        self.push_entry(t.handling_no, 91, src, dst, t.amount, settle);
        let how = if settle == Settle::Rtgs { "即時決済" } else { "差額決済" };
        self.log(&format!(
            "振込を受理 ({how}): 銀行{src:04} → 銀行{dst:04} 取扱番号 {} {}円",
            t.handling_no, t.amount
        ));
        self.enqueue(dst, Item::Transfer(t));
    }

    // ---- 配信 ----

    /// 配信待ちを、銀行ごと・種別ごとに分けたもの。
    pub fn pending(&self) -> Vec<(u16, FileKind, Vec<Item>)> {
        let mut out = Vec::new();
        for (&bank, items) in &self.outq {
            for kind in [FileKind::Ex91, FileKind::Ex92, FileKind::Ex93, FileKind::Ex94] {
                let v: Vec<Item> = items.iter().filter(|i| i.file_kind() == kind).cloned().collect();
                if !v.is_empty() {
                    out.push((bank, kind, v));
                }
            }
        }
        out
    }

    pub fn queued(&self, bank: u16) -> usize {
        self.outq.get(&bank).map_or(0, Vec::len)
    }

    fn remove_queued(&mut self, bank: u16, items: &[Item]) {
        if let Some(q) = self.outq.get_mut(&bank) {
            for it in items {
                if let Some(p) = q.iter().position(|x| x == it) {
                    q.remove(p);
                }
            }
        }
        self.save();
    }

    /// 為替通知のファイルを作れない為替通知を、配信待ちから外して記録する。
    fn quarantine(&mut self, bank: u16, bad: &[(Item, String)]) {
        for (item, why) in bad {
            self.log(&format!(
                "配信できない為替通知を隔離: 銀行{bank:04} 種別{} 取扱番号 {} ({why})",
                item.type_code(),
                item.handling_no()
            ));
            self.store.append("quarantine.tsv", &format!("{bank}\t{}", item.to_line()));
        }
        let items: Vec<Item> = bad.iter().map(|(i, _)| i.clone()).collect();
        self.remove_queued(bank, &items);
    }

    fn next_cycle(&mut self, bank: u16, kind: FileKind) -> u8 {
        let c = self.cycles.entry((bank, fmt(kind).type_code as u8)).or_insert(0);
        *c = *c % 99 + 1;
        *c
    }

    // ---- 清算 ----

    /// 清算前の仕訳を集計して清算一覧を作り、清算済みにする。
    pub fn settle(&mut self) -> Result<Settlement> {
        let mut paid: BTreeMap<u16, u64> = BTreeMap::new();
        let mut received: BTreeMap<u16, u64> = BTreeMap::new();
        let mut rtgs = Vec::new();
        for e in self.ledger.iter().filter(|e| !e.settled) {
            match e.settle {
                Settle::Netting => {
                    *paid.entry(e.payer).or_default() += e.amount;
                    *received.entry(e.payee).or_default() += e.amount;
                }
                Settle::Rtgs => rtgs.push(e.clone()),
            }
        }
        // 設定から外した銀行の仕訳が残っていても、差引の合計が 0 になるように、仕訳にある銀行は全部載せる
        let mut nos: BTreeSet<u16> = self.banks.keys().copied().collect();
        nos.extend(paid.keys().chain(received.keys()).copied());
        let rows: Vec<Row> = nos
            .into_iter()
            .map(|no| {
                let (p, r) = (paid.get(&no).copied().unwrap_or(0), received.get(&no).copied().unwrap_or(0));
                let name = self.banks.get(&no).map_or("(未登録)".to_string(), |b| b.name.clone());
                Row {
                    bank_no: no,
                    name,
                    paid: p,
                    received: r,
                    net: r as i64 - p as i64,
                }
            })
            .collect();
        if rows.iter().map(|r| r.net).sum::<i64>() != 0 {
            return proto("清算の差引の合計が 0 にならない (内部の不整合)");
        }
        self.settle_no += 1;
        let t = self.clock.now();
        let mut text = vec![
            format!("全銀ネット 清算一覧 {}-{:02}-{:02} 第{}回", t.y, t.mo, t.d, self.settle_no),
            String::new(),
        ];
        text.push("差額決済 (1億円未満)".into());
        text.push(format!(
            "{:<6}{:<16}{:>16}{:>16}{:>16}",
            "銀行", "名称", "支払合計", "受取合計", "差引"
        ));
        for r in &rows {
            text.push(format!(
                "{:04}  {:<16}{:>16}{:>16}{:>16}",
                r.bank_no,
                r.name,
                yen(r.paid as i64),
                yen(r.received as i64),
                yen(r.net)
            ));
        }
        text.push(format!("差引の合計: {}", yen(rows.iter().map(|r| r.net).sum())));
        text.push(String::new());
        text.push(format!("即時決済 (1億円以上) {} 件", rtgs.len()));
        for e in &rtgs {
            text.push(format!(
                "取扱番号 {:010}  種別{}  銀行{:04} → 銀行{:04}  {}円",
                e.handling_no,
                e.ty,
                e.payer,
                e.payee,
                yen(e.amount as i64)
            ));
        }
        let text = text.join("\n");
        for e in self.ledger.iter_mut().filter(|e| !e.settled) {
            e.settled = true;
        }
        self.exposure.clear();
        self.save();
        let name = format!("settlement-{}-{:02}.txt", t.ymd(), self.settle_no);
        let lines: Vec<String> = text.lines().map(str::to_string).collect();
        if let Err(e) = self.store.write(&name, &lines) {
            eprintln!("[{WHO}] 清算一覧の保存に失敗: {e}");
        }
        self.log(&format!("清算を実行: 第{}回、即時決済 {} 件", self.settle_no, rtgs.len()));
        Ok(Settlement {
            no: self.settle_no,
            rows,
            rtgs,
            text,
        })
    }

    /// CLI が置いた清算の依頼を実行する。結果は `settle-result.txt` に書く。
    pub fn take_requests(&mut self) {
        match self.store.take("settle-request") {
            Ok(Some(_)) => {
                let text = match self.settle() {
                    Ok(s) => s.text,
                    Err(e) => format!("清算に失敗: {e}"),
                };
                let lines: Vec<String> = text.lines().map(str::to_string).collect();
                if let Err(e) = self.store.write("settle-result.txt", &lines) {
                    eprintln!("[{WHO}] 清算結果の保存に失敗: {e}");
                }
                self.store.done("settle-request");
            }
            Ok(None) => {}
            Err(e) => self.log(&format!("清算の依頼を取り出せない: {e}")),
        }
    }

    /// 清算前の仕向超過額と、配信待ちの件数。
    pub fn report(&self) -> String {
        let mut out = vec![format!(
            "{:<6}{:<16}{:>16}{:>16}{:>10}",
            "銀行", "名称", "仕向超過額", "限度額", "配信待ち"
        )];
        for b in self.banks.values() {
            out.push(format!(
                "{:04}  {:<16}{:>16}{:>16}{:>10}",
                b.bank_no,
                b.name,
                yen(self.exposure(b.bank_no)),
                yen(b.limit as i64),
                self.queued(b.bank_no)
            ));
        }
        out.join("\n")
    }
}

/// 銀行から全銀ネットへ接続してきたときの受け口 (二次局)。
impl Handler for NetState {
    fn find_peer(&mut self, remote: &CenterCode, own: &CenterCode) -> Option<PeerCfg> {
        if *own != self.own {
            return None;
        }
        self.banks.values().find(|b| b.peer.remote == *remote).map(|b| b.peer.clone())
    }

    fn start_recv(&mut self, _: &PeerCfg, f: &FileCtl) -> std::result::Result<(), u8> {
        match kind_from_name(&name_kind(&f.name)) {
            Some(fm) if matches!(fm.kind, FileKind::Ex91 | FileKind::Ex92 | FileKind::Ex93 | FileKind::Ex94) => {
                if f.rec_len as usize == fm.rec_len {
                    Ok(())
                } else {
                    Err(RECORD_LEN_ERR)
                }
            }
            _ => Err(FILE_NAME_ERR),
        }
    }

    fn validate_recv(&mut self, peer: &PeerCfg, f: &InFile) -> std::result::Result<(), u8> {
        let fm = kind_from_name(&name_kind(&f.name)).ok_or(FILE_NAME_ERR)?;
        let (from, to, _) = parse_file(fm.kind, &f.data).map_err(|e| {
            self.log(&format!("{} の為替通知ファイルが不正: {e}", peer.name));
            OTHER_ERR
        })?;
        if peer.name.parse::<u16>() != Ok(from) || to != NET_BANK_NO {
            Err(OTHER_ERR)
        } else {
            Ok(())
        }
    }

    /// 閉局回答を返す前に呼ばれる。受理は (仕向銀行, 取扱番号) で重複を無視するので、送信側が再送しても二重にならない。
    fn commit_recv(&mut self, peer: &PeerCfg, f: InFile) {
        let Some(fm) = kind_from_name(&name_kind(&f.name)) else { return };
        match (peer.name.parse::<u16>(), parse_file(fm.kind, &f.data)) {
            (Ok(bank), Ok((_, _, items))) => self.accept(bank, items),
            _ => self.log("確定した為替通知ファイルを解析できない"),
        }
    }

    fn start_send(&mut self, _: &PeerCfg, _: &FileCtl) -> std::result::Result<Option<OutFile>, u8> {
        Ok(None)
    }

    fn commit_send(&mut self, _: &PeerCfg, _: &OutFile) {}
}

/// 配信待ちを、宛先の銀行へ送る (全銀ネットが一次局、連絡モード)。送れたファイルの数を返す。
/// 振込 → 返却 → 組戻依頼 → 組戻不能の順に送り、ある種別が送れなかった銀行には、あとの種別を送らない
/// (組戻依頼が振込より先に届かないようにするため)。
pub fn deliver_once(state: &Mutex<NetState>, opts: &Opts) -> usize {
    let (jobs, clock) = {
        let s = lock(state);
        (s.pending(), s.clock.clone())
    };
    let mut sent = 0;
    let mut failed: BTreeSet<u16> = BTreeSet::new();
    for (bank, kind, items) in jobs {
        if failed.contains(&bank) {
            continue;
        }
        let Some(entry) = lock(state).banks.get(&bank).cloned() else {
            continue;
        };
        let fm = fmt(kind);
        let (files, bad) = build_files(kind, NET_BANK_NO, bank, &items, &clock.now(), Code::Jis);
        if !bad.is_empty() {
            lock(state).quarantine(bank, &bad);
        }
        for (data, sent_items) in files {
            let cycle = lock(state).next_cycle(bank, kind);
            let file = OutFile {
                name: make_name(&fm.name_kind[..4], &fm.name_kind[4..], cycle),
                rec_len: fm.rec_len,
                code: Code::Jis,
                data,
                compress: false,
            };
            let result = connect(&entry.addr, opts)
                .map_err(Into::into)
                .and_then(|s| send_file(s, &entry.peer, &clock, &file, opts));
            let mut s = lock(state);
            match result {
                Ok(()) => {
                    s.log(&format!("銀行{bank:04} へ種別{} を {} 件配信", fm.type_code, sent_items.len()));
                    s.remove_queued(bank, &sent_items);
                    sent += 1;
                }
                Err(e) => {
                    s.log(&format!("銀行{bank:04} への配信に失敗 (後で再試行): {e}"));
                    failed.insert(bank);
                    break;
                }
            }
        }
    }
    sent
}

pub fn connect(addr: &str, opts: &Opts) -> io::Result<TcpStream> {
    let a = addr
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "アドレスを解決できない"))?;
    let s = TcpStream::connect_timeout(&a, Duration::from_secs(3))?;
    tune(&s, opts)?;
    Ok(s)
}

/// 全銀ネットを起動する。`interval` ごとに配信待ちを送る。
pub fn start(state: NetState, listen: &str, opts: Opts, interval: Duration) -> io::Result<(Server, Arc<Mutex<NetState>>)> {
    let clock = state.clock.clone();
    let st = Arc::new(Mutex::new(state));
    let mut srv = Server::start(listen, st.clone(), clock, opts.clone(), WHO)?;
    let st2 = st.clone();
    srv.every(interval, move || {
        lock(&st2).take_requests();
        deliver_once(&st2, &opts);
    });
    Ok((srv, st))
}
