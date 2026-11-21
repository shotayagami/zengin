//! 日付をまたぐ動作: ファイル名のサイクル、清算の回数、和暦、重複の記録の保持期間。

use std::path::PathBuf;
use zengin::bank::BankState;
use zengin::charcode::Code;
use zengin::config::Conf;
use zengin::control::{CenterCode, FileCtl, START_REQ, make_name, name_str};
use zengin::exchange::*;
use zengin::handler::{Handler, InFile, PeerCfg};
use zengin::net::{BankEntry, NetState};
use zengin::record::{FileKind, fmt};
use zengin::store::Store;
use zengin::util::{Clock, DateTime};

const CONF: &str = "
bank = 1
name = ｻｸﾗ ｷﾞﾝｺｳ
center = 00000000010001
net_center = 00000000000001
net_password = NETPW1
net_key = NETKEY
net_addr = 127.0.0.1:1

[company ACME]
center = 00000001010001
password = CPW001
key = CKEY01
code = 1
branch = 10
kind = 1
account = 1
balance = 100000

[account TANAKA]
name = ﾀﾅｶ
branch = 10
kind = 1
number = 2
balance = 5000
";

fn dt(y: u16, mo: u8, d: u8, h: u8, mi: u8) -> DateTime {
    DateTime { y, mo, d, h, mi, s: 0 }
}

struct Tmp(PathBuf);

impl Tmp {
    fn new(name: &str) -> Tmp {
        let p = std::env::temp_dir().join(format!("zengin-dates-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        Tmp(p)
    }
    fn store(&self) -> Store {
        Store::new(Some(self.0.clone())).unwrap()
    }
}

impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn bank(t: &Tmp, clock: &Clock) -> BankState {
    BankState::from_conf(&Conf::parse(CONF).unwrap(), clock.clone(), t.store()).unwrap()
}

/// 全銀ネットから、ACME の口座への振込が届いたことにする。
fn credit(b: &mut BankState, clock: &Clock, h: u64) {
    let t = Transfer {
        handling_no: h,
        src_bank: 2,
        src_branch: 20,
        dst_bank: 1,
        dst_branch: 10,
        acct_kind: 1,
        acct_no: "0000001".into(),
        name: "ｱｸﾐ".into(),
        amount: 100,
        client_code: 9,
        client_name: "ﾍﾞﾀ".into(),
        edi: String::new(),
    };
    let data = build_file(FileKind::Ex91, 0, 1, &[Item::Transfer(t)], &clock.now(), Code::Jis).unwrap();
    let file = InFile {
        name: make_name("9020", "9091", (h % 99) as u8 + 1),
        rec_len: 200,
        rec_id: 0,
        data,
    };
    let net = b.net.clone();
    b.validate_recv(&net, &file).unwrap();
    b.commit_recv(&net, file);
}

/// 振込入金通知を取りに来たものとして、銀行が付けたファイル名のサイクルを返す。
fn notice_cycle(b: &mut BankState) -> String {
    let p = b.companies[0].peer.clone();
    let name = make_name("5020", "0001", 1);
    let out = b
        .start_send(&p, &FileCtl::new(START_REQ, name, p.key, 200, false))
        .unwrap()
        .unwrap();
    b.commit_send(&p, &out);
    name_str(&out.name)[8..10].to_string()
}

#[test]
fn file_name_cycle_restarts_every_day() {
    // サイクルは「1日のうちに送った順に 01〜99」(docs/spec-net.md §3.1)
    let t = Tmp::new("cycle");
    let clock = Clock::fixed(dt(2026, 11, 21, 23, 0));
    let mut b = bank(&t, &clock);
    credit(&mut b, &clock, 1);
    assert_eq!(notice_cycle(&mut b), "01");
    credit(&mut b, &clock, 2);
    assert_eq!(notice_cycle(&mut b), "02");
    clock.set(dt(2026, 11, 22, 0, 30));
    credit(&mut b, &clock, 3);
    assert_eq!(notice_cycle(&mut b), "01", "日が変わったら 01 から");
    // 再起動しても、その日のサイクルの続きから
    credit(&mut b, &clock, 4);
    drop(b);
    let mut b = bank(&t, &clock);
    assert_eq!(notice_cycle(&mut b), "02");
}

fn net(t: &Tmp, clock: &Clock) -> NetState {
    let own = CenterCode::parse("00000000000001").unwrap();
    let entry = |no: u16, code: &str| BankEntry {
        bank_no: no,
        name: format!("B{no}"),
        addr: "127.0.0.1:1".into(),
        peer: PeerCfg::new(&format!("{no:04}"), own, CenterCode::parse(code).unwrap(), "PW0001", "KEY001").unwrap(),
        limit: 1_000_000,
    };
    NetState::new(
        clock.clone(),
        t.store(),
        own,
        vec![entry(1, "00000000010001"), entry(2, "00000000020001")],
    )
    .unwrap()
}

fn transfer(h: u64) -> Item {
    Item::Transfer(Transfer {
        handling_no: h,
        src_bank: 1,
        src_branch: 10,
        dst_bank: 2,
        dst_branch: 20,
        acct_kind: 1,
        acct_no: "0000001".into(),
        name: "ｳｹﾄﾘ".into(),
        amount: 100,
        client_code: 1,
        client_name: "ｶ)ｱｸﾐ".into(),
        edi: String::new(),
    })
}

#[test]
fn settlement_rounds_are_numbered_per_day() {
    let t = Tmp::new("settle");
    let clock = Clock::fixed(dt(2026, 11, 21, 15, 0));
    let mut n = net(&t, &clock);
    n.accept(1, vec![transfer(1)]);
    assert_eq!(n.settle().unwrap().no, 1);
    n.accept(1, vec![transfer(2)]);
    assert_eq!(n.settle().unwrap().no, 2);
    clock.set(dt(2026, 11, 22, 15, 0));
    drop(n);
    let mut n = net(&t, &clock); // 再起動をまたいでも、日付で数え直す
    n.accept(1, vec![transfer(3)]);
    let s = n.settle().unwrap();
    assert_eq!(s.no, 1, "日が変わったら第1回から");
    assert!(s.text.contains("2026-11-22 第1回"), "{}", s.text);
    for f in [
        "settlement-20261121-01.txt",
        "settlement-20261121-02.txt",
        "settlement-20261122-01.txt",
    ] {
        assert!(t.0.join(f).exists(), "{f}");
    }
}

#[test]
fn statement_spans_days_and_the_new_year_in_reiwa() {
    let t = Tmp::new("reiwa");
    let clock = Clock::fixed(dt(2026, 12, 31, 23, 50));
    let mut b = bank(&t, &clock);
    credit(&mut b, &clock, 1);
    clock.set(dt(2027, 1, 1, 0, 10));
    credit(&mut b, &clock, 2);
    let p = b.companies[0].peer.clone();
    let name = make_name("5020", "0003", 1);
    let out = b
        .start_send(&p, &FileCtl::new(START_REQ, name, p.key, 200, false))
        .unwrap()
        .unwrap();
    let m = fmt(FileKind::Meisai).parse(&out.data).unwrap();
    // 令和8年12月31日 〜 令和9年1月1日
    assert_eq!((m.header.get("from"), m.header.get("to")), ("081231", "090101"));
    let dates: Vec<&str> = m.data.iter().map(|d| d.get("acct_date")).collect();
    assert_eq!(dates, ["081231", "090101"]);
    // 振込入金通知の勘定日も、入金した日
    let name = make_name("5020", "0001", 1);
    let out = b
        .start_send(&p, &FileCtl::new(START_REQ, name, p.key, 200, false))
        .unwrap()
        .unwrap();
    let n = fmt(FileKind::Nyukin).parse(&out.data).unwrap();
    assert_eq!(n.data.iter().map(|d| d.get("acct_date")).collect::<Vec<_>>(), ["081231", "090101"]);
}

#[test]
fn duplicate_window_is_seven_days() {
    let t = Tmp::new("window");
    let clock = Clock::fixed(dt(2026, 11, 21, 10, 0));
    let mut b = bank(&t, &clock);
    let f = fmt(FileKind::Sogo);
    let h = f
        .new_header()
        .set_num("client_code", 1)
        .set("client_name", "ｶ)ｱｸﾐ")
        .set_num("bank_no", 1);
    let d = f
        .new_data()
        .set_num("bank_no", 1)
        .set_num("branch_no", 10)
        .set_num("acct_kind", 1)
        .set("acct_no", "0000002")
        .set("name", "ﾀﾅｶ")
        .set_num("amount", 10);
    let data = f.build(Code::Jis, h, &[d]).unwrap();
    let p = b.companies[0].peer.clone();
    let name = make_name("5020", "0121", 1);
    let send = |b: &mut BankState| {
        b.start_recv(&p, &FileCtl::new(START_REQ, name, p.key, 120, false)).unwrap();
        let file = InFile {
            name,
            rec_len: 120,
            rec_id: 0,
            data: data.clone(),
        };
        b.validate_recv(&p, &file).unwrap();
        b.commit_recv(&p, file);
    };
    let tanaka = (10, 1, "0000002".to_string());
    send(&mut b);
    clock.set(dt(2026, 11, 28, 23, 59)); // 7 日後: まだ重複
    send(&mut b);
    assert_eq!(b.balance(&tanaka), Some(5010));
    clock.set(dt(2026, 11, 29, 0, 0)); // 8 日後: 新しい依頼
    send(&mut b);
    assert_eq!(b.balance(&tanaka), Some(5020));
}
