//! 全銀ネット1つと銀行2つ、企業2つを、1プロセスの中でローカルのポートに立てて、振込の流れを確かめる。

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use zengin::bank::{self, AcctKey, BankState, Company};
use zengin::charcode::Code;
use zengin::control::{CenterCode, make_name};
use zengin::exchange::*;
use zengin::handler::PeerCfg;
use zengin::net::{self, BankEntry, NetState, Settle};
use zengin::record::{FileKind, fmt};
use zengin::server::Server;
use zengin::session::{Opts, fetch_file, send_file, tune};
use zengin::store::Store;
use zengin::util::{Clock, DateTime};

fn cc(s: &str) -> CenterCode {
    CenterCode::parse(s).unwrap()
}

const NET_CODE: &str = "00000000000001";
const BANK1_CODE: &str = "00000000010001";
const BANK2_CODE: &str = "00000000020001";
const COMP_A_CODE: &str = "00000001010001";
const COMP_B_CODE: &str = "00000002010001";

const A_ACCT: (u16, u8, &str) = (10, 1, "0000001");
const B_ACCT: (u16, u8, &str) = (20, 1, "0000001");

fn key(a: (u16, u8, &str)) -> AcctKey {
    (a.0, a.1, a.2.to_string())
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

fn quick() -> Opts {
    Opts {
        timeout: Duration::from_secs(5),
        ..Opts::default()
    }
}

struct World {
    net: Arc<Mutex<NetState>>,
    net_srv: Server,
    bank1: Arc<Mutex<BankState>>,
    bank2: Option<Arc<Mutex<BankState>>>,
    srvs: Vec<Server>,
    bank1_addr: SocketAddr,
    bank2_addr: Option<SocketAddr>,
    opts: Opts,
}

fn bank_state(no: u16, code: &str, net_addr: &SocketAddr, company: Company, opening: i64) -> BankState {
    let own = cc(code);
    let net = PeerCfg::new("net", own, cc(NET_CODE), "NETPW1", "NETKEY").unwrap();
    let mut b = BankState::new(clock(), Store::new(None).unwrap(), no, "ﾃｽﾄｷﾞﾝｺｳ", own, net, &net_addr.to_string());
    b.add_company(company, opening);
    b
}

fn company(name: &str, code: u64, center: &str, bank_code: &str, acct: (u16, u8, &str)) -> Company {
    Company {
        name: name.into(),
        code,
        peer: PeerCfg::new(name, cc(bank_code), cc(center), "CPW001", "CKEY01").unwrap(),
        acct: key(acct),
    }
}

impl World {
    /// `bank2_up` が false なら、銀行2は後から起動する。
    fn new(limit1: u64, bank2_up: bool) -> World {
        let opts = quick();
        let own = cc(NET_CODE);
        let mk = |no: u16, code: &str, limit: u64| BankEntry {
            bank_no: no,
            name: format!("ｷﾞﾝｺｳ{no}"),
            addr: "127.0.0.1:1".into(),
            peer: PeerCfg::new(&format!("{no:04}"), own, cc(code), "NETPW1", "NETKEY").unwrap(),
            limit,
        };
        let state = NetState::new(
            clock(),
            Store::new(None).unwrap(),
            own,
            vec![mk(1, BANK1_CODE, limit1), mk(2, BANK2_CODE, 5_000_000)],
        )
        .unwrap();
        let (net_srv, net) = net::start(state, "127.0.0.1:0", opts.clone(), Duration::from_millis(50)).unwrap();
        let mut w = World {
            net,
            net_srv,
            bank1: Arc::new(Mutex::new(dummy())),
            bank2: None,
            srvs: vec![],
            bank1_addr: net_srv_addr(),
            bank2_addr: None,
            opts,
        };
        let net_addr = w.net_srv.addr;
        let b1 = bank_state(
            1,
            BANK1_CODE,
            &net_addr,
            company("A", 1, COMP_A_CODE, BANK1_CODE, A_ACCT),
            10_000_000_000,
        );
        let (s1, b1) = bank::start(b1, "127.0.0.1:0", w.opts.clone(), Duration::from_millis(50)).unwrap();
        w.bank1_addr = s1.addr;
        w.bank1 = b1;
        w.net.lock().unwrap().banks.get_mut(&1).unwrap().addr = w.bank1_addr.to_string();
        w.srvs.push(s1);
        if bank2_up {
            w.start_bank2();
        }
        w
    }

    fn start_bank2(&mut self) {
        let b2 = bank_state(
            2,
            BANK2_CODE,
            &self.net_srv.addr,
            company("B", 2, COMP_B_CODE, BANK2_CODE, B_ACCT),
            0,
        );
        let (s2, b2) = bank::start(b2, "127.0.0.1:0", self.opts.clone(), Duration::from_millis(50)).unwrap();
        self.bank2_addr = Some(s2.addr);
        self.net.lock().unwrap().banks.get_mut(&2).unwrap().addr = s2.addr.to_string();
        self.bank2 = Some(b2);
        self.srvs.push(s2);
    }

    fn b2(&self) -> &Arc<Mutex<BankState>> {
        self.bank2.as_ref().unwrap()
    }

    fn balance(&self, bank: &Arc<Mutex<BankState>>, acct: (u16, u8, &str)) -> i64 {
        bank.lock().unwrap().balance(&key(acct)).unwrap()
    }

    /// 企業A (銀行1) の総合振込ファイル。items は (被仕向銀行, 支店, 口座番号, 金額)。
    fn sogo_bytes(items: &[(u16, u16, &str, u64)], recipient: &str) -> Vec<u8> {
        let f = fmt(FileKind::Sogo);
        let h = f
            .new_header()
            .set_num("client_code", 1)
            .set("client_name", "ｶ)ｴｲ")
            .set("date", "1121")
            .set_num("bank_no", 1);
        let data: Vec<_> = items
            .iter()
            .map(|&(bank, branch, no, amt)| {
                f.new_data()
                    .set_num("bank_no", bank as u64)
                    .set_num("branch_no", branch as u64)
                    .set("acct_kind", "1")
                    .set("acct_no", no)
                    .set("name", recipient)
                    .set_num("amount", amt)
            })
            .collect();
        f.build(Code::Jis, h, &data).unwrap()
    }

    fn try_send_bytes(&self, bytes: Vec<u8>, cycle: u8) -> Result<(), zengin::util::ZError> {
        let file = zengin::handler::OutFile {
            name: make_name("5020", "0121", cycle),
            rec_len: 120,
            rec_id: 0,
            code: Code::Jis,
            data: bytes,
            compress: false,
        };
        let cfg = PeerCfg::new("bank", cc(COMP_A_CODE), cc(BANK1_CODE), "CPW001", "CKEY01").unwrap();
        let s = std::net::TcpStream::connect(self.bank1_addr).unwrap();
        tune(&s, &self.opts).unwrap();
        send_file(s, &cfg, &clock(), &file, &self.opts)
    }

    /// 企業A (銀行1) が総合振込を送る。
    fn send_sogo(&self, items: &[(u16, u16, &str, u64)]) {
        self.try_send_bytes(World::sogo_bytes(items, "ｳｹﾄﾘ"), 1).unwrap();
    }

    /// 企業B (銀行2) が振込入金通知を取得する。
    fn fetch_b(&self) -> Option<Vec<(u64, String, bool)>> {
        let cfg = PeerCfg::new("bank", cc(COMP_B_CODE), cc(BANK2_CODE), "CPW001", "CKEY01").unwrap();
        let s = std::net::TcpStream::connect(self.bank2_addr.unwrap()).unwrap();
        tune(&s, &self.opts).unwrap();
        let f = fetch_file(s, &cfg, &clock(), make_name("5020", "0001", 1), 200, false, &self.opts).unwrap()?;
        let p = fmt(FileKind::Nyukin).parse(&f.data).unwrap();
        assert_eq!(p.header.num("bank_no"), 2);
        assert_eq!(p.header.get("acct_no"), "0000001");
        assert_eq!(p.trailer.num("count") + p.trailer.num("cancel_count"), p.data.len() as u64);
        Some(
            p.data
                .iter()
                .map(|d| (d.num("amount"), d.get("client_name").to_string(), d.num("cancel") == 1))
                .collect(),
        )
    }

    fn stop(self) {
        self.net_srv.stop();
        for s in self.srvs {
            s.stop();
        }
    }
}

fn dummy() -> BankState {
    let own = cc(BANK1_CODE);
    BankState::new(
        clock(),
        Store::new(None).unwrap(),
        0,
        "",
        own,
        PeerCfg::new("net", own, cc(NET_CODE), "X", "X").unwrap(),
        "",
    )
}

fn net_srv_addr() -> SocketAddr {
    "127.0.0.1:1".parse().unwrap()
}

fn wait(what: &str, mut cond: impl FnMut() -> bool) {
    let t = Instant::now();
    while t.elapsed() < Duration::from_secs(10) {
        if cond() {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("時間内に成立しなかった: {what}");
}

fn idle(w: &World) -> bool {
    let n = w.net.lock().unwrap();
    let b1 = w.bank1.lock().unwrap().queued();
    let b2 = w.bank2.as_ref().map_or(0, |b| b.lock().unwrap().queued());
    b1 == 0 && b2 == 0 && n.queued(1) == 0 && n.queued(2) == 0
}

#[test]
fn normal_transfer_return_and_notice() {
    let w = World::new(5_000_000, true);
    // B 宛 1000 円、存在しない口座宛 300 円、存在しない銀行宛 200 円、自行内の存在しない口座宛 50 円
    w.send_sogo(&[
        (2, 20, "0000001", 1000),
        (2, 20, "0000999", 300),
        (9, 1, "0000001", 200),
        (1, 10, "0000777", 50),
    ]);
    wait("送金と返却が済む", || {
        idle(&w) && w.balance(&w.bank1, A_ACCT) == 10_000_000_000 - 1000 && w.balance(w.b2(), B_ACCT) == 1000
    });
    // 口座なし (300) と銀行なし (200) は、元の口座に戻る。自行内の口座なし (50) もすぐ戻る
    assert_eq!(w.balance(&w.bank1, A_ACCT), 10_000_000_000 - 1000);
    // 振込入金通知: 1回目は1件、2回目はファイルなし
    let n = w.fetch_b().expect("入金通知がある");
    assert_eq!(n, vec![(1000, "ｶ)ｴｲ".to_string(), false)]);
    assert!(w.fetch_b().is_none());
    // 全銀ネットの仕訳: 受理した振込 1 件 + 返却 1 件 (口座なし)。銀行なしは受理しない
    let net = w.net.lock().unwrap();
    let kinds: Vec<u8> = net.ledger.iter().map(|e| e.ty).collect();
    assert_eq!(kinds, [91, 91, 92]);
    drop(net);
    w.stop();
}

#[test]
fn limit_is_enforced_and_cleared_by_settlement() {
    let w = World::new(1000, true);
    w.send_sogo(&[(2, 20, "0000001", 600), (2, 20, "0000001", 600)]);
    wait("限度額超過の返却が済む", || {
        idle(&w) && w.balance(&w.bank1, A_ACCT) == 10_000_000_000 - 600
    });
    assert_eq!(w.balance(w.b2(), B_ACCT), 600);
    assert_eq!(w.net.lock().unwrap().exposure(1), 600);
    // 清算で仕向超過額が 0 に戻り、同じ額がまた通る
    let s = w.net.lock().unwrap().settle().unwrap();
    assert_eq!(s.rows.iter().map(|r| r.net).sum::<i64>(), 0);
    let r1 = s.rows.iter().find(|r| r.bank_no == 1).unwrap();
    assert_eq!((r1.paid, r1.received, r1.net), (600, 0, -600));
    assert_eq!(w.net.lock().unwrap().exposure(1), 0);
    w.send_sogo(&[(2, 20, "0000001", 600)]);
    wait("清算後に送金できる", || idle(&w) && w.balance(w.b2(), B_ACCT) == 1200);
    w.stop();
}

#[test]
fn rtgs_is_outside_limit_and_netting() {
    let w = World::new(1000, true);
    w.send_sogo(&[(2, 20, "0000001", 150_000_000)]);
    wait("大口が届く", || idle(&w) && w.balance(w.b2(), B_ACCT) == 150_000_000);
    let mut net = w.net.lock().unwrap();
    assert_eq!(net.exposure(1), 0);
    assert_eq!(net.ledger[0].settle, Settle::Rtgs);
    let s = net.settle().unwrap();
    assert_eq!(s.rtgs.len(), 1);
    assert!(s.rows.iter().all(|r| r.paid == 0 && r.received == 0));
    assert!(s.text.contains("即時決済 (1億円以上) 1 件"));
    drop(net);
    w.stop();
}

#[test]
fn recall_accepted_and_refused() {
    let w = World::new(5_000_000, true);
    w.send_sogo(&[(2, 20, "0000001", 700), (2, 20, "0000001", 800)]);
    wait("届く", || idle(&w) && w.balance(w.b2(), B_ACCT) == 1500);
    let handlings: Vec<u64> = w.bank1.lock().unwrap().sent.keys().copied().collect();
    assert_eq!(handlings.len(), 2);
    // 1件目: 承諾される
    w.bank1.lock().unwrap().request_recall(handlings[0]).unwrap();
    wait("組戻しが済む", || {
        idle(&w) && w.balance(w.b2(), B_ACCT) == 800 && w.balance(&w.bank1, A_ACCT) == 10_000_000_000 - 800
    });
    assert_eq!(w.bank1.lock().unwrap().sent[&handlings[0]].status, "returned");
    // 2件目: 被仕向銀行で資金が引き出された後 → 組戻不能
    w.b2().lock().unwrap().accounts.get_mut(&key(B_ACCT)).unwrap().balance = 0;
    w.bank1.lock().unwrap().request_recall(handlings[1]).unwrap();
    wait("組戻不能が届く", || {
        idle(&w) && w.bank1.lock().unwrap().sent[&handlings[1]].status == "recall-refused"
    });
    assert_eq!(w.balance(&w.bank1, A_ACCT), 10_000_000_000 - 800);
    // 同じ振込を二度は依頼できない
    assert!(w.bank1.lock().unwrap().request_recall(handlings[1]).is_err());
    // 存在しない取扱番号
    assert!(w.bank1.lock().unwrap().request_recall(1).is_err());
    w.stop();
}

#[test]
fn delivery_is_retried_until_destination_is_up() {
    let mut w = World::new(5_000_000, false);
    w.send_sogo(&[(2, 20, "0000001", 500)]);
    wait("全銀ネットが受理して配信待ちにする", || {
        w.net.lock().unwrap().queued(2) == 1
    });
    std::thread::sleep(Duration::from_millis(300)); // 接続できない状態で何度か再試行される
    assert_eq!(w.net.lock().unwrap().queued(2), 1);
    w.start_bank2();
    wait("起動後に届く", || idle(&w) && w.balance(w.b2(), B_ACCT) == 500);
    w.stop();
}

#[test]
fn duplicates_do_not_double_credit() {
    let w = World::new(5_000_000, true);
    let t = Transfer {
        handling_no: 1121000001,
        src_bank: 1,
        src_branch: 10,
        dst_bank: 2,
        dst_branch: 20,
        acct_kind: 1,
        acct_no: "0000001".into(),
        name: "ｳｹﾄﾘ".into(),
        amount: 400,
        client_code: 1,
        client_name: "ｶ)ｴｲ".into(),
        edi: String::new(),
    };
    // 全銀ネットに同じ振込を2回渡しても、仕訳と配信は1件
    for _ in 0..2 {
        w.net.lock().unwrap().accept(1, vec![Item::Transfer(t.clone())]);
    }
    assert_eq!(w.net.lock().unwrap().ledger.len(), 1);
    wait("届く", || idle(&w) && w.balance(w.b2(), B_ACCT) == 400);
    // 銀行2に同じ為替通知がもう一度届いても、二重に入金しない (全銀ネットを介さず直接渡す)
    use zengin::handler::{Handler, InFile};
    let data = build_file(FileKind::Ex91, 0, 2, &[Item::Transfer(t)], &clock().now(), Code::Jis).unwrap();
    let f = InFile {
        name: make_name("9020", "9091", 9),
        rec_len: 200,
        rec_id: 0,
        data,
    };
    let peer = PeerCfg::new("net", cc(BANK2_CODE), cc(NET_CODE), "NETPW1", "NETKEY").unwrap();
    w.b2().lock().unwrap().commit_recv(&peer, f);
    assert_eq!(w.balance(w.b2(), B_ACCT), 400);
    w.stop();
}

#[test]
fn spoofed_origin_is_rejected() {
    let w = World::new(5_000_000, true);
    // 銀行2 が、銀行1 を仕向とする振込を送ってきた → 破棄 (返却 12 が銀行2 へ)
    let t = Transfer {
        handling_no: 1121000042,
        src_bank: 1,
        src_branch: 10,
        dst_bank: 2,
        dst_branch: 20,
        acct_kind: 1,
        acct_no: "0000001".into(),
        name: "X".into(),
        amount: 10,
        client_code: 1,
        client_name: "X".into(),
        edi: String::new(),
    };
    w.net.lock().unwrap().accept(2, vec![Item::Transfer(t)]);
    assert!(w.net.lock().unwrap().ledger.is_empty());
    wait("返却が処理される", || idle(&w));
    assert_eq!(w.balance(w.b2(), B_ACCT), 0);
    w.stop();
}

#[test]
fn same_company_file_resent_is_not_processed_twice() {
    let w = World::new(5_000_000, true);
    let bytes = World::sogo_bytes(&[(2, 20, "0000001", 900)], "ｳｹﾄﾘ");
    // 企業が閉局回答を受け取れずに、同じファイルを再送した場合に相当する
    w.try_send_bytes(bytes.clone(), 1).unwrap();
    w.try_send_bytes(bytes, 1).unwrap();
    wait("届く", || idle(&w) && w.balance(w.b2(), B_ACCT) == 900);
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(w.balance(&w.bank1, A_ACCT), 10_000_000_000 - 900);
    assert_eq!(w.balance(w.b2(), B_ACCT), 900);
    assert_eq!(w.bank1.lock().unwrap().sent.len(), 1);
    // サイクルを変えた別のファイルは、別の依頼として処理する
    w.try_send_bytes(World::sogo_bytes(&[(2, 20, "0000001", 900)], "ｳｹﾄﾘ"), 2).unwrap();
    wait("2回目が届く", || idle(&w) && w.balance(w.b2(), B_ACCT) == 1800);
    w.stop();
}

#[test]
fn recall_withdraws_unnotified_receipt_and_cancels_notified_one() {
    let w = World::new(5_000_000, true);
    w.send_sogo(&[(2, 20, "0000001", 700), (2, 20, "0000001", 300)]);
    wait("届く", || idle(&w) && w.balance(w.b2(), B_ACCT) == 1000);
    let handlings: Vec<u64> = w.bank1.lock().unwrap().sent.keys().copied().collect();
    // 1件目の組戻し: 企業はまだ通知を受けていない → 通知が取り下げられる
    w.bank1.lock().unwrap().request_recall(handlings[0]).unwrap();
    wait("組戻し1", || idle(&w) && w.balance(w.b2(), B_ACCT) == 300);
    assert_eq!(w.fetch_b().expect("通知がある"), vec![(300, "ｶ)ｴｲ".to_string(), false)]);
    // 2件目の組戻し: すでに通知した → 取消の通知が届く
    w.bank1.lock().unwrap().request_recall(handlings[1]).unwrap();
    wait("組戻し2", || idle(&w) && w.balance(w.b2(), B_ACCT) == 0);
    assert_eq!(w.fetch_b().expect("取消の通知がある"), vec![(300, "ｶ)ｴｲ".to_string(), true)]);
    assert!(w.fetch_b().is_none());
    w.stop();
}

#[test]
fn invalid_returns_and_recall_refusals_are_ignored() {
    use zengin::handler::{Handler, InFile};
    let w = World::new(5_000_000, true);
    w.send_sogo(&[(2, 20, "0000001", 500)]);
    wait("届く", || idle(&w) && w.balance(w.b2(), B_ACCT) == 500);
    let h = *w.bank1.lock().unwrap().sent.keys().next().unwrap();
    let net_peer = PeerCfg::new("net", cc(BANK1_CODE), cc(NET_CODE), "NETPW1", "NETKEY").unwrap();
    let feed = |kind: FileKind, item: Item, name: &str| {
        let data = build_file(kind, 0, 1, &[item], &clock().now(), Code::Jis).unwrap();
        w.bank1.lock().unwrap().commit_recv(
            &net_peer,
            InFile {
                name: make_name("9020", name, 7),
                rec_len: 200,
                rec_id: 0,
                data,
            },
        );
    };
    let before = w.balance(&w.bank1, A_ACCT);
    // 金額が元より大きい返却、相手の銀行が違う返却は無視する
    feed(
        FileKind::Ex92,
        Item::Return(Return {
            handling_no: h,
            src_bank: 1,
            dst_bank: 2,
            reason: 1,
            amount: 501,
        }),
        "9092",
    );
    feed(
        FileKind::Ex92,
        Item::Return(Return {
            handling_no: h,
            src_bank: 1,
            dst_bank: 9,
            reason: 1,
            amount: 500,
        }),
        "9092",
    );
    assert_eq!(w.balance(&w.bank1, A_ACCT), before);
    // 組戻依頼中でない振込への組戻不能は、状態を変えない
    feed(
        FileKind::Ex94,
        Item::RecallNg(RecallNg {
            handling_no: h,
            src_bank: 1,
            dst_bank: 2,
            reason: 1,
        }),
        "9094",
    );
    assert_eq!(w.bank1.lock().unwrap().sent[&h].status, "sent");
    // 正しい返却は1回だけ入金する
    feed(
        FileKind::Ex92,
        Item::Return(Return {
            handling_no: h,
            src_bank: 1,
            dst_bank: 2,
            reason: 1,
            amount: 500,
        }),
        "9092",
    );
    feed(
        FileKind::Ex92,
        Item::Return(Return {
            handling_no: h,
            src_bank: 1,
            dst_bank: 2,
            reason: 1,
            amount: 500,
        }),
        "9092",
    );
    assert_eq!(w.balance(&w.bank1, A_ACCT), before + 500);
    // 全銀ネットは、組戻依頼のない組戻不能を中継しない
    w.net.lock().unwrap().accept(
        2,
        vec![Item::RecallNg(RecallNg {
            handling_no: h,
            src_bank: 1,
            dst_bank: 2,
            reason: 1,
        })],
    );
    assert_eq!(w.net.lock().unwrap().queued(1), 0);
    w.stop();
}

#[test]
fn names_outside_the_allowed_characters_are_rejected() {
    let w = World::new(5_000_000, true);
    // "/" は使用文字表にはあるが、人名欄では使えない
    let bytes = World::sogo_bytes(&[(2, 20, "0000001", 100)], "A/B");
    match w.try_send_bytes(bytes, 1) {
        Err(zengin::util::ZError::Rejected(c, _)) => assert_eq!(c, 0x99),
        other => panic!("拒否されるはず: {other:?}"),
    }
    assert_eq!(w.balance(&w.bank1, A_ACCT), 10_000_000_000);
    w.stop();
}

#[test]
fn large_batch_is_split_into_several_files() {
    let w = World::new(u64::MAX / 4, true);
    // 5000 件を超える振込は、複数のファイルに分けて全銀ネットへ送られる
    let n = MAX_ITEMS_PER_FILE + 5;
    let items: Vec<(u16, u16, &str, u64)> = (0..n).map(|_| (2, 20, "0000001", 1)).collect();
    w.send_sogo(&items);
    wait("全件が届く", || idle(&w) && w.balance(w.b2(), B_ACCT) == n as i64);
    assert_eq!(w.net.lock().unwrap().ledger.len(), n);
    w.stop();
}

#[test]
fn duplicate_file_can_be_rejected_with_code_16() {
    // 設定 duplicate = reject のとき、受信済みと同じファイルは二重ファイル伝送 (16) で拒否する
    let w = World::new(5_000_000, true);
    w.bank1.lock().unwrap().reject_duplicates = true;
    let bytes = World::sogo_bytes(&[(2, 20, "0000001", 900)], "ｳｹﾄﾘ");
    w.try_send_bytes(bytes.clone(), 1).unwrap();
    match w.try_send_bytes(bytes, 1) {
        Err(zengin::util::ZError::Rejected(c, _)) => assert_eq!(c, 0x16),
        other => panic!("拒否されるはず: {other:?}"),
    }
    wait("届く", || idle(&w) && w.balance(w.b2(), B_ACCT) == 900);
    assert_eq!(w.bank1.lock().unwrap().sent.len(), 1);
    w.stop();
}
