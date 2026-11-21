//! 処理の途中でプロセスが落ちて、再起動した場合。
//!
//! 状態は処理ごとにいくつものファイルへ書く。`Store::crash_after(k)` で k 回目より後の書き込みを捨て、
//! 落ちる位置をすべて試す。落ちたあとは、再起動して、相手が同じものを送り直す (閉局回答を受け取れなかったため)。
//! どの位置で落ちても、二重に処理せず、取りこぼさないこと。

use std::path::PathBuf;
use zengin::bank::BankState;
use zengin::charcode::Code;
use zengin::config::Conf;
use zengin::control::{CenterCode, FileCtl, START_REQ, make_name};
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

fn clock() -> Clock {
    Clock::fixed(DateTime {
        y: 2026,
        mo: 11,
        d: 21,
        h: 10,
        mi: 0,
        s: 0,
    })
}

struct Tmp(PathBuf);

impl Tmp {
    fn new(name: &str) -> Tmp {
        let p = std::env::temp_dir().join(format!("zengin-crash-{}-{name}", std::process::id()));
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

fn bank(store: Store) -> BankState {
    BankState::from_conf(&Conf::parse(CONF).unwrap(), clock(), store).unwrap()
}

fn key(n: &str) -> (u16, u8, String) {
    (10, 1, n.to_string())
}

/// 企業から総合振込を受け取って確定する (閉局回答の前まで)。
fn receive_sogo(b: &mut BankState, items: &[(u64, &str, u64)]) {
    let f = fmt(FileKind::Sogo);
    let h = f
        .new_header()
        .set_num("client_code", 1)
        .set("client_name", "ｶ)ｱｸﾐ")
        .set_num("bank_no", 1);
    let data: Vec<_> = items
        .iter()
        .map(|(bank, acct, amount)| {
            f.new_data()
                .set_num("bank_no", *bank)
                .set_num("branch_no", 10)
                .set_num("acct_kind", 1)
                .set("acct_no", *acct)
                .set("name", "ﾀﾅｶ")
                .set_num("amount", *amount)
        })
        .collect();
    let name = make_name("5020", "0121", 1);
    let file = InFile {
        name,
        rec_len: 120,
        rec_id: 0,
        data: f.build(Code::Jis, h, &data).unwrap(),
    };
    let p = b.companies[0].peer.clone();
    b.start_recv(&p, &FileCtl::new(START_REQ, name, p.key, 120, false)).unwrap();
    b.validate_recv(&p, &file).unwrap();
    b.commit_recv(&p, file);
}

/// 落ちる位置 k = 0, 1, 2, … を、最後まで書けるまで試す。`run` は (状態のディレクトリ, 書き込みを k 回に制限した保存先) を受け取る。
fn every_crash_point(name: &str, mut run: impl FnMut(&Tmp, Store)) {
    for k in 0.. {
        let t = Tmp::new(&format!("{name}-{k}"));
        let limited = t.store().crash_after(k);
        let probe = limited.clone();
        run(&t, limited);
        if probe.budget_left().unwrap() > 0 {
            assert!(k > 0, "書き込みがない");
            return; // 落ちずに最後まで書けた
        }
        assert!(k < 200, "書き込みが多すぎる");
    }
}

#[test]
fn bank_receive_is_exactly_once_whatever_the_crash_point() {
    // 自行宛 (TANAKA へ 700) と他行宛 (銀行2 へ 300) を1ファイルで
    every_crash_point("bank", |t, limited| {
        // 落ちる前: 一部の状態ファイルだけが書かれる
        receive_sogo(&mut bank(limited), &[(1, "0000002", 700), (2, "0000001", 300)]);
        // 再起動して、企業が同じファイルを送り直す
        let mut b = bank(t.store());
        receive_sogo(&mut b, &[(1, "0000002", 700), (2, "0000001", 300)]);
        let (acme, tanaka) = (b.balance(&key("0000001")).unwrap(), b.balance(&key("0000002")).unwrap());
        assert_eq!((acme, tanaka, b.queued()), (99000, 5700, 1), "ちょうど1回処理する");
    });
}

fn net(store: Store) -> NetState {
    let own = CenterCode::parse("00000000000001").unwrap();
    let entry = |no: u16, code: &str| BankEntry {
        bank_no: no,
        name: format!("B{no}"),
        addr: "127.0.0.1:1".into(),
        peer: PeerCfg::new(&format!("{no:04}"), own, CenterCode::parse(code).unwrap(), "PW0001", "KEY001").unwrap(),
        limit: 1_000_000,
    };
    NetState::new(clock(), store, own, vec![entry(1, "00000000010001"), entry(2, "00000000020001")]).unwrap()
}

#[test]
fn net_accept_is_exactly_once_whatever_the_crash_point() {
    let transfer = Transfer {
        handling_no: 1121000001,
        src_bank: 1,
        src_branch: 10,
        dst_bank: 2,
        dst_branch: 20,
        acct_kind: 1,
        acct_no: "0000001".into(),
        name: "ｳｹﾄﾘ".into(),
        amount: 600,
        client_code: 1,
        client_name: "ｶ)ｱｸﾐ".into(),
        edi: String::new(),
    };
    every_crash_point("net", |t, limited| {
        net(limited).accept(1, vec![Item::Transfer(transfer.clone())]);
        let mut n = net(t.store());
        n.accept(1, vec![Item::Transfer(transfer.clone())]);
        assert_eq!((n.ledger.len(), n.queued(2), n.exposure(1)), (1, 1, 600), "ちょうど1回受理する");
    });
}

#[test]
fn old_per_file_state_is_read_and_migrated() {
    // スナップショットのない、以前の形式 (状態を名前ごとのファイルに分けて書いたもの) も読める
    let t = Tmp::new("migrate");
    std::fs::create_dir_all(&t.0).unwrap();
    std::fs::write(t.0.join("accounts.tsv"), "10\t1\t0000001\tACME\t1234\n").unwrap();
    let mut b = bank(t.store());
    assert_eq!(b.balance(&key("0000001")), Some(1234));
    // 次に保存すると、スナップショットに移る。古いファイルより、スナップショットが優先される
    receive_sogo(&mut b, &[(1, "0000002", 34)]);
    assert!(t.0.join("state.snapshot").exists());
    let b = bank(t.store());
    assert_eq!(b.balance(&key("0000001")), Some(1200));
}

#[test]
fn broken_snapshot_fails_loudly() {
    let t = Tmp::new("broken");
    std::fs::create_dir_all(&t.0).unwrap();
    std::fs::write(t.0.join("state.snapshot"), "\u{1}accounts.tsv\t5\n10\t1\t0000001\tACME\t1\n").unwrap();
    let e = BankState::from_conf(&Conf::parse(CONF).unwrap(), clock(), t.store())
        .err()
        .expect("途中で切れている");
    assert!(e.to_string().contains("state.snapshot"), "{e}");
}
