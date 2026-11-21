//! 実際のバイナリを起動して、企業 → 銀行1 → 全銀ネット → 銀行2 → 企業 の流れを確かめる。

use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

fn zengin(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_zengin")).args(args).output().unwrap()
}

fn text(o: &Output) -> String {
    format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr))
}

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port()
}

/// 起動したプロセスを、終わるときに止める。
struct Procs(Vec<Child>);

impl Drop for Procs {
    fn drop(&mut self) {
        for c in &mut self.0 {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

struct Tmp(PathBuf);

impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn write(dir: &Path, name: &str, body: &str) -> String {
    let p = dir.join(name);
    std::fs::write(&p, body).unwrap();
    p.to_string_lossy().into_owned()
}

fn wait_port(port: u16) {
    let t = Instant::now();
    while t.elapsed() < Duration::from_secs(10) {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("ポート {port} が開かない");
}

#[test]
fn usage_and_errors_have_exit_codes() {
    assert_eq!(zengin(&[]).status.code(), Some(2));
    assert_eq!(zengin(&["unknown"]).status.code(), Some(2));
    let o = zengin(&["send", "--conf", "/nonexistent", "--file", "/nonexistent"]);
    assert_eq!(o.status.code(), Some(1));
    assert!(text(&o).contains("エラー"));
    let o = zengin(&["gen", "sogo", "--out", "/nonexistent/x", "--client-name", "ｱ", "--item", "bad"]);
    assert_eq!(o.status.code(), Some(1));
}

#[test]
fn gen_writes_a_valid_file() {
    let tmp = Tmp(std::env::temp_dir().join(format!("zengin-cli-gen-{}", std::process::id())));
    std::fs::create_dir_all(&tmp.0).unwrap();
    let out = tmp.0.join("s.dat");
    let o = zengin(&[
        "gen",
        "kyuyo",
        "--out",
        out.to_str().unwrap(),
        "--client-code",
        "7",
        "--client-name",
        "ｶ)ﾃｽﾄ",
        "--bank",
        "1",
        "--item",
        "2:20:1:5:ﾔﾏﾀﾞ:100",
        "--code",
        "ebcdic",
    ]);
    assert!(o.status.success(), "{}", text(&o));
    let bytes = std::fs::read(&out).unwrap();
    assert_eq!(bytes.len(), 120 * 4);
    let p = zengin::record::fmt(zengin::record::FileKind::Kyuyo).parse(&bytes).unwrap();
    assert_eq!(p.code, zengin::charcode::Code::Ebcdic);
    assert_eq!(p.trailer.num("amount"), 100);
}

#[test]
fn end_to_end_with_real_processes() {
    let tmp = Tmp(std::env::temp_dir().join(format!("zengin-cli-e2e-{}", std::process::id())));
    std::fs::create_dir_all(&tmp.0).unwrap();
    let d = &tmp.0;
    let (pn, p1, p2) = (free_port(), free_port(), free_port());
    let net = write(
        d,
        "net.conf",
        &format!(
            "center = 00000000000001\nlisten = 127.0.0.1:{pn}\n\
             [bank 0001]\ncenter = 00000000010001\npassword = NETPW1\nkey = NETKEY\naddr = 127.0.0.1:{p1}\nlimit = 5000000\n\
             [bank 0002]\ncenter = 00000000020001\npassword = NETPW2\nkey = NETKEY\naddr = 127.0.0.1:{p2}\nlimit = 5000000\n"
        ),
    );
    let bank = |no: u8, port: u16, net_pw: &str, comp: &str, ccode: &str, code: u8, branch: u8, balance: u32| {
        format!(
            "bank = {no}\nname = ｷﾞﾝｺｳ\ncenter = 000000000{no}0001\nlisten = 127.0.0.1:{port}\n\
             net_center = 00000000000001\nnet_password = {net_pw}\nnet_key = NETKEY\nnet_addr = 127.0.0.1:{pn}\n\
             [company {comp}]\ncenter = {ccode}\npassword = CPW001\nkey = CKEY01\ncode = {code}\nbranch = {branch}\nkind = 1\naccount = 1\nbalance = {balance}\n"
        )
    };
    let b1 = write(d, "b1.conf", &bank(1, p1, "NETPW1", "ACME", "00000001010001", 1, 10, 1_000_000));
    let b2 = write(d, "b2.conf", &bank(2, p2, "NETPW2", "BETA", "00000002010001", 2, 20, 0));
    let ca = write(
        d,
        "ca.conf",
        &format!("center = 00000001010001\nbank_center = 00000000010001\npassword = CPW001\nkey = CKEY01\naddr = 127.0.0.1:{p1}\n"),
    );
    let cb = write(
        d,
        "cb.conf",
        &format!("center = 00000002010001\nbank_center = 00000000020001\npassword = CPW001\nkey = CKEY01\naddr = 127.0.0.1:{p2}\n"),
    );
    let data = |n: &str| d.join(n).to_string_lossy().into_owned();

    let spawn = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_zengin"))
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap()
    };
    let procs = Procs(vec![
        spawn(&["net", "serve", "--conf", &net, "--data", &data("dn"), "--interval", "0.2"]),
        spawn(&["bank", "serve", "--conf", &b1, "--data", &data("d1"), "--interval", "0.2"]),
        spawn(&["bank", "serve", "--conf", &b2, "--data", &data("d2"), "--interval", "0.2"]),
    ]);
    for p in [pn, p1, p2] {
        wait_port(p);
    }

    let sogo = data("sogo.dat");
    let o = zengin(&[
        "gen",
        "sogo",
        "--out",
        &sogo,
        "--client-code",
        "1",
        "--client-name",
        "ｶ)ｱｸﾐ",
        "--bank",
        "1",
        "--item",
        "2:20:1:1:ﾍﾞﾀ:12000",
        "--item",
        "2:20:1:999:ｱﾅｼ:100",
    ]);
    assert!(o.status.success(), "{}", text(&o));
    let o = zengin(&["send", "--conf", &ca, "--file", &sogo, "--compress"]);
    assert!(o.status.success(), "{}", text(&o));

    // 銀行2の企業が振込入金通知を取得できるまで待つ
    let t = Instant::now();
    let nyukin = data("nyukin.dat");
    let out = loop {
        let o = zengin(&["fetch", "--conf", &cb, "--out", &nyukin]);
        assert!(o.status.success(), "{}", text(&o));
        if text(&o).contains("12000") {
            break text(&o);
        }
        assert!(t.elapsed() < Duration::from_secs(15), "入金通知が届かない: {}", text(&o));
        std::thread::sleep(Duration::from_millis(200));
    };
    assert!(out.contains("合計 1 件、12000 円"), "{out}");
    assert!(text(&zengin(&["fetch", "--conf", &cb, "--out", &nyukin])).contains("ない"));

    // 口座なしの 100 円は返却されて、銀行1の口座は 12000 円だけ減る
    let t = Instant::now();
    loop {
        let s = text(&zengin(&["bank", "statement", "--conf", &b1, "--data", &data("d1")]));
        if s.contains("988000") && s.contains("returned") {
            break;
        }
        assert!(t.elapsed() < Duration::from_secs(15), "返却が反映されない:\n{s}");
        std::thread::sleep(Duration::from_millis(200));
    }

    let report = text(&zengin(&["net", "report", "--conf", &net, "--data", &data("dn")]));
    assert!(report.contains("0001") && report.contains("0002"), "{report}");
    let settle = text(&zengin(&["net", "settle", "--data", &data("dn")]));
    assert!(settle.contains("差引の合計: 0"), "{settle}");
    assert!(settle.contains("12,000"), "{settle}");
    drop(procs);
}
