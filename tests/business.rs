//! 銀行ノードの、振込入金通知と為替通知以外の業務 (docs/spec-net.md §4.8)。
//! 通信は通さず、二次局のハンドラ (`Handler`) を直接呼ぶ。

use std::path::PathBuf;
use zengin::bank::BankState;
use zengin::charcode::Code;
use zengin::config::Conf;
use zengin::control::{FileCtl, RETRY_REQ, START_REQ, make_name, name_kind};
use zengin::handler::{Handler, InFile, OutFile, PeerCfg};
use zengin::record::{FileKind, Rec, fmt};
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
name = ﾀﾅｶ ｲﾁﾛｳ
branch = 10
kind = 1
number = 2
balance = 5000
";

const ACME: (u16, u8, &str) = (10, 1, "0000001");
const TANAKA: (u16, u8, &str) = (10, 1, "0000002");

fn clock() -> Clock {
    Clock::fixed(DateTime {
        y: 2026,
        mo: 11,
        d: 21,
        h: 10,
        mi: 30,
        s: 0,
    })
}

/// 試験ごとの一時ディレクトリ。終わると消す。
struct Tmp(PathBuf);

impl Tmp {
    fn new(name: &str) -> Tmp {
        let p = std::env::temp_dir().join(format!("zengin-biz-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        Tmp(p)
    }
    fn bank(&self) -> BankState {
        let store = Store::new(Some(self.0.clone())).unwrap();
        BankState::from_conf(&Conf::parse(CONF).unwrap(), clock(), store).unwrap()
    }
}

impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn acct(a: (u16, u8, &str)) -> (u16, u8, String) {
    (a.0, a.1, a.2.to_string())
}

fn peer(b: &BankState) -> PeerCfg {
    b.companies[0].peer.clone()
}

/// 企業から連絡モードで受け取ったものとして、ファイルを検査して確定する。
fn receive(b: &mut BankState, kind: FileKind, cycle: u8, data: Vec<u8>) -> Result<(), u8> {
    let f = fmt(kind);
    let p = peer(b);
    let name = make_name(&f.name_kind[..4], &f.name_kind[4..], cycle);
    b.start_recv(&p, &FileCtl::new(START_REQ, name, p.key, f.rec_len as u16, false))?;
    let file = InFile {
        name,
        rec_len: f.rec_len,
        rec_id: 0,
        data,
    };
    b.validate_recv(&p, &file)?;
    b.commit_recv(&p, file);
    Ok(())
}

/// 照会モードで取りに来たものとして、渡すファイルを作り、渡し終えたことにする。
fn fetch(b: &mut BankState, kind: FileKind, req: u8) -> Option<Vec<u8>> {
    let f = fmt(kind);
    let p = peer(b);
    let name = make_name(&f.name_kind[..4], &f.name_kind[4..], 1);
    let mut ctl = FileCtl::new(START_REQ, name, p.key, f.rec_len as u16, false);
    ctl.kind = req;
    let mut out: OutFile = b.start_send(&p, &ctl).unwrap()?;
    out.name = name;
    b.commit_send(&p, &out);
    assert_eq!(name_kind(&out.name), f.name_kind);
    Some(out.data)
}

/// 振込・口座振替・振込口座照会の明細 (受取人の口座と金額)。
fn item(f: &zengin::record::Fmt, bank: u64, a: (u16, u8, &str), name: &str, amount: u64) -> Rec {
    f.new_data()
        .set_num("bank_no", bank)
        .set_num("branch_no", a.0 as u64)
        .set_num("acct_kind", a.1 as u64)
        .set("acct_no", a.2)
        .set("name", name)
        .set_num("amount", amount)
}

fn header(f: &zengin::record::Fmt) -> Rec {
    f.new_header()
        .set_num("client_code", 1)
        .set("client_name", "ｶ)ｱｸﾐ")
        .set_num("bank_no", 1)
}

#[test]
fn transfer_like_kinds_are_processed_as_transfers() {
    // 賞与振込・地方公務員の給与振込・株式配当金振込なども、総合振込と同じく振込として処理する
    let t = Tmp::new("transfer");
    let mut b = t.bank();
    for (i, kind) in [FileKind::Shoyo, FileKind::ChihoKyuyo, FileKind::Haito, FileKind::Iryo]
        .into_iter()
        .enumerate()
    {
        let f = fmt(kind);
        let data = f
            .build(Code::Jis, header(f), &[item(f, 2, (20, 1, "0000001"), "ﾍﾞﾀ", 1000)])
            .unwrap();
        receive(&mut b, kind, i as u8 + 1, data).unwrap();
    }
    assert_eq!(b.balance(&acct(ACME)), Some(96000));
    assert_eq!(b.sent.len(), 4);
    assert_eq!(b.queued(), 4);
}

#[test]
fn direct_debit_moves_money_and_returns_results() {
    let t = Tmp::new("furikae");
    let mut b = t.bank();
    let f = fmt(FileKind::Furikae);
    let data = f
        .build(
            Code::Jis,
            header(f),
            &[
                item(f, 1, TANAKA, "ﾀﾅｶ", 3000),           // 振替済
                item(f, 1, TANAKA, "ﾀﾅｶ", 9000),           // 資金不足 (残り 2000)
                item(f, 1, (10, 1, "0000009"), "ﾅｼ", 100), // 取引なし
                item(f, 2, (20, 1, "0000001"), "ﾍﾞﾀ", 100), // 他行
            ],
        )
        .unwrap();
    receive(&mut b, FileKind::Furikae, 1, data).unwrap();
    assert_eq!(b.balance(&acct(TANAKA)), Some(2000));
    assert_eq!(b.balance(&acct(ACME)), Some(103000));

    let out = fetch(&mut b, FileKind::FurikaeKekka, START_REQ).expect("処理結果明細");
    let p = fmt(FileKind::FurikaeKekka).parse(&out).unwrap();
    let codes: Vec<u64> = p.data.iter().map(|d| d.num("result_code")).collect();
    assert_eq!(codes, [0, 1, 2, 9]);
    assert_eq!((p.trailer.num("ok_count"), p.trailer.num("ok_amount")), (1, 3000));
    assert_eq!((p.trailer.num("ng_count"), p.trailer.num("ng_amount")), (3, 9200));
    // 渡し終えたら、もうない。再送要求なら、同じものをもう一度渡す
    assert!(fetch(&mut b, FileKind::FurikaeKekka, START_REQ).is_none());
    assert_eq!(fetch(&mut b, FileKind::FurikaeKekka, RETRY_REQ).unwrap(), out);
}

#[test]
fn account_inquiry_checks_existence_and_name() {
    let t = Tmp::new("koza");
    let mut b = t.bank();
    let f = fmt(FileKind::KozaShokai);
    let data = f
        .build(
            Code::Ebcdic,
            header(f),
            &[
                item(f, 1, TANAKA, "ﾀﾅｶ ｲﾁﾛｳ", 0),
                item(f, 1, TANAKA, "ｽｽﾞｷ", 0),
                item(f, 1, (10, 1, "0000009"), "ﾅｼ", 0),
                item(f, 2, (20, 1, "0000001"), "ﾍﾞﾀ", 0),
            ],
        )
        .unwrap();
    receive(&mut b, FileKind::KozaShokai, 1, data).unwrap();
    let out = fetch(&mut b, FileKind::KozaShokaiKekka, START_REQ).unwrap();
    let p = fmt(FileKind::KozaShokaiKekka).parse(&out).unwrap();
    assert_eq!(p.code, Code::Ebcdic, "依頼と同じコードで返す");
    let codes: Vec<u64> = p.data.iter().map(|d| d.num("result_code")).collect();
    assert_eq!(codes, [0, 2, 1, 9]);
}

#[test]
fn zaikei_accumulates_per_member() {
    let t = Tmp::new("zaikei");
    let mut b = t.bank();
    let f = fmt(FileKind::Zaikei);
    let rec = |member: &str, salary: u64| f.new_data().set("member_code", member).set_num("salary_amount", salary);
    for cycle in 1..=2 {
        let data = f
            .build(
                Code::Jis,
                f.new_header().set_num("client_code", 1),
                &[rec("11", 10000), rec("12", 500)],
            )
            .unwrap();
        receive(&mut b, FileKind::Zaikei, cycle, data).unwrap();
        let out = fetch(&mut b, FileKind::ZaikeiTsumitate, START_REQ).unwrap();
        let p = fmt(FileKind::ZaikeiTsumitate).parse(&out).unwrap();
        let balances: Vec<u64> = p.data.iter().map(|d| d.num("balance")).collect();
        assert_eq!(balances, [10000 * cycle as u64, 500 * cycle as u64]);
    }
    assert_eq!(b.balance(&acct(ACME)), Some(100000 - 21000));
}

#[test]
fn statement_and_balance_notice() {
    let t = Tmp::new("meisai");
    let mut b = t.bank();
    // 取引がなければ、入出金取引明細はない。残高通知はいつでもある
    assert!(fetch(&mut b, FileKind::Meisai, START_REQ).is_none());
    let f = fmt(FileKind::Sogo);
    let data = f.build(Code::Jis, header(f), &[item(f, 1, TANAKA, "ﾀﾅｶ", 700)]).unwrap();
    receive(&mut b, FileKind::Sogo, 1, data).unwrap();

    let out = fetch(&mut b, FileKind::Meisai, START_REQ).unwrap();
    let p = fmt(FileKind::Meisai).parse(&out).unwrap();
    assert_eq!(p.data.len(), 1);
    assert_eq!((p.data[0].num("io_kind"), p.data[0].num("amount")), (2, 700));
    assert_eq!((p.trailer.num("out_count"), p.trailer.num("out_amount")), (1, 700));
    assert_eq!(p.header.num("prev_balance"), 100000);
    assert_eq!(p.header.get("from"), "081121", "和暦 YYMMDD");
    assert!(fetch(&mut b, FileKind::Meisai, START_REQ).is_none(), "通知済み");

    let out = fetch(&mut b, FileKind::Zandaka, START_REQ).unwrap();
    let p = fmt(FileKind::Zandaka).parse(&out).unwrap();
    assert_eq!(p.data[0].num("balance"), 99300);
    assert!(fetch(&mut b, FileKind::Zandaka, START_REQ).is_some());
}

#[test]
fn published_files_record_only_requests_and_restart() {
    let t = Tmp::new("publish");
    {
        let mut b = t.bank();
        // 担当者が登録したファイル (zengin bank publish)
        let f = fmt(FileKind::LoanZaikei);
        let data = f
            .build(
                Code::Jis,
                f.new_header().set_num("client_code", 1),
                &[f.new_data().set_num("balance", 1000000)],
            )
            .unwrap();
        let hex: String = data.iter().map(|b| format!("{b:02x}")).collect();
        std::fs::write(t.0.join("publish-requests.tsv"), format!("ACME\t50200211\t{hex}\n")).unwrap();
        b.take_requests();
        // 外国送金依頼は受け付けて記録するだけ
        let f = fmt(FileKind::Soukin);
        let data = f
            .build(
                Code::Jis,
                f.new_header().set_num("remitter_code", 1),
                &[f.new_data_of("本体情報").unwrap().set("record_kind", "01")],
            )
            .unwrap();
        receive(&mut b, FileKind::Soukin, 1, data).unwrap();
        // 取引の履歴も残る
        let f = fmt(FileKind::Sogo);
        let data = f.build(Code::Jis, header(f), &[item(f, 1, TANAKA, "ﾀﾅｶ", 1)]).unwrap();
        receive(&mut b, FileKind::Sogo, 1, data).unwrap();
    }
    let inbox = std::fs::read_to_string(t.0.join("inbox.tsv")).unwrap();
    assert!(inbox.contains("ACME\t外国送金依頼\t1"), "{inbox}");
    // 再起動しても、登録したファイルと取引の履歴は残っている
    let mut b = t.bank();
    assert_eq!(b.history(&acct(TANAKA)).len(), 1);
    let out = fetch(&mut b, FileKind::LoanZaikei, START_REQ).expect("登録したファイル");
    assert_eq!(fmt(FileKind::LoanZaikei).parse(&out).unwrap().data[0].num("balance"), 1000000);
    assert!(fetch(&mut b, FileKind::LoanZaikei, START_REQ).is_none());
}

#[test]
fn company_cannot_send_inquiry_kinds_or_exchange_notices() {
    let t = Tmp::new("reject");
    let mut b = t.bank();
    let p = peer(&b);
    for kind in [FileKind::Meisai, FileKind::Ex91] {
        let f = fmt(kind);
        let name = make_name(&f.name_kind[..4], &f.name_kind[4..], 1);
        assert!(
            b.start_recv(&p, &FileCtl::new(START_REQ, name, p.key, f.rec_len as u16, false))
                .is_err()
        );
    }
}
