//! 銀行ノード: 企業の振込の依頼を受け付けて為替通知にし、全銀ネットから来た為替通知を口座に反映する。
//! 振込以外の業務 (入出金取引明細、口座振替など) は `biz`。
//! docs/spec-net.md §4。

use crate::charcode::Code;
use crate::config::Conf;
use crate::control::{CenterCode, FILE_NAME_ERR, FileCtl, OTHER_ERR, RECORD_LEN_ERR, make_name, name_kind};
use crate::exchange::*;
use crate::handler::{Handler, InFile, OutFile, PeerCfg};
use crate::net::connect;
use crate::record::{FileKind, Parsed, fmt, kind_from_name, kind_from_name_mode, name_chars_ok};
use crate::server::Server;
use crate::session::{Opts, send_file};
use crate::store::Store;
use crate::util::{Clock, Result, days_from_ymd, lock, proto, small};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io;
use std::sync::{Arc, Mutex};
use std::time::Duration;

mod biz;

pub use biz::TRANSFER_KINDS;
use biz::{Biz, TXN_OTHER, TXN_TRANSFER};

const WHO: &str = "銀行";
/// 受け取った企業のファイルの記録を保持する日数。この間に同じファイルが届いたら重複とみなす
const FILES_RETAIN_DAYS: i64 = 7;
/// 照会で渡している途中のものを、企業・業務ごとに覚えておく数 (同時の照会と、途中で終わった照会のため)
const MAX_INFLIGHT: usize = 8;
const NET: &str = "net";

/// 口座の識別: (支店番号, 預金種目, 口座番号7桁)
pub type AcctKey = (u16, u8, String);

#[derive(Clone, Debug)]
pub struct Company {
    pub name: String,
    pub code: u64,
    pub peer: PeerCfg,
    pub acct: AcctKey,
}

#[derive(Clone, Debug)]
pub struct Account {
    pub name: String,
    pub balance: i64,
}

/// 企業へまだ通知していない振込入金 (または取消)。
#[derive(Clone, Debug)]
struct Notice {
    id: u64,
    company: String,
    amount: u64,
    client_code: u64,
    client_name: String,
    src_bank: u16,
    edi: String,
    date: String,
    inquiry_no: u64,
    /// 組戻しによる取消の通知 (通知済みの入金を取り消す)
    cancel: bool,
    /// 元の為替通知 (仕向銀行, 取扱番号)。自行内の振込は (0, 0)
    ref_src: u16,
    ref_no: u64,
}

/// 入金に付いてきた、通知の元の情報。
#[derive(Clone, Debug)]
struct Origin {
    client_code: u64,
    client_name: String,
    src_bank: u16,
    edi: String,
    ref_src: u16,
    ref_no: u64,
}

/// 他行宛に送った振込。
#[derive(Clone, Debug)]
pub struct Sent {
    pub dst_bank: u16,
    pub acct: AcctKey,
    pub amount: u64,
    /// sent / returned / recall-requested / recall-refused / failed
    pub status: String,
}

/// 他行から受けて入金した振込 (組戻しの判断に使う)。
#[derive(Clone, Debug)]
struct Credited {
    acct: AcctKey,
    amount: u64,
    recalled: bool,
    client_code: u64,
    client_name: String,
    edi: String,
}

pub struct BankState {
    clock: Clock,
    store: Store,
    pub bank_no: u16,
    pub name: String,
    pub own: CenterCode,
    pub net: PeerCfg,
    pub net_addr: String,
    pub companies: Vec<Company>,
    pub accounts: BTreeMap<AcctKey, Account>,
    /// 受信済みと同じ企業のファイルが届いたら、二重ファイル伝送 (16) で拒否する。偽なら、黙って重複を無視する (既定)
    pub reject_duplicates: bool,
    outq: Vec<Item>,
    notices: Vec<Notice>,
    pub sent: BTreeMap<u64, Sent>,
    credited: BTreeMap<(u16, u64), Credited>,
    seen: BTreeSet<(u16, u64, u8)>,
    /// 確定した企業のファイル: 「企業 \t 指紋」→ 受け取った日 (YYYYMMDD)。再送された同じファイルを無視するため。
    /// 日付を判定に含めない (日をまたいだ再送も重複とみなす)。`FILES_RETAIN_DAYS` 日より古い記録は捨てる
    files_seen: BTreeMap<String, String>,
    /// 取扱番号の通し番号 (年をまたいでも重ならない)
    counter: u64,
    next_notice_id: u64,
    inquiry: u64,
    cycles: HashMap<String, u8>,
    /// 振込入金通知として渡している途中の通知: 企業 → [(渡したファイルの指紋, 通知の識別子)]。
    /// 同じ企業が同時に照会しても、閉局まで終わったファイルに入っていた通知だけを通知済みにする
    inflight: HashMap<String, Vec<(u64, Vec<u64>)>>,
    /// 最後に渡し終えた振込入金通知のファイル (企業ごと)。再送要求で、もう一度渡すために残す
    last_notice: HashMap<String, Vec<u8>>,
    /// 振込以外の業務の状態
    biz: Biz,
}

fn acct_str(k: &AcctKey) -> String {
    format!("{}\t{}\t{}", k.0, k.1, k.2)
}

fn acct_text(k: &AcctKey) -> String {
    format!("{}-{}-{}", k.0, k.1, k.2)
}

/// FNV-1a 64bit。ファイルの重複判定用で、暗号学的な強さは要らない。
fn fingerprint(parts: &[&[u8]]) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for p in parts {
        for &b in *p {
            h = (h ^ b as u64).wrapping_mul(0x0100_0000_01b3);
        }
    }
    h
}

fn corrupt<T>(file: &str, line: &str) -> Result<T> {
    proto(format!("状態ファイル {file} の行が壊れている: {line:?}"))
}

/// 1回の振込入金通知に入れる通知の件数の上限。
const MAX_NOTICES_PER_FILE: usize = 5000;

impl BankState {
    pub fn new(clock: Clock, store: Store, bank_no: u16, name: &str, own: CenterCode, net: PeerCfg, net_addr: &str) -> BankState {
        BankState {
            clock,
            store,
            bank_no,
            name: name.into(),
            own,
            net,
            net_addr: net_addr.into(),
            companies: Vec::new(),
            accounts: BTreeMap::new(),
            reject_duplicates: false,
            outq: Vec::new(),
            notices: Vec::new(),
            sent: BTreeMap::new(),
            credited: BTreeMap::new(),
            seen: BTreeSet::new(),
            files_seen: BTreeMap::new(),
            counter: 0,
            next_notice_id: 0,
            inquiry: 0,
            cycles: HashMap::new(),
            inflight: HashMap::new(),
            last_notice: HashMap::new(),
            biz: Biz::default(),
        }
    }

    /// 取引先企業を加える。企業の口座も作る。
    pub fn add_company(&mut self, c: Company, opening: i64) {
        self.accounts.entry(c.acct.clone()).or_insert(Account {
            name: c.name.clone(),
            balance: opening,
        });
        self.companies.push(c);
    }

    pub fn add_account(&mut self, key: AcctKey, name: &str, opening: i64) {
        self.accounts.entry(key).or_insert(Account {
            name: name.into(),
            balance: opening,
        });
    }

    /// 設定: 全体の `bank`、`name`、`center`、`net_center`、`net_password`、`net_key`、`net_addr`。
    /// `[company 名前]` に `center`、`password`、`key`、`code`、`branch`、`kind`、`account`、`balance`。
    /// `[account 名前]` に `branch`、`kind`、`number`、`balance`。口座の残高は初回だけ設定を使い、以降は状態ファイルが優先。
    /// 状態ファイルがあるのに読めない、解釈できない場合は Err (黙って初期状態に戻さない)。
    pub fn from_conf(conf: &Conf, clock: Clock, store: Store) -> Result<BankState> {
        let g = &conf.global;
        let num = |s: &str, what: &str| -> Result<u64> { s.parse().or_else(|_| proto(format!("{what} は整数: {s:?}"))) };
        let own = CenterCode::parse(g.req("center")?)?;
        let net = PeerCfg::new(
            NET,
            own,
            CenterCode::parse(g.req("net_center")?)?,
            g.req("net_password")?,
            g.req("net_key")?,
        )?;
        let bank_no: u16 = small(num(g.req("bank")?, "bank")?, "bank")?;
        if !(1..=9999).contains(&bank_no) {
            return proto("bank は 1〜9999");
        }
        let name = g.get("name").unwrap_or("ｷﾞﾝｺｳ");
        if !name_chars_ok(name) || name.chars().count() > 15 {
            return proto(format!("name は半角カナ・英大文字・数字などで15文字以内: {name:?}"));
        }
        let mut b = BankState::new(clock, store, bank_no, name, own, net, g.req("net_addr")?);
        b.reject_duplicates = match g.get("duplicate").unwrap_or("ignore") {
            "ignore" => false,
            "reject" => true,
            x => return proto(format!("duplicate は ignore か reject: {x:?}")),
        };
        let acct_of = |sec: &crate::config::Section, number_key: &str| -> Result<AcctKey> {
            let branch: u16 = small(num(sec.req("branch")?, "branch")?, "branch")?;
            let kind: u8 = small(num(sec.req("kind")?, "kind")?, "kind")?;
            let number = num(sec.req(number_key)?, number_key)?;
            if branch > 999 || kind > 9 || number > 9_999_999 {
                return proto(format!(
                    "[{} {}] の支店(3桁)・預金種目(1桁)・口座番号(7桁)が範囲外",
                    sec.kind, sec.name
                ));
            }
            Ok((branch, kind, format!("{number:07}")))
        };
        for sec in conf.of_kind("company") {
            if sec.name == NET {
                return proto("企業名に net は使えない");
            }
            let code = num(sec.req("code")?, "code")?;
            if code > 9_999_999_999 {
                return proto("code は 10 桁以内");
            }
            let c = Company {
                name: sec.name.clone(),
                code,
                peer: PeerCfg::from_section(sec, own)?,
                acct: acct_of(sec, "account")?,
            };
            let opening = sec.get("balance").map_or(Ok(0), |v| num(v, "balance"))?;
            // 口座の名義 (入出金取引明細・残高通知・振込入金通知に入る)。省略時はセクション名
            let holder = sec.get("name").unwrap_or(&sec.name);
            if !name_chars_ok(holder) || holder.chars().count() > 40 {
                return proto(format!(
                    "[company {}] の名義は半角カナ・英大文字・数字などで40文字以内にする (name で指定できる): {holder:?}",
                    sec.name
                ));
            }
            let acct = c.acct.clone();
            b.add_company(c, small::<i64>(opening, "balance")?);
            if let Some(a) = b.accounts.get_mut(&acct) {
                a.name = holder.to_string();
            }
        }
        for sec in conf.of_kind("account") {
            let opening = sec.get("balance").map_or(Ok(0), |v| num(v, "balance"))?;
            // 名義 (半角カナなど)。振込口座照会で、受取人名と照らし合わせる。省略時はセクション名
            let name = sec.get("name").unwrap_or(&sec.name);
            if !name_chars_ok(name) || name.chars().count() > 30 {
                return proto(format!(
                    "[account {}] の name は半角カナ・英大文字・数字などで30文字以内: {name:?}",
                    sec.name
                ));
            }
            b.add_account(acct_of(sec, "number")?, name, small::<i64>(opening, "balance")?);
        }
        b.load()?;
        Ok(b)
    }

    fn log(&self, msg: &str) {
        self.store.journal(&self.clock, &format!("{WHO}{:04}", self.bank_no), msg);
    }

    // ---- 永続化 ----

    /// 状態ファイルがあれば読み込む。口座は、設定で増えた分を残す。
    pub fn load(&mut self) -> Result<()> {
        let n = |s: &str| s.parse::<u64>().ok();
        for l in self.store.lines("accounts.tsv")? {
            let f: Vec<&str> = l.split('\t').collect();
            match (
                f.len(),
                n(f[0]),
                n(f.get(1).unwrap_or(&"")),
                f.get(4).and_then(|s| s.parse::<i64>().ok()),
            ) {
                (5, Some(b), Some(k), Some(bal)) => {
                    self.accounts.insert(
                        (b as u16, k as u8, f[2].to_string()),
                        Account {
                            name: f[3].to_string(),
                            balance: bal,
                        },
                    );
                }
                _ => return corrupt("accounts.tsv", &l),
            }
        }
        for l in self.store.lines("outq.tsv")? {
            match Item::from_line(&l) {
                Ok(i) => self.outq.push(i),
                Err(_) => return corrupt("outq.tsv", &l),
            }
        }
        for l in self.store.lines("notices.tsv")? {
            let f: Vec<&str> = l.split('\t').collect();
            let g = |i: usize| f.get(i).and_then(|s| s.parse::<u64>().ok());
            match (f.len(), g(0), g(2), g(3), g(5), g(9), g(10), g(11)) {
                (12, Some(id), Some(amount), Some(code), Some(src), Some(inq), Some(rs), Some(rn)) => self.notices.push(Notice {
                    id,
                    company: f[1].into(),
                    amount,
                    client_code: code,
                    client_name: f[4].into(),
                    src_bank: src as u16,
                    edi: f[6].into(),
                    date: f[7].into(),
                    inquiry_no: inq,
                    cancel: f[8] == "1",
                    ref_src: rs as u16,
                    ref_no: rn,
                }),
                _ => return corrupt("notices.tsv", &l),
            }
        }
        for l in self.store.lines("sent.tsv")? {
            let f: Vec<&str> = l.split('\t').collect();
            match (
                f.len(),
                n(f[0]),
                n(f.get(1).unwrap_or(&"")),
                n(f.get(2).unwrap_or(&"")),
                n(f.get(3).unwrap_or(&"")),
                n(f.get(5).unwrap_or(&"")),
            ) {
                (7, Some(h), Some(dst), Some(br), Some(kind), Some(amount)) => {
                    self.sent.insert(
                        h,
                        Sent {
                            dst_bank: dst as u16,
                            acct: (br as u16, kind as u8, f[4].into()),
                            amount,
                            status: f[6].into(),
                        },
                    );
                }
                _ => return corrupt("sent.tsv", &l),
            }
        }
        for l in self.store.lines("credited.tsv")? {
            let f: Vec<&str> = l.split('\t').collect();
            let g = |i: usize| f.get(i).and_then(|s| s.parse::<u64>().ok());
            match (f.len(), g(0), g(1), g(2), g(3), g(5), g(7)) {
                (10, Some(src), Some(h), Some(br), Some(kind), Some(amount), Some(code)) => {
                    self.credited.insert(
                        (src as u16, h),
                        Credited {
                            acct: (br as u16, kind as u8, f[4].into()),
                            amount,
                            recalled: f[6] == "1",
                            client_code: code,
                            client_name: f[8].into(),
                            edi: f[9].into(),
                        },
                    );
                }
                _ => return corrupt("credited.tsv", &l),
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
        for l in self.store.lines("files.tsv")? {
            // 企業 \t 日付 \t 指紋
            match l.split('\t').collect::<Vec<_>>()[..] {
                [c, d, h] if days_from_ymd(d).is_some() => {
                    self.files_seen.insert(format!("{c}\t{h}"), d.to_string());
                }
                _ => return corrupt("files.tsv", &l),
            }
        }
        for l in self.store.lines("meta.tsv")? {
            let f: Vec<&str> = l.split('\t').collect();
            match f[..] {
                ["handling", c] if c.parse::<u64>().is_ok() => self.counter = c.parse().unwrap(),
                ["notice_id", c] if c.parse::<u64>().is_ok() => self.next_notice_id = c.parse().unwrap(),
                ["inquiry", c] if c.parse::<u64>().is_ok() => self.inquiry = c.parse().unwrap(),
                ["cycle", name, c] if c.parse::<u8>().is_ok() => {
                    self.cycles.insert(name.into(), c.parse().unwrap());
                }
                _ => return corrupt("meta.tsv", &l),
            }
        }
        for l in self.store.lines("lastnotice.tsv")? {
            let data = l.split_once('\t').and_then(|(name, hex)| {
                let bytes = (0..hex.len()).step_by(2).map(|i| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok());
                Some((name.to_string(), bytes.collect::<Option<Vec<u8>>>()?))
            });
            match data {
                Some((name, bytes)) => self.last_notice.insert(name, bytes),
                None => return corrupt("lastnotice.tsv", &l),
            };
        }
        self.load_biz()
    }

    fn save(&self) {
        let accounts: Vec<String> = self
            .accounts
            .iter()
            .map(|(k, a)| format!("{}\t{}\t{}", acct_str(k), a.name, a.balance))
            .collect();
        let outq: Vec<String> = self.outq.iter().map(Item::to_line).collect();
        let notices: Vec<String> = self
            .notices
            .iter()
            .map(|n| {
                format!(
                    "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                    n.id,
                    n.company,
                    n.amount,
                    n.client_code,
                    n.client_name,
                    n.src_bank,
                    n.edi,
                    n.date,
                    n.cancel as u8,
                    n.inquiry_no,
                    n.ref_src,
                    n.ref_no
                )
            })
            .collect();
        let sent: Vec<String> = self
            .sent
            .iter()
            .map(|(h, s)| format!("{h}\t{}\t{}\t{}\t{}", s.dst_bank, acct_str(&s.acct), s.amount, s.status))
            .collect();
        let credited: Vec<String> = self
            .credited
            .iter()
            .map(|((src, h), c)| {
                format!(
                    "{src}\t{h}\t{}\t{}\t{}\t{}\t{}\t{}",
                    acct_str(&c.acct),
                    c.amount,
                    c.recalled as u8,
                    c.client_code,
                    c.client_name,
                    c.edi
                )
            })
            .collect();
        let seen: Vec<String> = self.seen.iter().map(|(a, b, c)| format!("{a}\t{b}\t{c}")).collect();
        let files: Vec<String> = self
            .files_seen
            .iter()
            .map(|(k, d)| {
                let (c, h) = k.split_once('\t').unwrap_or((k, ""));
                format!("{c}\t{d}\t{h}")
            })
            .collect();
        let mut meta = vec![
            format!("handling\t{}", self.counter),
            format!("notice_id\t{}", self.next_notice_id),
            format!("inquiry\t{}", self.inquiry),
        ];
        meta.extend(self.cycles.iter().map(|(k, v)| format!("cycle\t{k}\t{v}")));
        let hex = |d: &Vec<u8>| d.iter().map(|b| format!("{b:02x}")).collect::<String>();
        let last: Vec<String> = self.last_notice.iter().map(|(k, d)| format!("{k}\t{}", hex(d))).collect();
        for (name, lines) in [
            ("accounts.tsv", accounts),
            ("outq.tsv", outq),
            ("notices.tsv", notices),
            ("sent.tsv", sent),
            ("credited.tsv", credited),
            ("seen.tsv", seen),
            ("files.tsv", files),
            ("meta.tsv", meta),
            ("lastnotice.tsv", last),
        ] {
            if let Err(e) = self.store.write(name, &lines) {
                eprintln!("[{WHO}] {name} の保存に失敗: {e}");
            }
        }
        for (name, lines) in self.biz_files() {
            if let Err(e) = self.store.write(name, &lines) {
                eprintln!("[{WHO}] {name} の保存に失敗: {e}");
            }
        }
    }

    // ---- 口座 ----

    pub fn balance(&self, key: &AcctKey) -> Option<i64> {
        self.accounts.get(key).map(|a| a.balance)
    }

    fn company_of(&self, key: &AcctKey) -> Option<&Company> {
        self.companies.iter().find(|c| &c.acct == key)
    }

    /// 取扱番号は永続の通し番号 (10桁)。年や日をまたいでも重ならない。
    fn next_handling_no(&mut self) -> u64 {
        self.counter = self.counter % 9_999_999_999 + 1;
        self.counter
    }

    fn push_notice(&mut self, company: String, amount: u64, o: Origin, cancel: bool) {
        self.next_notice_id += 1;
        self.inquiry = self.inquiry % 999_999 + 1;
        let date = self.clock.now().reiwa_yymmdd();
        self.notices.push(Notice {
            id: self.next_notice_id,
            company,
            amount,
            client_code: o.client_code,
            client_name: o.client_name,
            src_bank: o.src_bank,
            edi: o.edi,
            date,
            inquiry_no: self.inquiry,
            cancel,
            ref_src: o.ref_src,
            ref_no: o.ref_no,
        });
    }

    /// 口座に入金する。企業の口座なら、振込入金通知の待ち行列にも積む。
    fn credit(&mut self, key: &AcctKey, amount: u64, origin: Option<Origin>) {
        let desc = origin.as_ref().map_or("ﾍﾝｷﾔｸ".to_string(), |o| o.client_name.clone());
        self.post(key, amount as i64, TXN_TRANSFER, &desc);
        if let (Some(o), Some(company)) = (origin, self.company_of(key).map(|c| c.name.clone())) {
            self.push_notice(company, amount, o, false);
        }
    }

    // ---- 企業からの総合振込・給与振込 ----

    /// 企業のファイルの重複判定に使う指紋 (企業、ファイル名とデータ)。
    fn file_fp(&self, company: &str, f: &InFile) -> String {
        format!("{company}\t{:016x}", fingerprint(&[&f.name, &f.data]))
    }

    /// 受信済みの記録のうち、保持する期間を過ぎたものを捨てる。
    fn prune_files_seen(&mut self) {
        let today = days_from_ymd(&self.clock.now().ymd()).unwrap_or(0);
        self.files_seen
            .retain(|_, d| days_from_ymd(d).is_some_and(|x| today - x <= FILES_RETAIN_DAYS));
    }

    /// 同じファイルを、保持する期間の中で受け取ったか。
    fn file_seen(&mut self, key: &str) -> bool {
        self.prune_files_seen();
        self.files_seen.contains_key(key)
    }

    fn process_company_file(&mut self, company: &Company, kind: FileKind, p: &Parsed) {
        let who = company.name.clone();
        let client_name = p.header.get("client_name").to_string();
        for d in &p.data {
            let amount = d.num("amount");
            let (dst_bank, dst_branch, dst_kind, dst_no) = (
                d.num("bank_no") as u16,
                d.num("branch_no") as u16,
                d.num("acct_kind") as u8,
                d.get("acct_no").to_string(),
            );
            if amount == 0 {
                self.log(&format!("{who}: 金額 0 の行を無視"));
                continue;
            }
            if self.balance(&company.acct).unwrap_or(0) < amount as i64 {
                self.log(&format!(
                    "{who}: 残高不足のため {amount} 円の振込を不能とした (受取人 {})",
                    d.get("name")
                ));
                continue;
            }
            self.post(&company.acct, -(amount as i64), TXN_TRANSFER, d.get("name"));
            let dst_key: AcctKey = (dst_branch, dst_kind, dst_no.clone());
            if dst_bank == self.bank_no {
                // 自行内: 全銀ネットを通さない
                if self.accounts.contains_key(&dst_key) {
                    let o = Origin {
                        client_code: company.code,
                        client_name: client_name.clone(),
                        src_bank: self.bank_no,
                        edi: String::new(),
                        ref_src: 0,
                        ref_no: 0,
                    };
                    self.credit(&dst_key, amount, Some(o));
                    self.log(&format!("{who}: 自行内振込 {amount} 円 → {}", acct_text(&dst_key)));
                } else {
                    self.post(&company.acct, amount as i64, TXN_TRANSFER, "ﾓﾄﾞｼ");
                    self.log(&format!("{who}: 自行内の口座がないため {amount} 円を戻した"));
                }
                continue;
            }
            let edi = if kind == FileKind::Sogo && d.get("edi_flag") == "Y" {
                format!("{:<10}{:<10}", d.get("cust1"), d.get("cust2"))
            } else {
                String::new()
            };
            let handling_no = self.next_handling_no();
            let t = Transfer {
                handling_no,
                src_bank: self.bank_no,
                src_branch: company.acct.0,
                dst_bank,
                dst_branch,
                acct_kind: dst_kind,
                acct_no: dst_no,
                name: d.get("name").to_string(),
                amount,
                client_code: company.code,
                client_name: client_name.clone(),
                edi: edi.trim_end().to_string(),
            };
            self.sent.insert(
                handling_no,
                Sent {
                    dst_bank,
                    acct: company.acct.clone(),
                    amount,
                    status: "sent".into(),
                },
            );
            self.log(&format!(
                "{who}: 他行宛の振込 取扱番号 {handling_no} → 銀行{dst_bank:04} {amount} 円"
            ));
            self.outq.push(Item::Transfer(t));
        }
        self.save();
    }

    // ---- 全銀ネットからの為替通知 ----

    fn process_net_items(&mut self, items: Vec<Item>) {
        for item in items {
            if item.destination() != self.bank_no {
                self.log(&format!(
                    "宛先が違う為替通知を破棄: 種別{} 取扱番号 {}",
                    item.type_code(),
                    item.handling_no()
                ));
                continue;
            }
            // 重複の記録は、受理したときに付ける (検査で落ちた為替通知が、あとの正しい為替通知を妨げないように)
            if self.seen.contains(&item.key()) {
                self.log(&format!("重複を無視: 種別{} 取扱番号 {}", item.type_code(), item.handling_no()));
                continue;
            }
            match item {
                Item::Transfer(t) => {
                    self.seen.insert((t.src_bank, t.handling_no, 91));
                    self.receive_transfer(t)
                }
                Item::Return(r) => {
                    // 自行が送った振込で、相手の銀行と金額が合い、まだ返却されていないものだけ
                    let ok = self.sent.get(&r.handling_no).is_some_and(|s| {
                        s.dst_bank == r.dst_bank && r.amount <= s.amount && matches!(s.status.as_str(), "sent" | "recall-requested")
                    });
                    if !ok {
                        self.log(&format!(
                            "該当しない返却を無視: 取扱番号 {} 銀行{:04} {} 円",
                            r.handling_no, r.dst_bank, r.amount
                        ));
                        continue;
                    }
                    self.seen.insert((r.src_bank, r.handling_no, 92));
                    let s = self.sent.get_mut(&r.handling_no).unwrap();
                    s.status = "returned".into();
                    let key = s.acct.clone();
                    self.credit(&key, r.amount, None);
                    self.log(&format!(
                        "返却を受けて入金し直した: 取扱番号 {} {} 円 (事由 {:02})",
                        r.handling_no, r.amount, r.reason
                    ));
                }
                Item::Recall(r) => {
                    self.seen.insert((r.src_bank, r.handling_no, 93));
                    self.receive_recall(r)
                }
                Item::RecallNg(r) => match self.sent.get_mut(&r.handling_no) {
                    Some(s) if s.dst_bank == r.dst_bank && s.status == "recall-requested" => {
                        s.status = "recall-refused".into();
                        self.seen.insert((r.src_bank, r.handling_no, 94));
                        self.log(&format!("組戻しを拒否された: 取扱番号 {} (事由 {:02})", r.handling_no, r.reason));
                    }
                    _ => self.log(&format!("組戻依頼中でない振込の組戻不能を無視: 取扱番号 {}", r.handling_no)),
                },
            }
        }
        self.save();
    }

    fn receive_transfer(&mut self, t: Transfer) {
        let key: AcctKey = (t.dst_branch, t.acct_kind, t.acct_no.clone());
        if self.accounts.contains_key(&key) {
            let o = Origin {
                client_code: t.client_code,
                client_name: t.client_name.clone(),
                src_bank: t.src_bank,
                edi: t.edi.clone(),
                ref_src: t.src_bank,
                ref_no: t.handling_no,
            };
            self.credit(&key, t.amount, Some(o));
            self.credited.insert(
                (t.src_bank, t.handling_no),
                Credited {
                    acct: key,
                    amount: t.amount,
                    recalled: false,
                    client_code: t.client_code,
                    client_name: t.client_name.clone(),
                    edi: t.edi.clone(),
                },
            );
            self.log(&format!("入金: 銀行{:04} 取扱番号 {} {} 円", t.src_bank, t.handling_no, t.amount));
        } else {
            self.log(&format!("口座がないため返却: 取扱番号 {} ({})", t.handling_no, acct_text(&key)));
            let r = Return {
                handling_no: t.handling_no,
                src_bank: t.src_bank,
                dst_bank: self.bank_no,
                reason: REASON_NO_ACCOUNT,
                amount: t.amount,
            };
            self.outq.push(Item::Return(r));
        }
    }

    fn receive_recall(&mut self, r: Recall) {
        let found = self.credited.get(&(r.src_bank, r.handling_no)).cloned();
        let reply = match found {
            Some(c) if !c.recalled && self.balance(&c.acct).unwrap_or(0) >= c.amount as i64 => {
                self.post(&c.acct, -(c.amount as i64), TXN_OTHER, "ｸﾐﾓﾄﾞｼ");
                self.credited.get_mut(&(r.src_bank, r.handling_no)).unwrap().recalled = true;
                self.cancel_notice(&r, &c);
                self.log(&format!("組戻しを承諾: 取扱番号 {} {} 円", r.handling_no, c.amount));
                Item::Return(Return {
                    handling_no: r.handling_no,
                    src_bank: r.src_bank,
                    dst_bank: self.bank_no,
                    reason: REASON_RECALL,
                    amount: c.amount,
                })
            }
            Some(_) => {
                self.log(&format!("組戻しできない (資金なし/処理済み): 取扱番号 {}", r.handling_no));
                Item::RecallNg(RecallNg {
                    handling_no: r.handling_no,
                    src_bank: r.src_bank,
                    dst_bank: self.bank_no,
                    reason: RECALL_NO_FUNDS,
                })
            }
            None => {
                self.log(&format!("該当する入金がない組戻依頼: 取扱番号 {}", r.handling_no));
                Item::RecallNg(RecallNg {
                    handling_no: r.handling_no,
                    src_bank: r.src_bank,
                    dst_bank: self.bank_no,
                    reason: RECALL_NOT_FOUND,
                })
            }
        };
        self.outq.push(reply);
    }

    /// 組戻しで入金を取り消すとき、企業へ通知する。まだ通知していなければ、その通知を取り下げる。
    /// 通知済みなら、取消の通知 (取消区分 1) を積む。
    fn cancel_notice(&mut self, r: &Recall, c: &Credited) {
        if let Some(p) = self
            .notices
            .iter()
            .position(|n| !n.cancel && n.ref_src == r.src_bank && n.ref_no == r.handling_no)
        {
            self.notices.remove(p);
        } else if let Some(company) = self.company_of(&c.acct).map(|x| x.name.clone()) {
            let o = Origin {
                client_code: c.client_code,
                client_name: c.client_name.clone(),
                src_bank: r.src_bank,
                edi: c.edi.clone(),
                ref_src: r.src_bank,
                ref_no: r.handling_no,
            };
            self.push_notice(company, c.amount, o, true);
        }
    }

    /// 他行宛に送った振込の組戻しを依頼する。
    pub fn request_recall(&mut self, handling_no: u64) -> Result<()> {
        let Some(s) = self.sent.get_mut(&handling_no) else {
            return proto(format!("取扱番号 {handling_no} の送金がない"));
        };
        if s.status != "sent" {
            return proto(format!("取扱番号 {handling_no} は組戻しできない状態: {}", s.status));
        }
        s.status = "recall-requested".into();
        let item = Item::Recall(Recall {
            handling_no,
            src_bank: self.bank_no,
            dst_bank: s.dst_bank,
            amount: s.amount,
        });
        self.log(&format!("組戻依頼: 取扱番号 {handling_no}"));
        self.outq.push(item);
        self.save();
        Ok(())
    }

    /// CLI が書いた組戻依頼のファイルを取り込む。
    pub fn take_requests(&mut self) {
        self.take_recall_requests();
        self.take_publish_requests();
    }

    fn take_recall_requests(&mut self) {
        let lines = match self.store.take("recall-requests.tsv") {
            Ok(Some(l)) => l,
            Ok(None) => return,
            Err(e) => return self.log(&format!("組戻依頼を取り出せない: {e}")),
        };
        for l in lines {
            match l.trim().parse::<u64>() {
                Ok(h) => {
                    if let Err(e) = self.request_recall(h) {
                        self.log(&format!("組戻依頼を取り込めない: {e}"));
                    }
                }
                Err(_) => self.log(&format!("組戻依頼の行が不正: {l:?}")),
            }
        }
        self.store.done("recall-requests.tsv");
    }

    fn take_publish_requests(&mut self) {
        // 担当者が登録した照会用のファイル (zengin bank publish): 企業 \t 識別部 \t 16進
        let lines = match self.store.take("publish-requests.tsv") {
            Ok(Some(l)) => l,
            Ok(None) => return,
            Err(e) => return self.log(&format!("登録されたファイルを取り出せない: {e}")),
        };
        for l in lines {
            let f: Vec<&str> = l.split('\t').collect();
            let data = f.get(2).and_then(|h| {
                (0..h.len())
                    .step_by(2)
                    .map(|i| u8::from_str_radix(h.get(i..i + 2)?, 16).ok())
                    .collect()
            });
            match (f.len(), data) {
                (3, Some(data)) => {
                    if let Err(e) = self.publish(f[0], f[1], data) {
                        self.log(&format!("登録されたファイルを積めない: {e}"));
                    }
                }
                _ => self.log("登録されたファイルの行が不正"),
            }
        }
        self.store.done("publish-requests.tsv");
        self.save();
    }

    /// 為替通知のファイルを作れない為替通知を、送信待ちから外して記録する。
    /// 振込なら、引き落とした資金を企業の口座へ戻す。
    fn quarantine(&mut self, bad: &[(Item, String)]) {
        for (item, why) in bad {
            self.log(&format!(
                "送信できない為替通知を隔離: 種別{} 取扱番号 {} ({why})",
                item.type_code(),
                item.handling_no()
            ));
            self.store.append("quarantine.tsv", &item.to_line());
            if let Item::Transfer(t) = item {
                if let Some(s) = self.sent.get_mut(&t.handling_no).filter(|s| s.status == "sent") {
                    s.status = "failed".into();
                    let key = s.acct.clone();
                    self.post(&key, t.amount as i64, TXN_TRANSFER, "ﾓﾄﾞｼ");
                }
            }
            if let Some(p) = self.outq.iter().position(|x| x == item) {
                self.outq.remove(p);
            }
        }
        self.save();
    }

    pub fn queued(&self) -> usize {
        self.outq.len()
    }

    pub fn pending_notices(&self, company: &str) -> usize {
        self.notices.iter().filter(|n| n.company == company).count()
    }

    /// 口座の残高と、送金の状態の一覧。
    pub fn statement(&self) -> String {
        let mut out = vec![
            format!("銀行{:04} {}", self.bank_no, self.name),
            "口座 (支店-種目-番号 名義 残高)".to_string(),
        ];
        for (k, a) in &self.accounts {
            out.push(format!("  {}  {}  {}", acct_text(k), a.name, a.balance));
        }
        out.push("他行宛の送金 (取扱番号 宛先 金額 状態)".into());
        for (h, s) in &self.sent {
            out.push(format!("  {h:010}  銀行{:04}  {}  {}", s.dst_bank, s.amount, s.status));
        }
        out.push(format!("全銀ネット宛の送信待ち: {} 件", self.outq.len()));
        out.join("\n")
    }

    // ---- 振込入金通知 ----

    fn build_notice_file(&mut self, company: &Company) -> Result<Option<OutFile>> {
        // 1ファイルに入る件数と合計金額 (トレーラの桁) に収まる分だけを渡す。残りは次回
        let mut mine: Vec<Notice> = Vec::new();
        let mut sum = 0u64;
        for n in self.notices.iter().filter(|n| n.company == company.name) {
            if mine.len() >= MAX_NOTICES_PER_FILE || sum + n.amount > MAX_AMOUNT_PER_FILE {
                break;
            }
            sum += n.amount;
            mine.push(n.clone());
        }
        if mine.is_empty() {
            return Ok(None);
        }
        let f = fmt(FileKind::Nyukin);
        let today = self.clock.now().reiwa_yymmdd();
        let holder = self.accounts.get(&company.acct).map(|a| a.name.clone()).unwrap_or_default();
        let holder: String = if name_chars_ok(&holder) {
            holder.chars().take(40).collect()
        } else {
            String::new()
        };
        let h = f
            .new_header()
            .set("created", today.clone())
            .set("from", today.clone())
            .set("to", today)
            .set_num("bank_no", self.bank_no as u64)
            .set("bank_name", self.name.clone())
            .set_num("branch_no", company.acct.0 as u64)
            .set_num("acct_kind", company.acct.1 as u64)
            .set("acct_no", company.acct.2.clone())
            .set("acct_name", holder);
        let data: Vec<_> = mine
            .iter()
            .map(|n| {
                f.new_data()
                    .set_num("inquiry_no", n.inquiry_no)
                    .set("acct_date", n.date.clone())
                    .set("start_date", n.date.clone())
                    .set_num("amount", n.amount)
                    .set_num("client_code", n.client_code)
                    .set("client_name", n.client_name.clone())
                    .set("src_bank", format!("{:04}", n.src_bank))
                    .set_num("cancel", n.cancel as u64)
                    .set("edi", n.edi.clone())
            })
            .collect();
        let bytes = f.build(Code::Jis, h, &data)?;
        let flights = self.inflight.entry(company.name.clone()).or_default();
        flights.push((fingerprint(&[&bytes]), mine.iter().map(|n| n.id).collect()));
        if flights.len() > MAX_INFLIGHT {
            flights.remove(0); // 途中で終わった照会の分。通知は消さずに残っているので、次の照会でまた渡す
        }
        let cycle = self.cycles.get(&company.name).copied().unwrap_or(0) % 99 + 1;
        Ok(Some(OutFile {
            name: make_name("5020", "0001", cycle),
            rec_len: f.rec_len,
            rec_id: 0,
            code: Code::Jis,
            data: bytes,
            compress: false,
        }))
    }
}

impl Handler for BankState {
    fn find_peer(&mut self, remote: &CenterCode, own: &CenterCode) -> std::result::Result<PeerCfg, u8> {
        if *own != self.own {
            return Err(crate::control::PEER_CODE_ERR);
        }
        if self.net.remote == *remote {
            return Ok(self.net.clone());
        }
        let found = self.companies.iter().find(|c| c.peer.remote == *remote);
        found.map(|c| c.peer.clone()).ok_or(crate::control::OWN_CODE_ERR)
    }

    fn start_recv(&mut self, peer: &PeerCfg, f: &FileCtl) -> std::result::Result<(), u8> {
        if f.rec_id != 0 {
            return Err(crate::control::RECORD_ID_ERR); // 業務のファイルは固定長だけ
        }
        let Some(fm) = kind_from_name(&name_kind(&f.name)) else {
            return Err(FILE_NAME_ERR);
        };
        let allowed = if peer.name == NET {
            matches!(fm.kind, FileKind::Ex91 | FileKind::Ex92 | FileKind::Ex93 | FileKind::Ex94)
        } else {
            // 企業からは、連絡モードの業務ならどれでも (為替通知を除く)
            !fm.inquiry && !matches!(fm.kind, FileKind::Ex91 | FileKind::Ex92 | FileKind::Ex93 | FileKind::Ex94)
        };
        if !allowed {
            return Err(FILE_NAME_ERR);
        }
        if f.rec_len as usize == fm.rec_len {
            Ok(())
        } else {
            Err(RECORD_LEN_ERR)
        }
    }

    fn validate_recv(&mut self, peer: &PeerCfg, f: &InFile) -> std::result::Result<(), u8> {
        let fm = kind_from_name(&name_kind(&f.name)).ok_or(FILE_NAME_ERR)?;
        if peer.name == NET {
            let (from, to, _) = parse_file(fm.kind, &f.data).map_err(|e| {
                self.log(&format!("為替通知ファイルが不正: {e}"));
                OTHER_ERR
            })?;
            return if from == NET_BANK_NO && to == self.bank_no {
                Ok(())
            } else {
                Err(OTHER_ERR)
            };
        }
        let p = fm.parse(&f.data).map_err(|e| {
            self.log(&format!("{}: 振込ファイルが不正: {e}", peer.name));
            OTHER_ERR
        })?;
        let Some(c) = self.companies.iter().find(|c| c.name == peer.name).cloned() else {
            return Err(OTHER_ERR);
        };
        let key = self.file_fp(&c.name, f);
        if self.reject_duplicates && self.file_seen(&key) {
            self.log(&format!("{}: 受信済みのファイルなので、二重ファイル伝送として拒否した", peer.name));
            return Err(crate::control::DUPLICATE_FILE);
        }
        // ヘッダの企業のコード (業務によって項目名が違う)
        let code_field = ["client_code", "remitter_code", "company_code"]
            .into_iter()
            .find(|f| p.header.has(f));
        if code_field.is_some_and(|f| p.header.num(f) != c.code) {
            self.log(&format!("{}: ヘッダの企業コードが登録と違う", peer.name));
            return Err(OTHER_ERR);
        }
        // 人名欄の文字の制限 (別冊 付録1 の注1)。振込の業務だけ
        let transfer = TRANSFER_KINDS.contains(&fm.kind);
        if transfer && (!name_chars_ok(p.header.get("client_name")) || p.data.iter().any(|d| !name_chars_ok(d.get("name")))) {
            self.log(&format!("{}: 人名欄に使えない文字がある", peer.name));
            return Err(OTHER_ERR);
        }
        Ok(())
    }

    /// 閉局回答を返す前に呼ばれる。企業のファイルは指紋で、為替通知は (仕向銀行, 取扱番号) で重複を無視するので、
    /// 送信側が閉局回答を受け取れずに再送しても、二重にはならない。
    fn commit_recv(&mut self, peer: &PeerCfg, f: InFile) {
        let Some(fm) = kind_from_name(&name_kind(&f.name)) else { return };
        if peer.name == NET {
            if let Ok((_, _, items)) = parse_file(fm.kind, &f.data) {
                self.process_net_items(items);
            }
        } else if let (Some(c), Ok(p)) = (self.companies.iter().find(|c| c.name == peer.name).cloned(), fm.parse(&f.data)) {
            // 企業が閉局回答を受け取れずに同じファイルを再送しても、二重に処理しない
            let key = self.file_fp(&c.name, &f);
            if self.file_seen(&key) {
                self.log(&format!("{}: 同じファイルを再受信したので無視した", c.name));
                return;
            }
            self.files_seen.insert(key, self.clock.now().ymd());
            if TRANSFER_KINDS.contains(&fm.kind) {
                self.process_company_file(&c, fm.kind, &p);
            } else {
                self.process_request(&c, fm.kind, &p);
            }
            self.save();
        }
    }

    fn start_send(&mut self, peer: &PeerCfg, f: &FileCtl) -> std::result::Result<Option<OutFile>, u8> {
        if peer.name == NET {
            return Err(FILE_NAME_ERR);
        }
        let Some(fm) = kind_from_name_mode(&name_kind(&f.name), true) else {
            return Err(FILE_NAME_ERR);
        };
        let Some(c) = self.companies.iter().find(|c| c.name == peer.name).cloned() else {
            return Err(OTHER_ERR);
        };
        if fm.kind != FileKind::Nyukin {
            return self.start_send_biz(&c, fm, f.kind).map_err(|e| {
                self.log(&format!("{} を作れない: {e}", fm.title));
                OTHER_ERR
            });
        }
        let built = self.build_notice_file(&c).map_err(|e| {
            self.log(&format!("振込入金通知を作れない: {e}"));
            OTHER_ERR
        })?;
        if built.is_some() || f.kind != crate::control::RETRY_REQ {
            return Ok(built);
        }
        // 再送要求で、まだ渡していない通知がなければ、最後に渡し終えたファイルをもう一度渡す
        let Some(data) = self.last_notice.get(&c.name).cloned() else {
            return Ok(None);
        };
        self.log(&format!("{}: 渡し終えた振込入金通知を、再送要求でもう一度渡す", c.name));
        Ok(Some(OutFile {
            name: f.name,
            rec_len: fmt(FileKind::Nyukin).rec_len,
            rec_id: 0,
            code: Code::Jis,
            data,
            compress: false,
        }))
    }

    /// 閉局回答を返したあとに呼ばれる (企業が受け取れなかった場合に通知を失わないため)。
    fn commit_send(&mut self, peer: &PeerCfg, f: &OutFile) {
        match kind_from_name_mode(&name_kind(&f.name), true) {
            Some(fm) if fm.kind != FileKind::Nyukin => return self.commit_send_biz(&peer.name, fm, &f.data),
            _ => {}
        }
        let h = fingerprint(&[&f.data]);
        let found = self.inflight.get_mut(&peer.name).and_then(|v| {
            let i = v.iter().position(|(x, _)| *x == h)?;
            Some(v.remove(i).1)
        });
        if let Some(ids) = found {
            self.last_notice.insert(peer.name.clone(), f.data.clone());
            // 件数ではなく識別子で消す。取得が重なっても、渡していない通知は消えない
            self.notices.retain(|x| !(x.company == peer.name && ids.contains(&x.id)));
            let c = self.cycles.entry(peer.name.clone()).or_insert(0);
            *c = *c % 99 + 1;
            self.log(&format!("{}: 振込入金通知 {} 件を通知済みにした", peer.name, ids.len()));
            self.save();
        }
    }
}

/// 送信待ちの為替通知を全銀ネットへ送る (銀行が一次局、連絡モード)。送れたファイルの数を返す。
/// 振込 → 返却 → 組戻依頼 → 組戻不能の順に送り、どれかが送れなかったら、あとの種別は送らない
/// (組戻依頼が振込より先に届かないようにするため)。
pub fn flush_to_net(state: &Mutex<BankState>, opts: &Opts) -> usize {
    let (outq, clock, bank_no, net, addr) = {
        let s = lock(state);
        (s.outq.clone(), s.clock.clone(), s.bank_no, s.net.clone(), s.net_addr.clone())
    };
    let mut sent = 0;
    for kind in [FileKind::Ex91, FileKind::Ex92, FileKind::Ex93, FileKind::Ex94] {
        let items: Vec<Item> = outq.iter().filter(|i| i.file_kind() == kind).cloned().collect();
        if items.is_empty() {
            continue;
        }
        let fm = fmt(kind);
        let (files, bad) = build_files(kind, bank_no, NET_BANK_NO, &items, &clock.now(), Code::Jis);
        if !bad.is_empty() {
            lock(state).quarantine(&bad);
        }
        for (data, sent_items) in files {
            let cycle = {
                let mut s = lock(state);
                let c = s.cycles.entry(format!("net{}", fm.type_code)).or_insert(0);
                *c = *c % 99 + 1;
                *c
            };
            let file = OutFile {
                name: make_name(&fm.name_kind[..4], &fm.name_kind[4..], cycle),
                rec_len: fm.rec_len,
                rec_id: 0,
                code: Code::Jis,
                data,
                compress: false,
            };
            let result = connect(&addr, opts)
                .map_err(Into::into)
                .and_then(|s| send_file(s, &net, &clock, &file, opts));
            let mut s = lock(state);
            match result {
                Ok(()) => {
                    for it in &sent_items {
                        if let Some(p) = s.outq.iter().position(|x| x == it) {
                            s.outq.remove(p);
                        }
                    }
                    s.save();
                    s.log(&format!("全銀ネットへ種別{} を {} 件送信", fm.type_code, sent_items.len()));
                    sent += 1;
                }
                Err(e) => {
                    s.log(&format!("全銀ネットへの送信に失敗 (後で再試行): {e}"));
                    return sent;
                }
            }
        }
    }
    sent
}

/// 銀行ノードを起動する。`interval` ごとに組戻依頼を取り込み、送信待ちを全銀ネットへ送る。
pub fn start(state: BankState, listen: &str, opts: Opts, interval: Duration) -> io::Result<(Server, Arc<Mutex<BankState>>)> {
    let st = Arc::new(Mutex::new(state));
    let mut srv = Server::start(listen, st.clone(), opts.clone(), WHO)?;
    let st2 = st.clone();
    srv.every(interval, move || {
        lock(&st2).take_requests();
        flush_to_net(&st2, &opts);
    });
    Ok((srv, st))
}
