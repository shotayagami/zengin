//! 銀行ノードの、振込入金通知と為替通知以外の業務 (docs/spec-net.md §4.8)。
//!
//! - 取引の履歴を口座ごとに記録し、入出金取引明細・残高通知 (照会) を作る。
//! - 振込口座照会・預金口座振替・財形貯蓄の依頼 (連絡) を処理し、処理結果明細を照会用の送信待ちに積む。
//! - 外国送金依頼・荷為替輸入信用状発行依頼 (連絡) は、受け付けて記録する (処理は銀行の担当者が行う想定)。
//! - そのほかの照会の業務 (住宅ローン、借入金、外国為替など) は、銀行の担当者が `zengin bank publish` で
//!   登録したファイルを、企業が取りに来たときに渡す。

use super::{AcctKey, BankState, Company, MAX_INFLIGHT, acct_str, corrupt, fingerprint};
use crate::charcode::Code;
use crate::control::RETRY_REQ;
use crate::handler::OutFile;
use crate::record::{FileKind, Fmt, Parsed, Rec, fmt, kind_from_name_mode};
use crate::util::{Result, proto};
use std::collections::{BTreeMap, HashMap};

/// 口座の取引1件。
#[derive(Clone, Debug)]
pub(super) struct Txn {
    pub id: u64,
    pub acct: AcctKey,
    /// 西暦 YYYYMMDD
    pub date: String,
    /// 1 入金、2 出金
    pub io: u8,
    /// 取引区分 (別冊): 11 振込、14 振替、18 その他
    pub txn: u8,
    pub amount: u64,
    /// 取引後の残高
    pub balance: i64,
    /// 相手の名前など
    pub desc: String,
    /// 入出金取引明細で通知済み
    pub notified: bool,
}

/// 照会で企業に渡すのを待っているファイル。
#[derive(Clone, Debug)]
pub(super) struct OutItem {
    pub id: u64,
    pub company: String,
    /// ファイル名の識別部 (産業別コード + データコード)
    pub name_kind: String,
    pub data: Vec<u8>,
}

/// 照会で渡している途中のもの。閉局まで終わったら確定する。
#[derive(Clone, Debug)]
pub(super) enum Flight {
    Txns(Vec<u64>),
    Out(u64),
    Nothing,
}

pub(super) const TXN_TRANSFER: u8 = 11;
pub(super) const TXN_DEBIT: u8 = 14;
pub(super) const TXN_OTHER: u8 = 18;

/// 業務の状態。
#[derive(Default)]
pub(super) struct Biz {
    pub history: Vec<Txn>,
    pub outbox: Vec<OutItem>,
    /// 財形貯蓄の残高: (企業, 加入者コード) → 残高
    pub zaikei: BTreeMap<(String, String), u64>,
    /// 業務ごとに最後に渡し終えたファイル: (企業, 識別部) → データ。再送要求でもう一度渡す
    pub last_sent: HashMap<(String, String), Vec<u8>>,
    /// 照会で渡している途中のもの: (企業, 識別部) → [(渡したファイルの指紋, 中身)]
    pub inflight: HashMap<(String, String), Vec<(u64, Flight)>>,
    pub next_id: u64,
}

/// 振込の業務 (データ・レコードが受取人の口座と金額)。総合振込・給与振込と同じ処理をする。
pub const TRANSFER_KINDS: [FileKind; 9] = [
    FileKind::Sogo,
    FileKind::Kyuyo,
    FileKind::Shoyo,
    FileKind::ChihoKyuyo,
    FileKind::ChihoShoyo,
    FileKind::Haito,
    FileKind::NenkinShintaku,
    FileKind::KotekiNenkin,
    FileKind::Iryo,
];

/// 受け付けて記録するだけの依頼。
const RECORD_ONLY: [FileKind; 2] = [FileKind::Soukin, FileKind::ShinyoJo];

fn hex(d: &[u8]) -> String {
    d.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok())
        .collect()
}

/// 西暦 YYYYMMDD → 和暦 (令和) YYMMDD。
fn reiwa(ymd: &str) -> String {
    let y: u16 = ymd.get(..4).and_then(|s| s.parse().ok()).unwrap_or(2019);
    format!("{:02}{}", y.saturating_sub(2018), ymd.get(4..8).unwrap_or("0101"))
}

/// 文字数で切る (固定長の欄に入れるため)。
fn cut(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

/// 名義の比較 (スペースの有無と英字の大小を無視する)。
fn same_name(a: &str, b: &str) -> bool {
    let norm = |s: &str| s.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_uppercase();
    norm(a) == norm(b)
}

impl BankState {
    // ---- 状態の保存 ----

    pub(super) fn load_biz(&mut self) -> Result<()> {
        for l in self.store.lines("history.tsv")? {
            let f: Vec<&str> = l.split('\t').collect();
            let n = |i: usize| f.get(i).and_then(|s| s.parse::<u64>().ok());
            match (
                f.len(),
                n(0),
                n(1),
                n(2),
                n(5),
                n(6),
                n(7),
                f.get(8).and_then(|s| s.parse::<i64>().ok()),
            ) {
                (11, Some(id), Some(b), Some(k), Some(io), Some(txn), Some(amount), Some(balance)) => self.biz.history.push(Txn {
                    id,
                    acct: (b as u16, k as u8, f[3].into()),
                    date: f[4].into(),
                    io: io as u8,
                    txn: txn as u8,
                    amount,
                    balance,
                    desc: f[9].into(),
                    notified: f[10] == "1",
                }),
                _ => return corrupt("history.tsv", &l),
            }
        }
        for l in self.store.lines("outbox.tsv")? {
            let f: Vec<&str> = l.split('\t').collect();
            match (f.len(), f.first().and_then(|s| s.parse().ok()), f.get(3).and_then(|s| unhex(s))) {
                (4, Some(id), Some(data)) => self.biz.outbox.push(OutItem {
                    id,
                    company: f[1].into(),
                    name_kind: f[2].into(),
                    data,
                }),
                _ => return corrupt("outbox.tsv", &l),
            }
        }
        for l in self.store.lines("zaikei.tsv")? {
            let f: Vec<&str> = l.split('\t').collect();
            match (f.len(), f.get(2).and_then(|s| s.parse().ok())) {
                (3, Some(b)) => {
                    self.biz.zaikei.insert((f[0].into(), f[1].into()), b);
                }
                _ => return corrupt("zaikei.tsv", &l),
            }
        }
        for l in self.store.lines("lastsent.tsv")? {
            let f: Vec<&str> = l.split('\t').collect();
            match (f.len(), f.get(2).and_then(|s| unhex(s))) {
                (3, Some(d)) => {
                    self.biz.last_sent.insert((f[0].into(), f[1].into()), d);
                }
                _ => return corrupt("lastsent.tsv", &l),
            }
        }
        let max_id = self
            .biz
            .history
            .iter()
            .map(|t| t.id)
            .chain(self.biz.outbox.iter().map(|o| o.id))
            .max();
        self.biz.next_id = max_id.map_or(1, |m| m + 1);
        Ok(())
    }

    pub(super) fn biz_files(&self) -> Vec<(&'static str, Vec<String>)> {
        let b = &self.biz;
        vec![
            (
                "history.tsv",
                b.history
                    .iter()
                    .map(|t| {
                        format!(
                            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                            t.id,
                            acct_str(&t.acct),
                            t.date,
                            t.io,
                            t.txn,
                            t.amount,
                            t.balance,
                            t.desc,
                            t.notified as u8
                        )
                    })
                    .collect(),
            ),
            (
                "outbox.tsv",
                b.outbox
                    .iter()
                    .map(|o| format!("{}\t{}\t{}\t{}", o.id, o.company, o.name_kind, hex(&o.data)))
                    .collect(),
            ),
            ("zaikei.tsv", b.zaikei.iter().map(|((c, m), v)| format!("{c}\t{m}\t{v}")).collect()),
            (
                "lastsent.tsv",
                b.last_sent.iter().map(|((c, k), d)| format!("{c}\t{k}\t{}", hex(d))).collect(),
            ),
        ]
    }

    // ---- 残高の変更と取引の記録 ----

    /// 口座の残高を `delta` だけ変え、取引の履歴に記録する。口座がなければ何もしない。
    pub(super) fn post(&mut self, key: &AcctKey, delta: i64, txn: u8, desc: &str) {
        let Some(a) = self.accounts.get_mut(key) else { return };
        a.balance += delta;
        let balance = a.balance;
        let id = self.biz.next_id;
        self.biz.next_id += 1;
        self.biz.history.push(Txn {
            id,
            acct: key.clone(),
            date: self.clock.now().ymd(),
            io: if delta >= 0 { 1 } else { 2 },
            txn,
            amount: delta.unsigned_abs(),
            balance,
            desc: desc.replace(['\t', '\n'], " "),
            notified: false,
        });
    }

    /// 口座の取引の履歴 (古い順)。
    pub fn history(&self, key: &AcctKey) -> Vec<(String, u8, u64, i64, String)> {
        let rows = self.biz.history.iter().filter(|t| &t.acct == key);
        rows.map(|t| (t.date.clone(), t.io, t.amount, t.balance, t.desc.clone())).collect()
    }

    // ---- 企業からの依頼 (連絡) ----

    /// 企業のファイル (総合振込・給与振込を除く) を処理する。指紋による重複の判定は済んでいる。
    pub(super) fn process_request(&mut self, company: &Company, kind: FileKind, p: &Parsed) {
        let result = match kind {
            FileKind::KozaShokai => self.koza_shokai(company, p),
            FileKind::Furikae => self.furikae(company, p),
            FileKind::Zaikei => self.zaikei(company, p),
            k if RECORD_ONLY.contains(&k) => {
                let title = fmt(k).title;
                self.store.append(
                    "inbox.tsv",
                    &format!("{}\t{}\t{title}\t{}", self.clock.now().ymd(), company.name, p.data.len()),
                );
                self.log(&format!("{}: {title} を受け付けた ({} 件)", company.name, p.data.len()));
                Ok(())
            }
            k => proto(format!("処理できない業務: {k:?}")),
        };
        if let Err(e) = result {
            self.log(&format!("{}: 依頼を処理できない: {e}", company.name));
        }
    }

    /// 依頼の項目を、同じ名前の項目がある結果のレコードへ写す。
    fn copied(from: &Rec, to: Rec) -> Rec {
        let names: Vec<&'static str> = to.layout.fields.iter().map(|f| f.name).collect();
        names
            .into_iter()
            .filter(|n| from.has(n) && !["data_kind", "type", "code"].contains(n))
            .fold(to, |r, n| r.set(n, from.get(n)))
    }

    fn push_out(&mut self, company: &str, f: &Fmt, data: Vec<u8>) {
        let id = self.biz.next_id;
        self.biz.next_id += 1;
        self.biz.outbox.push(OutItem {
            id,
            company: company.into(),
            name_kind: f.name_kind.into(),
            data,
        });
        self.log(&format!("{company}: {} を照会の送信待ちに積んだ", f.title));
    }

    /// 振込口座照会: 自行の口座の有無と名義を確かめる。他行の口座は、確かめられないので「その他」(9) とする。
    fn koza_shokai(&mut self, company: &Company, p: &Parsed) -> Result<()> {
        let out = fmt(FileKind::KozaShokaiKekka);
        let mut data = Vec::new();
        for d in &p.data {
            let key: AcctKey = (
                d.num("branch_no") as u16,
                d.num("acct_kind") as u8,
                format!("{:07}", d.num("acct_no")),
            );
            let result = if d.num("bank_no") != self.bank_no as u64 {
                9
            } else {
                match self.accounts.get(&key) {
                    None => 1,
                    Some(a) if !same_name(&a.name, d.get("name")) => 2,
                    Some(_) => 0,
                }
            };
            data.push(Self::copied(d, out.new_data()).set_num("result_code", result));
        }
        let file = out.build(p.code, Self::copied(&p.header, out.new_header()), &data)?;
        self.push_out(&company.name, out, file);
        Ok(())
    }

    /// 預金口座振替: 自行の預金者の口座から引き落とし、企業 (収納企業) の口座へ入金する。
    fn furikae(&mut self, company: &Company, p: &Parsed) -> Result<()> {
        let out = fmt(FileKind::FurikaeKekka);
        let mut data = Vec::new();
        let client = p.header.get("client_name").to_string();
        for d in &p.data {
            let key: AcctKey = (
                d.num("branch_no") as u16,
                d.num("acct_kind") as u8,
                format!("{:07}", d.num("acct_no")),
            );
            let amount = d.num("amount");
            let result = if d.num("bank_no") != self.bank_no as u64 {
                9 // その他 (他行の口座)
            } else if !self.accounts.contains_key(&key) {
                2 // 取引なし
            } else if self.balance(&key).unwrap_or(0) < amount as i64 {
                1 // 資金不足
            } else {
                self.post(&key, -(amount as i64), TXN_DEBIT, &client);
                self.post(&company.acct, amount as i64, TXN_DEBIT, d.get("name"));
                0 // 振替済
            };
            data.push(Self::copied(d, out.new_data()).set_num("result_code", result));
        }
        let ok = data.iter().filter(|d| d.num("result_code") == 0).count();
        self.log(&format!("{}: 預金口座振替 {} 件中 {ok} 件を振り替えた", company.name, data.len()));
        let file = out.build(p.code, Self::copied(&p.header, out.new_header()), &data)?;
        self.push_out(&company.name, out, file);
        Ok(())
    }

    /// 財形貯蓄: 加入者ごとの積立額の合計を企業の口座から引き落とし、加入者の財形の残高に積み立てる。
    fn zaikei(&mut self, company: &Company, p: &Parsed) -> Result<()> {
        let out = fmt(FileKind::ZaikeiTsumitate);
        let parts = ["salary_amount", "bonus_amount", "other1_amount", "other2_amount"];
        let mut data = Vec::new();
        for d in &p.data {
            let member = d.get("member_code").to_string();
            let amount: u64 = parts.iter().map(|f| d.num(f)).sum();
            let mut r = Self::copied(d, out.new_data());
            if self.balance(&company.acct).unwrap_or(0) < amount as i64 {
                self.log(&format!(
                    "{}: 残高不足のため、加入者 {member} の積立 {amount} 円をしなかった",
                    company.name
                ));
                for f in parts {
                    r = r.set_num(f, 0);
                }
            } else if amount > 0 {
                self.post(&company.acct, -(amount as i64), TXN_OTHER, "ｻﾞｲｹｲ");
                *self.biz.zaikei.entry((company.name.clone(), member.clone())).or_insert(0) += amount;
            }
            let balance = self.biz.zaikei.get(&(company.name.clone(), member)).copied().unwrap_or(0);
            data.push(r.set_num("balance", balance));
        }
        let file = out.build(p.code, Self::copied(&p.header, out.new_header()), &data)?;
        self.push_out(&company.name, out, file);
        Ok(())
    }

    /// 銀行の担当者が登録したファイル (`zengin bank publish`) を、照会の送信待ちに積む。
    pub(super) fn publish(&mut self, company: &str, name_kind: &str, data: Vec<u8>) -> Result<()> {
        if !self.companies.iter().any(|c| c.name == company) {
            return proto(format!("企業 {company} が登録にない"));
        }
        let Some(f) = kind_from_name_mode(name_kind, true) else {
            return proto(format!("照会の業務ではない: {name_kind}"));
        };
        f.parse(&data)?;
        self.push_out(company, f, data);
        Ok(())
    }

    // ---- 照会 ----

    /// 照会の業務 (振込入金通知を除く) で、企業に渡すファイルを作る。なければ None。
    pub(super) fn start_send_biz(&mut self, company: &Company, f: &Fmt, kind: u8) -> Result<Option<OutFile>> {
        let key = (company.name.clone(), f.name_kind.to_string());
        let built = match f.kind {
            FileKind::Meisai => self.build_meisai(company)?,
            FileKind::Zandaka => Some((self.build_zandaka(company)?, Flight::Nothing)),
            _ => self
                .biz
                .outbox
                .iter()
                .find(|o| o.company == company.name && o.name_kind == f.name_kind)
                .map(|o| (o.data.clone(), Flight::Out(o.id))),
        };
        let (data, flight) = match built {
            Some(x) => x,
            // 再送要求で、新しく渡すものがなければ、最後に渡し終えたファイルをもう一度渡す
            None if kind == RETRY_REQ => match self.biz.last_sent.get(&key) {
                Some(d) => (d.clone(), Flight::Nothing),
                None => return Ok(None),
            },
            None => return Ok(None),
        };
        let flights = self.biz.inflight.entry(key).or_default();
        flights.push((fingerprint(&[&data]), flight));
        if flights.len() > MAX_INFLIGHT {
            flights.remove(0);
        }
        let code = if data.first() == Some(&0xF1) { Code::Ebcdic } else { Code::Jis };
        Ok(Some(OutFile {
            name: [0; 12], // 二次局が要求のファイル名に置き換える
            rec_len: f.rec_len,
            rec_id: 0,
            code,
            data,
            compress: false,
        }))
    }

    /// 閉局まで終わった照会を確定する。
    pub(super) fn commit_send_biz(&mut self, company: &str, f: &Fmt, data: &[u8]) {
        let key = (company.to_string(), f.name_kind.to_string());
        let h = fingerprint(&[data]);
        let found = self.biz.inflight.get_mut(&key).and_then(|v| {
            let i = v.iter().position(|(x, _)| *x == h)?;
            Some(v.remove(i).1)
        });
        match found {
            Some(Flight::Txns(ids)) => {
                for t in self.biz.history.iter_mut().filter(|t| ids.contains(&t.id)) {
                    t.notified = true;
                }
            }
            Some(Flight::Out(id)) => self.biz.outbox.retain(|o| o.id != id),
            Some(Flight::Nothing) => {}
            None => return,
        }
        self.biz.last_sent.insert(key, data.to_vec());
        self.log(&format!("{company}: {} を渡した", f.title));
        self.save();
    }

    /// 入出金取引明細: 企業の口座の、まだ通知していない取引。
    fn build_meisai(&self, c: &Company) -> Result<Option<(Vec<u8>, Flight)>> {
        let txns: Vec<&Txn> = self.biz.history.iter().filter(|t| t.acct == c.acct && !t.notified).collect();
        let (Some(first), Some(last)) = (txns.first(), txns.last()) else {
            return Ok(None);
        };
        let f = fmt(FileKind::Meisai);
        let acct = self.accounts.get(&c.acct).map_or("", |a| a.name.as_str());
        let prev = first.balance
            - if first.io == 1 {
                first.amount as i64
            } else {
                -(first.amount as i64)
            };
        let header = f
            .new_header()
            .set("created", reiwa(&self.clock.now().ymd()))
            .set("from", reiwa(&first.date))
            .set("to", reiwa(&last.date))
            .set_num("bank_no", self.bank_no as u64)
            .set("bank_name", cut(&self.name, 15))
            .set_num("branch_no", c.acct.0 as u64)
            .set_num("acct_kind", c.acct.1 as u64)
            .set("acct_no", format!("{:0>10}", c.acct.2))
            .set("acct_name", cut(acct, 40))
            .set_num("od_flag", if prev < 0 { 2 } else { 1 })
            .set_num("prev_balance", prev.unsigned_abs());
        let label = if [1, 2, 4].contains(&c.acct.1) {
            "普通・当座・貯蓄"
        } else {
            "通知・定期・積立定期"
        };
        let data: Vec<Rec> = txns
            .iter()
            .map(|t| {
                let r = f.new_data_of(label).expect("入出金取引明細の様式");
                let r = if r.has("inquiry_no") {
                    r.set_num("inquiry_no", t.id % 100_000_000)
                } else {
                    r.set_num("ident_no", t.id % 100_000_000)
                };
                r.set("acct_date", reiwa(&t.date))
                    .set("start_date", reiwa(&t.date))
                    .set_num("io_kind", t.io as u64)
                    .set_num("txn_kind", t.txn as u64)
                    .set_num("amount", t.amount)
                    .set("summary", cut(&t.desc, 20))
            })
            .collect();
        let bytes = f.build(Code::Jis, header, &data)?;
        Ok(Some((bytes, Flight::Txns(txns.iter().map(|t| t.id).collect()))))
    }

    /// 残高通知: 企業の口座の、いまの残高。
    fn build_zandaka(&self, c: &Company) -> Result<Vec<u8>> {
        let f = fmt(FileKind::Zandaka);
        let now = self.clock.now();
        let a = self.accounts.get(&c.acct);
        let balance = a.map_or(0, |a| a.balance);
        let last = self.biz.history.iter().rev().find(|t| t.acct == c.acct);
        let header = f
            .new_header()
            .set_num("notice_kind", 1)
            .set("created", reiwa(&now.ymd()))
            .set_num("client_code", c.code)
            .set("client_name", cut(a.map_or("", |a| a.name.as_str()), 40))
            .set_num("bank_no", self.bank_no as u64)
            .set("bank_name", cut(&self.name, 15))
            .set_num("branch_no", c.acct.0 as u64);
        let od = if balance < 0 { 2 } else { 1 };
        let d = f
            .new_data()
            .set("base_date", reiwa(&now.ymd()))
            .set("base_time", &now.hms()[..4])
            .set_num("branch_no", c.acct.0 as u64)
            .set_num("acct_kind", c.acct.1 as u64)
            .set("acct_no", format!("{:0>10}", c.acct.2))
            .set("acct_name", cut(a.map_or("", |a| a.name.as_str()), 40))
            .set_num("od_flag", od)
            .set_num("balance", balance.unsigned_abs())
            .set_num("avail_od_flag", od)
            .set_num("avail_balance", balance.unsigned_abs())
            .set("last_txn_date", last.map_or(String::new(), |t| reiwa(&t.date)));
        f.build(Code::Jis, header, &[d])
    }
}
