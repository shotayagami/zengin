//! 同時接続と、大きさの限界。

use std::io::Read;
use std::net::{SocketAddr, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use zengin::bank::{self, BankState};
use zengin::charcode::Code;
use zengin::config::Conf;
use zengin::control::{CenterCode, FileCtl, START_REQ, make_name};
use zengin::exchange::*;
use zengin::handler::{Handler, InFile, OutFile, PeerCfg};
use zengin::record::{FileKind, fmt};
use zengin::server::Server;
use zengin::session::{Opts, fetch_file, send_file, tune};
use zengin::store::Store;
use zengin::util::{Clock, ZError};

fn cc(s: &str) -> CenterCode {
    CenterCode::parse(s).unwrap()
}

/// 何でも受け取る二次局 (業務の検査をしない)。
#[derive(Default)]
struct Sink {
    received: Vec<InFile>,
    /// 照会で渡すファイル
    outbox: Option<OutFile>,
}

fn server_cfg() -> PeerCfg {
    PeerCfg::new("client", cc("00000000010001"), cc("00000000020002"), "PASS01", "KEY001").unwrap()
}

fn client_cfg() -> PeerCfg {
    PeerCfg::new("server", cc("00000000020002"), cc("00000000010001"), "PASS01", "KEY001").unwrap()
}

impl Handler for Sink {
    fn find_peer(&mut self, _: &CenterCode, _: &CenterCode) -> Result<PeerCfg, u8> {
        Ok(server_cfg())
    }
    fn start_recv(&mut self, _: &PeerCfg, _: &FileCtl) -> Result<(), u8> {
        Ok(())
    }
    fn validate_recv(&mut self, _: &PeerCfg, _: &InFile) -> Result<(), u8> {
        Ok(())
    }
    fn commit_recv(&mut self, _: &PeerCfg, f: InFile) {
        self.received.push(f);
    }
    fn start_send(&mut self, _: &PeerCfg, _: &FileCtl) -> Result<Option<OutFile>, u8> {
        Ok(self.outbox.clone())
    }
    fn commit_send(&mut self, _: &PeerCfg, _: &OutFile) {}
}

fn connect(addr: SocketAddr, o: &Opts) -> TcpStream {
    let s = TcpStream::connect(addr).unwrap();
    tune(&s, o).unwrap();
    s
}

fn wait(what: &str, mut cond: impl FnMut() -> bool) {
    let t = Instant::now();
    while !cond() {
        assert!(t.elapsed() < Duration::from_secs(20), "待ちきれない: {what}");
        thread::sleep(Duration::from_millis(20));
    }
}

fn raw_file(data: Vec<u8>, rec_len: usize) -> OutFile {
    OutFile {
        name: make_name("5020", "0121", 1),
        rec_len,
        rec_id: 0,
        code: Code::Jis,
        data,
        compress: false,
    }
}

// ---- 同時接続 ----

#[test]
fn connections_beyond_64_are_refused_and_slots_are_reused() {
    let sink = Arc::new(Mutex::new(Sink::default()));
    let opts = Opts {
        timeout: Duration::from_secs(60),
        ..Opts::default()
    };
    let srv = Server::start("127.0.0.1:0", sink.clone(), opts.clone(), "試験").unwrap();
    // 開局要求を送らずに居座る接続を 64 本
    let idle: Vec<TcpStream> = (0..64).map(|_| TcpStream::connect(srv.addr).unwrap()).collect();
    thread::sleep(Duration::from_millis(300)); // 受け付けが追いつくまで
    // 65 本目は、受け付けたあとすぐ切られる
    let mut extra = TcpStream::connect(srv.addr).unwrap();
    extra.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let mut buf = [0u8; 1];
    assert!(matches!(extra.read(&mut buf), Ok(0) | Err(_)), "切られる");
    // 1 本切れば、空いた枠で受け付ける
    drop(idle.into_iter().next());
    let file = raw_file(vec![b'A'; 120 * 3], 120);
    wait("空きができる", || {
        send_file(connect(srv.addr, &opts), &client_cfg(), &Clock::default(), &file, &opts).is_ok()
    });
    assert_eq!(sink.lock().unwrap().received.len(), 1);
    srv.stop();
}

const CONF: &str = "
bank = 1
name = ｻｸﾗ ｷﾞﾝｺｳ
center = 00000000010001
listen = 127.0.0.1:0
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
balance = 1000000

[account TANAKA]
name = ﾀﾅｶ
branch = 10
kind = 1
number = 2
balance = 0
";

#[test]
fn many_simultaneous_sessions_keep_the_books_balanced() {
    // 30 本の接続が同時に、振込を送ったり、入出金取引明細・残高通知を取ったりする
    let store = Store::new(None).unwrap();
    let state = BankState::from_conf(&Conf::parse(CONF).unwrap(), Clock::default(), store).unwrap();
    let opts = Opts::default();
    let (srv, st) = bank::start(state, "127.0.0.1:0", opts.clone(), Duration::from_secs(3600)).unwrap();
    let addr = srv.addr;
    let company = PeerCfg::new("bank", cc("00000001010001"), cc("00000000010001"), "CPW001", "CKEY01").unwrap();
    let f = fmt(FileKind::Sogo);
    let workers: Vec<_> = (0..30u64)
        .map(|i| {
            let (company, opts) = (company.clone(), opts.clone());
            thread::spawn(move || {
                if i % 3 == 0 {
                    // 照会 (入出金取引明細・残高通知) も、振込の受信と同時に行う
                    for kind in ["0003", "0004"] {
                        let fm = zengin::record::kind_from_name_mode(&format!("5020{kind}"), true).unwrap();
                        let name = make_name("5020", kind, 1);
                        fetch_file(connect(addr, &opts), &company, &Clock::default(), name, fm.rec_len, false, &opts).unwrap();
                    }
                    return 0;
                }
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
                    .set_num("amount", 100 + i);
                let file = OutFile {
                    name: make_name("5020", "0121", (i % 99) as u8 + 1),
                    rec_len: 120,
                    rec_id: 0,
                    code: Code::Jis,
                    data: f.build(Code::Jis, h, &[d]).unwrap(),
                    compress: i % 2 == 0,
                };
                send_file(connect(addr, &opts), &company, &Clock::default(), &file, &opts).unwrap();
                100 + i
            })
        })
        .collect();
    let sent: u64 = workers.into_iter().map(|w| w.join().unwrap()).sum();
    let b = st.lock().unwrap();
    let (acme, tanaka) = (
        b.balance(&(10, 1, "0000001".into())).unwrap(),
        b.balance(&(10, 1, "0000002".into())).unwrap(),
    );
    assert_eq!((acme, tanaka), (1_000_000 - sent as i64, sent as i64), "どの振込も1回ずつ");
    drop(b);
    srv.stop();
}

// ---- 大きさの限界 ----

/// テキスト 1 件に 3 レコード (1 バイトずつ) だけ入る送り方。テキストの件数を多くするため。
fn tiny_texts(mn: u8) -> Opts {
    Opts {
        mn,
        text_len: 8, // TTC 5 + 3 バイト
        ..Opts::default()
    }
}

#[test]
fn text_count_limit_is_65535() {
    let sink = Arc::new(Mutex::new(Sink::default()));
    let srv_opts = Opts { mn: 15, ..Opts::default() };
    let srv = Server::start("127.0.0.1:0", sink.clone(), srv_opts, "試験").unwrap();
    // ちょうど 65,535 件のテキスト (シーケンス番号の上限)。連続受信回数 15 どうしで
    let data: Vec<u8> = (0..65_535 * 3).map(|i| b'A' + (i % 26) as u8).collect();
    let opts = tiny_texts(15);
    send_file(
        connect(srv.addr, &opts),
        &client_cfg(),
        &Clock::default(),
        &raw_file(data.clone(), 1),
        &opts,
    )
    .unwrap();
    wait("受け取る", || sink.lock().unwrap().received.len() == 1);
    assert_eq!(sink.lock().unwrap().received[0].data, data);
    srv.stop();
    // 1 件でも超えたら、開局要求も送らずに止める
    let quiet = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let over = raw_file(vec![b'A'; 65_536 * 3], 1);
    match send_file(
        connect(quiet.local_addr().unwrap(), &opts),
        &client_cfg(),
        &Clock::default(),
        &over,
        &opts,
    ) {
        Err(ZError::Protocol(m)) => assert!(m.contains("65535"), "{m}"),
        other => panic!("止まるはず: {other:?}"),
    }
    let (mut peer, _) = quiet.accept().unwrap();
    let mut got = Vec::new();
    peer.read_to_end(&mut got).unwrap();
    assert!(got.is_empty(), "何も送らない: {} バイト届いた", got.len());
}

#[test]
fn oversized_inquiry_file_is_refused_before_the_start_response() {
    // 照会で二次局が渡すファイルが上限を超えるなら、開始回答 (その他エラー 99) で断り、データ電文は送らない
    let sink = Arc::new(Mutex::new(Sink {
        outbox: Some(raw_file(vec![b'A'; 65_536 * 3], 1)),
        ..Sink::default()
    }));
    let srv = Server::start("127.0.0.1:0", sink, tiny_texts(0), "試験").unwrap();
    let o = Opts::default();
    let name = make_name("5020", "0121", 1);
    match fetch_file(connect(srv.addr, &o), &client_cfg(), &Clock::default(), name, 1, false, &o) {
        Err(ZError::Rejected(c, _)) => assert_eq!(c, 0x99),
        other => panic!("断られるはず: {other:?}"),
    }
    srv.stop();
}

#[test]
fn notice_file_holds_at_most_5000_and_the_rest_comes_next_time() {
    let store = Store::new(None).unwrap();
    let mut b = BankState::from_conf(&Conf::parse(CONF).unwrap(), Clock::default(), store).unwrap();
    let net = b.net.clone();
    let items: Vec<Item> = (0..5001u64)
        .map(|i| {
            Item::Transfer(Transfer {
                handling_no: 1_000_000 + i,
                src_bank: 2,
                src_branch: 20,
                dst_bank: 1,
                dst_branch: 10,
                acct_kind: 1,
                acct_no: "0000001".into(),
                name: "ｱｸﾐ".into(),
                amount: 1,
                client_code: 9,
                client_name: "ﾍﾞﾀ".into(),
                edi: String::new(),
            })
        })
        .collect();
    let data = build_file(FileKind::Ex91, 0, 1, &items, &Clock::default().now(), Code::Jis).unwrap();
    let file = InFile {
        name: make_name("9020", "9091", 1),
        rec_len: 200,
        rec_id: 0,
        data,
    };
    b.validate_recv(&net, &file).unwrap();
    b.commit_recv(&net, file);
    assert_eq!(b.pending_notices("ACME"), 5001);
    let p = b.companies[0].peer.clone();
    let name = make_name("5020", "0001", 1);
    let mut counts = Vec::new();
    while let Some(out) = b.start_send(&p, &FileCtl::new(START_REQ, name, p.key, 200, false)).unwrap() {
        counts.push(fmt(FileKind::Nyukin).parse(&out.data).unwrap().data.len());
        b.commit_send(&p, &out);
    }
    assert_eq!(counts, [5000, 1]);
}
