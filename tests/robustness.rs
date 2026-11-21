//! 不正な入力で panic やハングをしないこと、上限が効くこと。乱数は固定のシードで、結果は毎回同じ。

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use zengin::charcode::Code;
use zengin::config::Conf;
use zengin::control::{CenterCode, CommCtl, FileCtl, make_name};
use zengin::exchange::Item;
use zengin::handler::{Handler, InFile, OutFile, PeerCfg};
use zengin::record::{FileKind, fmt};
use zengin::session::{Opts, Timed, send_file, serve, tune};
use zengin::sublayer::{SH_LEN, Sh};
use zengin::text::{decompress_body, parse_data, parse_ttc};
use zengin::util::{Clock, ZError};

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.next() as u8).collect()
    }
}

#[test]
fn decoders_never_panic_on_random_bytes() {
    let mut r = Rng(0x9E37_79B9_7F4A_7C15);
    for i in 0..20_000 {
        let n = (r.next() % 300) as usize;
        let b = r.bytes(n);
        if b.len() >= SH_LEN {
            let _ = Sh::decode(b[..SH_LEN].try_into().unwrap());
        }
        let _ = parse_ttc(&b);
        let _ = parse_data(&b, i % 2 == 0);
        let _ = decompress_body(&b, (r.next() % 400) as usize);
        let _ = CommCtl::decode(&b);
        let _ = FileCtl::decode(&b);
        let _ = Item::from_line(&String::from_utf8_lossy(&b));
        let _ = Conf::parse(&String::from_utf8_lossy(&b));
    }
}

#[test]
fn record_parsers_never_panic() {
    let mut r = Rng(12345);
    for kind in [
        FileKind::Sogo,
        FileKind::Kyuyo,
        FileKind::Nyukin,
        FileKind::Ex91,
        FileKind::Ex92,
        FileKind::Ex93,
        FileKind::Ex94,
    ] {
        let f = fmt(kind);
        for _ in 0..500 {
            for first in [0x31u8, 0xF1, 0x00] {
                let k = 3 + (r.next() % 3) as usize;
                let mut b = r.bytes(f.rec_len * k);
                b[0] = first;
                let _ = f.parse(&b);
            }
        }
        // 正しいファイルを1バイトだけ壊しても、panic しない
        let h = f.new_header();
        let good = f.build(Code::Ebcdic, h, &[]).unwrap();
        for i in 0..good.len() {
            let mut b = good.clone();
            b[i] ^= 0xFF;
            let _ = f.parse(&b);
        }
    }
}

struct Sink {
    received: Vec<InFile>,
}

fn cc(s: &str) -> CenterCode {
    CenterCode::parse(s).unwrap()
}

fn server_cfg() -> PeerCfg {
    PeerCfg::new("c", cc("00000000010001"), cc("00000000020002"), "PASS01", "KEY001").unwrap()
}

impl Handler for Sink {
    fn find_peer(&mut self, remote: &CenterCode, own: &CenterCode) -> Result<PeerCfg, u8> {
        let c = server_cfg();
        if c.own != *own {
            Err(0x11)
        } else if c.remote != *remote {
            Err(0x12)
        } else {
            Ok(c)
        }
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
        Ok(None)
    }
    fn commit_send(&mut self, _: &PeerCfg, _: &OutFile) {}
}

fn short() -> Opts {
    Opts {
        timeout: Duration::from_millis(300),
        ..Opts::default()
    }
}

fn serve_once(opts: Opts) -> (std::net::SocketAddr, thread::JoinHandle<Result<(), ZError>>, Arc<Mutex<Sink>>) {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = l.local_addr().unwrap();
    let h = Arc::new(Mutex::new(Sink { received: vec![] }));
    let hh = h.clone();
    let t = thread::spawn(move || {
        let (s, _) = l.accept().unwrap();
        let s = Timed::new(s, opts.timeout, opts.max_session).unwrap();
        serve(s, &mut hh.clone(), &opts)
    });
    (addr, t, h)
}

#[test]
fn server_survives_garbage_connections() {
    let mut r = Rng(777);
    for _ in 0..30 {
        let opts = short();
        let (addr, t, h) = serve_once(opts);
        let mut s = TcpStream::connect(addr).unwrap();
        let n = (r.next() % 400) as usize;
        let mut b = r.bytes(n);
        if r.next() % 2 == 0 && b.len() > 8 {
            // SH だけは正しい形にして、中身を壊す
            let len = (b.len() as u16).to_be_bytes();
            b[..8].copy_from_slice(&[len[0], len[1], 0x10, 0x00, 0, 0, 0, 0]);
        }
        let _ = s.write_all(&b);
        let _ = s.flush();
        // 二次局は、エラーまたはタイムアウトで必ず終わる
        let res = t.join().expect("serve が panic しない");
        assert!(res.is_err());
        assert!(h.lock().unwrap().received.is_empty());
    }
}

#[test]
fn server_times_out_on_silent_client() {
    let (addr, t, _) = serve_once(short());
    let _s = TcpStream::connect(addr).unwrap(); // 何も送らない
    assert!(t.join().unwrap().is_err());
}

#[test]
fn oversize_file_is_cut_off() {
    let file = {
        let f = fmt(FileKind::Sogo);
        let data: Vec<_> = (0..500).map(|_| f.new_data().set_num("bank_no", 2).set_num("amount", 1)).collect();
        let bytes = f.build(Code::Jis, f.new_header().set_num("client_code", 1), &data).unwrap();
        OutFile {
            name: make_name("5020", "0121", 1),
            rec_len: 120,
            rec_id: 0,
            code: Code::Jis,
            data: bytes,
            compress: false,
        }
    };
    // 上限 10,000 バイト。ファイルは 60,000 バイト超
    let (addr, t, h) = serve_once(Opts {
        max_file: 10_000,
        ..short()
    });
    let s = TcpStream::connect(addr).unwrap();
    let opts = short();
    tune(&s, &opts).unwrap();
    let cfg = PeerCfg::new("s", cc("00000000020002"), cc("00000000010001"), "PASS01", "KEY001").unwrap();
    let r = send_file(s, &cfg, &Clock::default(), &file, &opts);
    assert!(r.is_err());
    assert!(t.join().unwrap().is_err());
    assert!(h.lock().unwrap().received.is_empty());
}

#[test]
fn oversize_sh_does_not_allocate_beyond_u16() {
    // メッセージ長 65535 を名乗って送り続けない相手でも、タイムアウトで終わる
    let (addr, t, _) = serve_once(short());
    let mut s = TcpStream::connect(addr).unwrap();
    s.write_all(&[0xFF, 0xFF, 0x10, 0, 0, 0, 0, 0, 1, 2, 3]).unwrap();
    assert!(t.join().unwrap().is_err());
    let mut buf = [0u8; 1];
    let _ = s.read(&mut buf);
}

#[test]
fn slow_drip_is_cut_by_the_session_deadline() {
    // 無通信監視タイマ (5秒) より短い間隔で1バイトずつ送り続けても、接続全体の期限 (0.5秒) で切れる
    let opts = Opts {
        timeout: Duration::from_secs(5),
        max_session: Duration::from_millis(500),
        ..Opts::default()
    };
    let (addr, t, _) = serve_once(opts);
    let mut s = TcpStream::connect(addr).unwrap();
    let start = std::time::Instant::now();
    let dripper = thread::spawn(move || {
        // 長さ 100 の情報メッセージを名乗り、中身を遅く送る
        let _ = s.write_all(&[0, 100, 0x10, 0, 0, 0, 0, 0]);
        for _ in 0..50 {
            if s.write_all(&[0]).is_err() {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
    });
    assert!(t.join().unwrap().is_err());
    assert!(start.elapsed() < Duration::from_secs(3), "期限で切れる");
    let _ = dripper.join();
}
