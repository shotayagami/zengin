use std::collections::HashMap;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, Instant};
use zengin::bank::{self, BankState};
use zengin::charcode::Code;
use zengin::config::Conf;
use zengin::control::{CenterCode, make_name};
use zengin::handler::{OutFile, PeerCfg};
use zengin::net::{self, NetState, connect};
use zengin::record::{FileKind, fmt};
use zengin::session::{Opts, fetch_file, send_file};
use zengin::store::Store;
use zengin::util::{Clock, Result, ZError, proto};

const USAGE: &str = "\
使い方:
  企業:
    zengin gen sogo|kyuyo --out FILE --client-code N --client-name NAME --bank N [--code jis|ebcdic]
                          --item 銀行:支店:種目:口座:受取人名:金額 [--item ...]
    zengin send  --conf FILE --file FILE [--kind sogo|kyuyo] [--to ADDR] [--cycle N] [--compress] [--mn N]
    zengin fetch --conf FILE --out FILE [--to ADDR] [--cycle N] [--resend] [--mn N]
  銀行ノード:
    zengin bank serve     --conf FILE --data DIR [--listen ADDR] [--interval 秒] [--mn N]
    zengin bank statement --conf FILE --data DIR
    zengin bank recall    --data DIR 取扱番号
  全銀ネット:
    zengin net serve  --conf FILE --data DIR [--listen ADDR] [--interval 秒] [--mn N]
    zengin net report --conf FILE --data DIR
    zengin net settle --data DIR [--offline --conf FILE]
";

struct Args {
    flags: HashMap<String, Vec<String>>,
    pos: Vec<String>,
}

impl Args {
    fn parse(v: &[String]) -> Result<Args> {
        const SWITCHES: [&str; 3] = ["compress", "offline", "resend"];
        let (mut flags, mut pos) = (HashMap::<String, Vec<String>>::new(), Vec::new());
        let mut it = v.iter();
        while let Some(a) = it.next() {
            if let Some(k) = a.strip_prefix("--") {
                let val = if SWITCHES.contains(&k) {
                    String::new()
                } else {
                    match it.next() {
                        Some(x) => x.clone(),
                        None => return proto(format!("--{k} に値がない")),
                    }
                };
                flags.entry(k.into()).or_default().push(val);
            } else {
                pos.push(a.clone());
            }
        }
        Ok(Args { flags, pos })
    }
    fn get(&self, k: &str) -> Option<&str> {
        self.flags.get(k).and_then(|v| v.last()).map(String::as_str)
    }
    fn req(&self, k: &str) -> Result<&str> {
        self.get(k).map_or_else(|| proto(format!("--{k} が必要")), Ok)
    }
    fn has(&self, k: &str) -> bool {
        self.flags.contains_key(k)
    }
    fn num<T: std::str::FromStr>(&self, k: &str, default: T) -> Result<T> {
        self.get(k)
            .map_or(Ok(default), |v| v.parse().or_else(|_| proto(format!("--{k} は数値: {v:?}"))))
    }
    fn opts(&self) -> Result<Opts> {
        Ok(Opts {
            mn: self.num("mn", 3u8)?.min(15),
            ..Opts::default()
        })
    }
    fn path(&self, k: &str) -> Result<PathBuf> {
        Ok(PathBuf::from(self.req(k)?))
    }
}

fn io<T>(r: std::io::Result<T>) -> Result<T> {
    r.map_err(ZError::Io)
}

/// 企業側の設定: `center`、`bank_center`、`password`、`key`、`addr`。
fn company_cfg(conf: &Conf) -> Result<PeerCfg> {
    let g = &conf.global;
    PeerCfg::new(
        "bank",
        CenterCode::parse(g.req("center")?)?,
        CenterCode::parse(g.req("bank_center")?)?,
        g.req("password")?,
        g.req("key")?,
    )
}

/// ファイル名のサイクル (1〜99)。
fn cycle(a: &Args) -> Result<u8> {
    let c = a.num("cycle", 1u8)?;
    if !(1..=99).contains(&c) {
        return proto("--cycle は 1〜99");
    }
    Ok(c)
}

fn kind_of(a: &Args) -> Result<FileKind> {
    match a.get("kind").unwrap_or("sogo") {
        "sogo" => Ok(FileKind::Sogo),
        "kyuyo" => Ok(FileKind::Kyuyo),
        k => proto(format!("--kind は sogo か kyuyo: {k}")),
    }
}

fn code_of(a: &Args) -> Result<Code> {
    match a.get("code").unwrap_or("jis") {
        "jis" => Ok(Code::Jis),
        "ebcdic" => Ok(Code::Ebcdic),
        c => proto(format!("--code は jis か ebcdic: {c}")),
    }
}

fn cmd_gen(kind: FileKind, a: &Args) -> Result<()> {
    let f = fmt(kind);
    let now = Clock::default().now();
    let h = f
        .new_header()
        .set_num("client_code", a.num("client-code", 0u64)?)
        .set("client_name", a.req("client-name")?)
        .set("date", now.mmdd())
        .set_num("bank_no", a.num("bank", 0u64)?);
    let mut data = Vec::new();
    for item in a.flags.get("item").map(Vec::as_slice).unwrap_or(&[]) {
        let p: Vec<&str> = item.split(':').collect();
        let [bank, branch, kind_, acct, name, amount] = p[..] else {
            return proto(format!("--item は 銀行:支店:種目:口座:受取人名:金額: {item}"));
        };
        let n = |s: &str| s.parse::<u64>().or_else(|_| proto(format!("数値でない: {s}")));
        data.push(
            f.new_data()
                .set_num("bank_no", n(bank)?)
                .set_num("branch_no", n(branch)?)
                .set_num("acct_kind", n(kind_)?)
                .set("acct_no", format!("{:07}", n(acct)?))
                .set("name", name)
                .set_num("amount", n(amount)?),
        );
    }
    let bytes = f.build(code_of(a)?, h, &data)?;
    io(std::fs::write(a.path("out")?, &bytes))?;
    println!("{} 件、{} バイト", data.len(), bytes.len());
    Ok(())
}

fn cmd_send(a: &Args) -> Result<()> {
    let conf = Conf::load(&a.path("conf")?)?;
    let cfg = company_cfg(&conf)?;
    let kind = kind_of(a)?;
    let fm = fmt(kind);
    let data = io(std::fs::read(a.path("file")?))?;
    let parsed = fm.parse(&data)?; // 送る前に自分で検査する
    let opts = a.opts()?;
    let addr = a
        .get("to")
        .or(conf.global.get("addr"))
        .map_or_else(|| proto("--to か設定の addr が必要"), Ok)?;
    let cycle = cycle(a)?;
    let file = OutFile {
        name: make_name(&fm.name_kind[..4], &fm.name_kind[4..], cycle),
        rec_len: fm.rec_len,
        code: parsed.code,
        data,
        compress: a.has("compress"),
    };
    send_file(io(connect(addr, &opts))?, &cfg, &Clock::default(), &file, &opts)?;
    println!("送信した: {} 件", parsed.data.len());
    Ok(())
}

fn cmd_fetch(a: &Args) -> Result<()> {
    let conf = Conf::load(&a.path("conf")?)?;
    let cfg = company_cfg(&conf)?;
    let fm = fmt(FileKind::Nyukin);
    let opts = a.opts()?;
    let addr = a
        .get("to")
        .or(conf.global.get("addr"))
        .map_or_else(|| proto("--to か設定の addr が必要"), Ok)?;
    let name = make_name(&fm.name_kind[..4], &fm.name_kind[4..], cycle(a)?);
    let Some(file) = fetch_file(
        io(connect(addr, &opts))?,
        &cfg,
        &Clock::default(),
        name,
        fm.rec_len,
        a.has("resend"),
        &opts,
    )?
    else {
        println!("振込入金通知はない");
        return Ok(());
    };
    let p = fm.parse(&file.data)?;
    io(std::fs::write(a.path("out")?, &file.data))?;
    println!(
        "口座 {}-{}-{} {}",
        p.header.get("branch_no"),
        p.header.get("acct_kind"),
        p.header.get("acct_no"),
        p.header.get("acct_name")
    );
    for d in &p.data {
        println!(
            "  {} 円  {}  (銀行 {})  {}",
            d.num("amount"),
            d.get("client_name"),
            d.get("src_bank"),
            d.get("edi")
        );
    }
    println!("合計 {} 件、{} 円", p.trailer.num("count"), p.trailer.num("amount"));
    Ok(())
}

fn serve_forever(server: zengin::server::Server, what: &str) -> Result<()> {
    eprintln!("{what}: {} で待ち受け中", server.addr);
    server.wait();
    Ok(())
}

fn interval(a: &Args) -> Result<Duration> {
    Ok(Duration::from_secs_f64(a.num("interval", 5.0f64)?))
}

fn cmd_bank(sub: &str, a: &Args) -> Result<()> {
    match sub {
        "serve" => {
            let conf = Conf::load(&a.path("conf")?)?;
            let store = Store::new(Some(a.path("data")?))?;
            let state = BankState::from_conf(&conf, Clock::default(), store)?;
            let listen = a.get("listen").or(conf.global.get("listen")).unwrap_or("0.0.0.0:5020").to_string();
            let (srv, _) = io(bank::start(state, &listen, a.opts()?, interval(a)?))?;
            serve_forever(srv, "銀行")
        }
        "statement" => {
            let conf = Conf::load(&a.path("conf")?)?;
            let store = Store::new(Some(a.path("data")?))?;
            println!("{}", BankState::from_conf(&conf, Clock::default(), store)?.statement());
            Ok(())
        }
        "recall" => {
            let Some(h) = a.pos.first() else {
                return proto("取扱番号が必要");
            };
            let n: u64 = h.parse().or_else(|_| proto(format!("取扱番号は数値: {h}")))?;
            let dir = a.path("data")?;
            io(std::fs::create_dir_all(&dir))?;
            use std::io::Write;
            let mut f = io(std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(dir.join("recall-requests.tsv")))?;
            io(writeln!(f, "{n}"))?;
            println!("組戻依頼を受け付けた。稼働中の銀行ノードが次の周期で処理する: {n}");
            Ok(())
        }
        _ => proto(format!("bank のサブコマンドが不明: {sub}\n{USAGE}")),
    }
}

fn cmd_net(sub: &str, a: &Args) -> Result<()> {
    match sub {
        "serve" => {
            let conf = Conf::load(&a.path("conf")?)?;
            let store = Store::new(Some(a.path("data")?))?;
            let state = NetState::from_conf(&conf, Clock::default(), store)?;
            let listen = a.get("listen").or(conf.global.get("listen")).unwrap_or("0.0.0.0:5020").to_string();
            let (srv, _) = io(net::start(state, &listen, a.opts()?, interval(a)?))?;
            serve_forever(srv, "全銀ネット")
        }
        "report" => {
            let conf = Conf::load(&a.path("conf")?)?;
            let store = Store::new(Some(a.path("data")?))?;
            println!("{}", NetState::from_conf(&conf, Clock::default(), store)?.report());
            Ok(())
        }
        "settle" => {
            let store = Store::new(Some(a.path("data")?))?;
            if a.has("offline") {
                // 全銀ネットを止めているときだけ使う。稼働中に実行すると、メモリ上の状態で上書きされる
                let conf = Conf::load(&a.path("conf")?)?;
                println!("{}", NetState::from_conf(&conf, Clock::default(), store)?.settle()?.text);
                return Ok(());
            }
            store.remove("settle-result.txt");
            store.write("settle-request", &["1".to_string()])?;
            let t = Instant::now();
            while t.elapsed() < Duration::from_secs(30) {
                if let Some(lines) = store.read("settle-result.txt")? {
                    println!("{}", lines.join("\n"));
                    return Ok(());
                }
                std::thread::sleep(Duration::from_millis(200));
            }
            store.remove("settle-request");
            proto("稼働中の全銀ネットから応答がない。止めている場合は --offline --conf を付ける")
        }
        _ => proto(format!("net のサブコマンドが不明: {sub}\n{USAGE}")),
    }
}

fn run() -> Result<()> {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let Some(cmd) = argv.first() else { return proto(USAGE) };
    let rest = &argv[1..];
    match cmd.as_str() {
        "gen" => {
            let Some(kind) = rest.first() else {
                return proto("sogo か kyuyo が必要");
            };
            let k = match kind.as_str() {
                "sogo" => FileKind::Sogo,
                "kyuyo" => FileKind::Kyuyo,
                k => return proto(format!("gen の種類が不明: {k}")),
            };
            cmd_gen(k, &Args::parse(&rest[1..])?)
        }
        "send" => cmd_send(&Args::parse(rest)?),
        "fetch" => cmd_fetch(&Args::parse(rest)?),
        "bank" | "net" => {
            let Some(sub) = rest.first() else { return proto(USAGE) };
            let a = Args::parse(&rest[1..])?;
            if cmd == "bank" { cmd_bank(sub, &a) } else { cmd_net(sub, &a) }
        }
        _ => proto(USAGE),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(ZError::Protocol(m)) if m.starts_with("使い方") => {
            eprintln!("{m}");
            ExitCode::from(2)
        }
        Err(e) => {
            eprintln!("エラー: {e}");
            ExitCode::FAILURE
        }
    }
}
