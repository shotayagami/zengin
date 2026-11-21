//! 状態ファイルへの保存と、再起動後の復元。

use std::path::PathBuf;
use zengin::bank::{BankState, Company};
use zengin::charcode::Code;
use zengin::control::{CenterCode, make_name};
use zengin::exchange::*;
use zengin::handler::{Handler, InFile, PeerCfg};
use zengin::net::{BankEntry, NetState};
use zengin::record::{FileKind, fmt};
use zengin::store::Store;
use zengin::util::{Clock, DateTime};

fn cc(s: &str) -> CenterCode {
    CenterCode::parse(s).unwrap()
}

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

/// 試験ごとの一時ディレクトリ。終わると消す。
struct Tmp(PathBuf);

impl Tmp {
    fn new(name: &str) -> Tmp {
        let p = std::env::temp_dir().join(format!("zengin-test-{}-{name}", std::process::id()));
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

fn transfer(h: u64, amount: u64) -> Transfer {
    Transfer {
        handling_no: h,
        src_bank: 1,
        src_branch: 10,
        dst_bank: 2,
        dst_branch: 20,
        acct_kind: 1,
        acct_no: "0000001".into(),
        name: "ｳｹﾄﾘ".into(),
        amount,
        client_code: 1,
        client_name: "ｶ)ｴｲ".into(),
        edi: "ﾒﾓ".into(),
    }
}

fn try_net_state(store: Store) -> zengin::util::Result<NetState> {
    let own = cc("00000000000001");
    let bank = |no: u16, code: &str, limit: u64| BankEntry {
        bank_no: no,
        name: format!("B{no}"),
        addr: "127.0.0.1:1".into(),
        peer: PeerCfg::new(&format!("{no:04}"), own, cc(code), "PW0001", "KEY001").unwrap(),
        limit,
    };
    NetState::new(
        clock(),
        store,
        own,
        vec![bank(1, "00000000010001", 1000), bank(2, "00000000020001", 1000)],
    )
}

fn net_state(store: Store) -> NetState {
    try_net_state(store).unwrap()
}

#[test]
fn net_state_survives_restart() {
    let tmp = Tmp::new("net");
    {
        let mut n = net_state(tmp.store());
        n.accept(
            1,
            vec![
                Item::Transfer(transfer(1121000001, 600)),
                Item::Transfer(transfer(1121000002, 300_000_000)),
            ],
        );
        assert_eq!(n.exposure(1), 600);
        assert_eq!(n.queued(2), 2);
    }
    let mut n = net_state(tmp.store());
    assert_eq!(n.ledger.len(), 2);
    assert_eq!(n.exposure(1), 600);
    assert_eq!(n.queued(2), 2);
    // 重複の記録も残っている
    n.accept(1, vec![Item::Transfer(transfer(1121000001, 600))]);
    assert_eq!(n.ledger.len(), 2);
    // 限度額の判定は、復元した仕訳をもとにする
    n.accept(1, vec![Item::Transfer(transfer(1121000003, 500))]);
    assert_eq!(n.ledger.len(), 2, "600 + 500 は限度額 1000 を超える");
    // 清算の状態も残る
    let s = n.settle().unwrap();
    assert_eq!(s.rtgs.len(), 1);
    drop(n);
    let n = net_state(tmp.store());
    assert_eq!(n.exposure(1), 0);
    assert!(n.ledger.iter().all(|e| e.settled));
    assert!(tmp.0.join("settlement-20261121-01.txt").exists());
    assert!(tmp.0.join("journal.tsv").exists());
}

fn bank_state(store: Store) -> BankState {
    let own = cc("00000000020001");
    let net = PeerCfg::new("net", own, cc("00000000000001"), "NETPW1", "NETKEY").unwrap();
    let mut b = BankState::new(clock(), store, 2, "ｳﾒ", own, net, "127.0.0.1:1");
    let peer = PeerCfg::new("B", own, cc("00000002010001"), "CPW001", "CKEY01").unwrap();
    b.add_company(
        Company {
            name: "B".into(),
            code: 2,
            peer,
            acct: (20, 1, "0000001".into()),
        },
        0,
    );
    b.load().unwrap();
    b
}

fn net_file(items: &[Item]) -> InFile {
    let data = build_file(FileKind::Ex91, 0, 2, items, &clock().now(), Code::Jis).unwrap();
    InFile {
        name: make_name("9020", "9091", 1),
        rec_len: 200,
        rec_id: 0,
        data,
    }
}

#[test]
fn bank_state_survives_restart() {
    let tmp = Tmp::new("bank");
    let net_peer = PeerCfg::new("net", cc("00000000020001"), cc("00000000000001"), "NETPW1", "NETKEY").unwrap();
    let acct = (20u16, 1u8, "0000001".to_string());
    {
        let mut b = bank_state(tmp.store());
        b.commit_recv(&net_peer, net_file(&[Item::Transfer(transfer(1121000001, 700))]));
        // 存在しない口座宛 → 返却が送信待ちに積まれる
        let mut t = transfer(1121000002, 50);
        t.acct_no = "0000999".into();
        b.commit_recv(&net_peer, net_file(&[Item::Transfer(t)]));
        assert_eq!(b.balance(&acct), Some(700));
        assert_eq!(b.queued(), 1);
        assert_eq!(b.pending_notices("B"), 1);
    }
    let mut b = bank_state(tmp.store());
    assert_eq!(b.balance(&acct), Some(700), "設定の初期残高 0 ではなく、状態ファイルの残高");
    assert_eq!(b.queued(), 1);
    assert_eq!(b.pending_notices("B"), 1);
    // 重複の記録が残っているので、二重に入金しない
    b.commit_recv(&net_peer, net_file(&[Item::Transfer(transfer(1121000001, 700))]));
    assert_eq!(b.balance(&acct), Some(700));
    // 振込入金通知を作って確定すると、通知済みになり、それも残る
    let peer = PeerCfg::new("B", cc("00000000020001"), cc("00000002010001"), "CPW001", "CKEY01").unwrap();
    let req = zengin::control::FileCtl::new(zengin::control::START_REQ, make_name("5020", "0001", 1), peer.key, 200, false);
    let out = b.start_send(&peer, &req).unwrap().expect("通知がある");
    let p = fmt(FileKind::Nyukin).parse(&out.data).unwrap();
    assert_eq!(p.data.len(), 1);
    b.commit_send(&peer, &out);
    drop(b);
    let mut b = bank_state(tmp.store());
    assert_eq!(b.pending_notices("B"), 0);
    assert!(b.start_send(&peer, &req).unwrap().is_none());
    // 再送要求なら、渡し終えたファイルをもう一度渡す (再起動をまたいでも残っている)。通知済みの件数は変わらない
    let mut again = req.clone();
    again.kind = zengin::control::RETRY_REQ;
    let resent = b.start_send(&peer, &again).unwrap().expect("最後に渡したファイル");
    assert_eq!(resent.data, out.data);
    b.commit_send(&peer, &resent);
    assert!(b.start_send(&peer, &req).unwrap().is_none());
}

#[test]
fn corrupt_or_unreadable_state_fails_loudly_instead_of_resetting() {
    // 壊れた行がある → 初期状態に戻さず、起動を失敗させる
    let tmp = Tmp::new("corrupt");
    let store = tmp.store();
    store.write("ledger.tsv", &["not	a	ledger".to_string()]).unwrap();
    assert!(try_net_state(tmp.store()).is_err());
    store.write("ledger.tsv", &[]).unwrap();
    store.write("outq.tsv", &["x".to_string()]).unwrap();
    assert!(try_net_state(tmp.store()).is_err());
    // UTF-8 でないファイル (読めない) も同じ
    std::fs::write(tmp.0.join("seen.tsv"), [0xFF, 0xFE, 0x00]).unwrap();
    store.write("outq.tsv", &[]).unwrap();
    assert!(try_net_state(tmp.store()).is_err());
    // 銀行も同じ
    let tmp = Tmp::new("corrupt-bank");
    tmp.store().write("accounts.tsv", &["1	2".to_string()]).unwrap();
    let own = cc("00000000020001");
    let net = PeerCfg::new("net", own, cc("00000000000001"), "NETPW1", "NETKEY").unwrap();
    let mut b = BankState::new(clock(), tmp.store(), 2, "ｳﾒ", own, net, "127.0.0.1:1");
    assert!(b.load().is_err());
}

#[test]
fn state_files_are_written_atomically_and_requests_are_not_lost() {
    let tmp = Tmp::new("take");
    let store = tmp.store();
    // 依頼を取り出したあとに追記された行は、次の取り出しで読める
    std::fs::write(
        tmp.0.join("req"),
        "1
",
    )
    .unwrap();
    assert_eq!(store.take("req").unwrap().unwrap(), ["1"]);
    std::fs::write(
        tmp.0.join("req"),
        "2
",
    )
    .unwrap();
    // まだ done していない (途中で落ちた) 場合は、続きから同じ内容を返す
    assert_eq!(store.take("req").unwrap().unwrap(), ["1"]);
    store.done("req");
    assert_eq!(store.take("req").unwrap().unwrap(), ["2"]);
    store.done("req");
    assert!(store.take("req").unwrap().is_none());
    // write は一時ファイルを残さない
    store.write("x.tsv", &["a".to_string()]).unwrap();
    assert!(!tmp.0.join("x.tsv.tmp").exists());
}

#[test]
fn settlement_includes_banks_removed_from_the_configuration() {
    let tmp = Tmp::new("removed");
    {
        let mut n = net_state(tmp.store());
        n.accept(1, vec![Item::Transfer(transfer(1121000001, 600))]);
    }
    // 銀行2を設定から外して再起動しても、仕訳にある銀行は清算に載り、差引の合計は 0 になる
    let own = cc("00000000000001");
    let bank1 = BankEntry {
        bank_no: 1,
        name: "B1".into(),
        addr: "127.0.0.1:1".into(),
        peer: PeerCfg::new("0001", own, cc("00000000010001"), "PW0001", "KEY001").unwrap(),
        limit: 1000,
    };
    let mut n = NetState::new(clock(), tmp.store(), own, vec![bank1]).unwrap();
    let s = n.settle().unwrap();
    assert_eq!(s.rows.len(), 2);
    assert!(s.rows.iter().any(|r| r.bank_no == 2 && r.name == "(未登録)" && r.net == 600));
    assert_eq!(s.rows.iter().map(|r| r.net).sum::<i64>(), 0);
}
