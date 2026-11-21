use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};
use zengin::bank::{self, BankState};
use zengin::charcode::Code;
use zengin::config::Conf;
use zengin::control::{CenterCode, make_name};
use zengin::handler::{InFile, OutFile, PeerCfg};
use zengin::net::{self, NetState, connect};
use zengin::record::{self, FileKind, Fmt, Rec, fmt};
use zengin::session::{self, Done, Job, Opts, fetch_file};
use zengin::store::Store;
use zengin::util::{Clock, Result, ZError, proto};

const USAGE: &str = "\
使い方:
  企業:
    zengin gen 業務 --out FILE [--code jis|ebcdic] [--client-code N] [--client-name NAME] [--bank N]
               [--item 銀行:支店:種目:口座:名前:金額 ...] [--set 項目=値 ...] [--rec 「項目=値;項目=値」 ...]
               (--item は振込・口座振替・振込口座照会の明細。--set はヘッダー、--rec はデータ・レコード1件)
    zengin send  --conf FILE --file FILE [--file ...] [--kind 業務] [--to ADDR] [--cycle N] [--compress]
                 [--fetch OUT] [--mn N]   (--file を重ねると1回の接続で続けて送る。--fetch で同じ接続で振込入金通知も取る)
    zengin fetch --conf FILE --out FILE [--kind 業務] [--to ADDR] [--cycle N] [--resend] [--mn N]
    zengin show  --kind 業務 FILE        (ファイルの中身を表示する)
    zengin kinds                         (業務の一覧)
  銀行ノード:
    zengin bank serve     --conf FILE --data DIR [--listen ADDR] [--interval 秒] [--mn N] [--resume]
    zengin bank statement --conf FILE --data DIR
    zengin bank recall    --data DIR 取扱番号
    zengin bank publish   --data DIR --company 企業 --kind 業務 --file FILE   (照会で渡すファイルを登録する)
    zengin bank inbox     --data DIR       (受け付けた外国送金依頼などの一覧)
  全銀ネット:
    zengin net serve  --conf FILE --data DIR [--listen ADDR] [--interval 秒] [--mn N] [--resume]
                      (--resume: 途中で切れた受信を、接続し直したときに続きから受ける)
    zengin net report --conf FILE --data DIR
    zengin net settle --data DIR [--offline --conf FILE]
  業務: 名前 (sogo、kyuyo、nyukin、meisai など。zengin kinds で一覧) か、データコード 4 桁 (0121 など)。
  共通の通信オプション:
    --text-len N   送るテキスト長の上限 (既定 2048。相手が対応していれば 32767 まで)
    --basic        TTC の接続形態区分を 0 (ベーシック手順、汎用機どうし) にする
    --hours HHMM-HHMM  待ち受け側のサービス時間帯 (外の開局要求には 13 を返す)
";

struct Args {
    flags: HashMap<String, Vec<String>>,
    pos: Vec<String>,
}

impl Args {
    fn parse(v: &[String]) -> Result<Args> {
        const SWITCHES: [&str; 5] = ["basic", "compress", "offline", "resend", "resume"];
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
    fn all(&self, k: &str) -> Vec<&str> {
        self.flags.get(k).map_or(Vec::new(), |v| v.iter().map(String::as_str).collect())
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
            resume: self.has("resume").then(Default::default),
            text_len: self.num("text-len", zengin::text::MAX_TEXT)?,
            basic: self.has("basic"),
            hours: self.get("hours").map(hours).transpose()?,
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
/// `--hours 0900-1700` を (900, 1700) にする。
fn hours(s: &str) -> Result<(u16, u16)> {
    let p = |x: &str| -> Option<u16> {
        let v: u16 = x.parse().ok()?;
        (x.len() == 4 && v / 100 <= 24 && v % 100 < 60).then_some(v)
    };
    match s.split_once('-').map(|(a, b)| (p(a), p(b))) {
        Some((Some(a), Some(b))) => Ok((a, b)),
        _ => proto(format!("--hours は HHMM-HHMM: {s:?}")),
    }
}

fn cycle(a: &Args) -> Result<u8> {
    let c = a.num("cycle", 1u8)?;
    if !(1..=99).contains(&c) {
        return proto("--cycle は 1〜99");
    }
    Ok(c)
}

/// 業務の名前 (FileKind の名前を小文字にしたもの)。
fn kind_name(k: FileKind) -> String {
    format!("{k:?}").to_lowercase()
}

/// 業務の名前かデータコードから、フォーマットを引く。`inquiry` で照会か連絡かを決める (同じデータコードの業務があるため)。
fn kind_by(s: &str, inquiry: bool) -> Result<&'static Fmt> {
    let s = s.to_lowercase();
    record::all()
        .filter(|f| !matches!(f.kind, FileKind::Ex91 | FileKind::Ex92 | FileKind::Ex93 | FileKind::Ex94))
        .find(|f| (kind_name(f.kind) == s || f.name_kind[4..] == s) && f.inquiry == inquiry)
        .map_or_else(
            || {
                let mode = if inquiry { "照会" } else { "連絡" };
                proto(format!("{mode}の業務ではない: {s} (zengin kinds で一覧)"))
            },
            Ok,
        )
}

fn kind_of(a: &Args) -> Result<FileKind> {
    Ok(kind_by(a.get("kind").unwrap_or("sogo"), false)?.kind)
}

fn code_of(a: &Args) -> Result<Code> {
    match a.get("code").unwrap_or("jis") {
        "jis" => Ok(Code::Jis),
        "ebcdic" => Ok(Code::Ebcdic),
        c => proto(format!("--code は jis か ebcdic: {c}")),
    }
}

/// `項目=値` を、レコードの項目に入れる。
fn set_fields(mut r: Rec, spec: &str, sep: char) -> Result<Rec> {
    for kv in spec.split(sep).map(str::trim).filter(|s| !s.is_empty()) {
        let Some((k, v)) = kv.split_once('=') else {
            return proto(format!("項目=値 の形でない: {kv}"));
        };
        if !r.has(k) {
            let names: Vec<&str> = r.layout.fields.iter().map(|f| f.name).collect();
            return proto(format!("項目 {k} がない。使える項目: {}", names.join(" ")));
        }
        r = r.set(k, v);
    }
    Ok(r)
}

fn cmd_gen(f: &'static Fmt, a: &Args) -> Result<()> {
    let now = Clock::default().now();
    let mut h = f.new_header();
    let opt = [
        ("client_code", a.get("client-code")),
        ("client_name", a.get("client-name")),
        ("bank_no", a.get("bank")),
    ];
    for (k, v) in opt {
        if let (true, Some(v)) = (h.has(k), v) {
            h = h.set(k, v);
        }
    }
    // 振込の業務のヘッダーの取組日 (MMDD) は、今日にしておく
    if h.layout.fields.iter().any(|f| f.name == "date" && f.len == 4) {
        h = h.set("date", now.mmdd());
    }
    for kv in a.all("set") {
        h = set_fields(h, kv, ',')?;
    }
    let mut data = Vec::new();
    let items = a.all("item");
    let item_fields = ["bank_no", "branch_no", "acct_kind", "acct_no", "name", "amount"];
    if !items.is_empty() && !item_fields.iter().all(|k| f.new_data().has(k)) {
        return proto(format!("{} には --item を使えない。--rec で項目を指定する", f.title));
    }
    for item in items {
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
    for spec in a.all("rec") {
        // データ・レコード区分 (record_kind) の値で様式を選ぶ。指定がなければ最初の様式
        let kind = spec.split(';').find_map(|kv| kv.trim().strip_prefix("record_kind="));
        let v = f
            .datas
            .iter()
            .find(|v| kind.is_some_and(|k| v.kinds.iter().any(|x| x.trim_start_matches('0') == k.trim_start_matches('0'))))
            .unwrap_or(&f.datas[0]);
        data.push(set_fields(Rec::new(v.layout), spec, ';')?);
    }
    let bytes = f.build(code_of(a)?, h, &data)?;
    f.parse(&bytes)?; // 様式の選び方まで含めて、読み戻せること
    io(std::fs::write(a.path("out")?, &bytes))?;
    println!("{} 件、{} バイト", data.len(), bytes.len());
    Ok(())
}

fn cmd_send(a: &Args) -> Result<()> {
    let conf = Conf::load(&a.path("conf")?)?;
    let cfg = company_cfg(&conf)?;
    let kind = kind_of(a)?;
    let fm = fmt(kind);
    let opts = a.opts()?;
    let addr = a
        .get("to")
        .or(conf.global.get("addr"))
        .map_or_else(|| proto("--to か設定の addr が必要"), Ok)?;
    let paths = a.all("file");
    if paths.is_empty() {
        return proto("--file が必要");
    }
    let first_cycle = cycle(a)?;
    let mut jobs = Vec::new();
    let mut counts = Vec::new();
    for (i, path) in paths.iter().enumerate() {
        let data = io(std::fs::read(path))?;
        let parsed = fm.parse(&data)?; // 送る前に自分で検査する
        counts.push(parsed.data.len());
        // 1回の接続で同じファイル名を重ねないよう、2つ目からはサイクルを1つずつ進める
        let cycle = ((first_cycle as usize - 1 + i) % 99 + 1) as u8;
        jobs.push(Job::Send(OutFile {
            name: make_name(&fm.name_kind[..4], &fm.name_kind[4..], cycle),
            rec_len: fm.rec_len,
            rec_id: 0,
            code: parsed.code,
            data,
            compress: a.has("compress"),
        }));
    }
    let nyukin = fmt(FileKind::Nyukin);
    if a.has("fetch") {
        jobs.push(Job::Fetch {
            name: make_name(&nyukin.name_kind[..4], &nyukin.name_kind[4..], 1),
            rec_len: nyukin.rec_len,
            resend: false,
        });
    }
    let done = session::run(io(connect(addr, &opts))?, &cfg, &Clock::default(), &jobs, &opts)?;
    for (path, n) in paths.iter().zip(&counts) {
        println!("送信した: {path} {n} 件");
    }
    if let Some(Done::Fetched(f)) = done.last().filter(|_| a.has("fetch")) {
        show_notices(f.as_ref(), Path::new(a.req("fetch")?))?;
    }
    Ok(())
}

fn cmd_fetch(a: &Args) -> Result<()> {
    let conf = Conf::load(&a.path("conf")?)?;
    let cfg = company_cfg(&conf)?;
    let fm = kind_by(a.get("kind").unwrap_or("nyukin"), true)?;
    let opts = a.opts()?;
    let addr = a
        .get("to")
        .or(conf.global.get("addr"))
        .map_or_else(|| proto("--to か設定の addr が必要"), Ok)?;
    let name = make_name(&fm.name_kind[..4], &fm.name_kind[4..], cycle(a)?);
    let file = fetch_file(
        io(connect(addr, &opts))?,
        &cfg,
        &Clock::default(),
        name,
        fm.rec_len,
        a.has("resend"),
        &opts,
    )?;
    if fm.kind == FileKind::Nyukin {
        return show_notices(file.as_ref(), &a.path("out")?);
    }
    let Some(file) = file else {
        println!("{} はない", fm.title);
        return Ok(());
    };
    io(std::fs::write(a.path("out")?, &file.data))?;
    show(fm, &file.data)
}

/// ファイルの中身を、組ごとに表示する (値のある項目だけ)。
fn show(fm: &Fmt, data: &[u8]) -> Result<()> {
    let p = fm.parse(data)?;
    let line = |r: &Rec| {
        let fields = r
            .layout
            .fields
            .iter()
            .filter(|f| f.name != "data_kind" && !f.name.starts_with("dummy"));
        let vals = fields.filter_map(|f| {
            let v = r.get(f.name);
            // 空と、すべて 0 の項目は省く
            (!v.trim_start_matches('0').is_empty()).then(|| format!("{}={v}", f.name))
        });
        vals.collect::<Vec<_>>().join(" ")
    };
    println!("{} ({:?}): {} 組、データ {} 件", fm.title, p.code, p.groups.len(), p.data.len());
    for g in &p.groups {
        println!("ヘッダー: {}", line(&g.header));
        for d in &g.data {
            let label = d.layout_label(fm);
            println!(
                "  {}{}",
                if label.is_empty() { String::new() } else { format!("[{label}] ") },
                line(d)
            );
        }
        println!("トレーラ: {}", line(&g.trailer));
    }
    Ok(())
}

/// 受け取った振込入金通知を保存して、内容を表示する。
fn show_notices(file: Option<&InFile>, out: &Path) -> Result<()> {
    let Some(file) = file else {
        println!("振込入金通知はない");
        return Ok(());
    };
    let p = fmt(FileKind::Nyukin).parse(&file.data)?;
    io(std::fs::write(out, &file.data))?;
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
        "publish" => {
            let fm = kind_by(a.req("kind")?, true)?;
            let data = io(std::fs::read(a.path("file")?))?;
            fm.parse(&data)?;
            let company = a.req("company")?;
            let dir = a.path("data")?;
            io(std::fs::create_dir_all(&dir))?;
            let hex: String = data.iter().map(|b| format!("{b:02x}")).collect();
            use std::io::Write;
            let mut f = io(std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(dir.join("publish-requests.tsv")))?;
            io(writeln!(f, "{company}\t{}\t{hex}", fm.name_kind))?;
            println!("{} を {company} 向けに登録した。稼働中の銀行ノードが次の周期で取り込む", fm.title);
            Ok(())
        }
        "inbox" => {
            let path = a.path("data")?.join("inbox.tsv");
            let text = std::fs::read_to_string(&path).unwrap_or_default();
            if text.is_empty() {
                println!("受け付けた依頼はない");
            }
            for l in text.lines() {
                println!("{}", l.replace('\t', "  "));
            }
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
                return proto("業務の名前が必要 (zengin kinds で一覧)");
            };
            let f = kind_by(kind, false).or_else(|_| kind_by(kind, true))?;
            cmd_gen(f, &Args::parse(&rest[1..])?)
        }
        "show" => {
            let a = Args::parse(rest)?;
            let Some(path) = a.pos.first() else {
                return proto("ファイルが必要");
            };
            let data = io(std::fs::read(path))?;
            let k = a.req("kind")?;
            let f = kind_by(k, true).or_else(|_| kind_by(k, false))?;
            show(f, &data)
        }
        "kinds" => {
            for f in record::all().filter(|f| !matches!(f.kind, FileKind::Ex91 | FileKind::Ex92 | FileKind::Ex93 | FileKind::Ex94)) {
                let mode = if f.inquiry { "照会" } else { "連絡" };
                println!(
                    "{:<16} {}  {mode}  {:>3}B  {}",
                    kind_name(f.kind),
                    &f.name_kind[4..],
                    f.rec_len,
                    f.title
                );
            }
            Ok(())
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
