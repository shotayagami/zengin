//! 設定ファイルの値の検査。範囲外の値を黙って丸めずに、エラーにする。

use zengin::bank::BankState;
use zengin::config::Conf;
use zengin::net::NetState;
use zengin::store::Store;
use zengin::util::Clock;

const BANK: &str = "bank = 1
name = ｷﾞﾝｺｳ
center = 00000000010001
net_center = 00000000000001
net_password = P
net_key = K
net_addr = 127.0.0.1:1
";
const COMPANY: &str = "[company A]
center = 00000001010001
password = P
key = K
code = 1
branch = 10
kind = 1
account = 1
";

fn bank(text: &str) -> zengin::util::Result<BankState> {
    BankState::from_conf(&Conf::parse(text)?, Clock::default(), Store::new(None)?)
}

fn net(text: &str) -> zengin::util::Result<NetState> {
    NetState::from_conf(&Conf::parse(text)?, Clock::default(), Store::new(None)?)
}

#[test]
fn bank_config_is_validated() {
    assert!(bank(&format!("{BANK}{COMPANY}")).is_ok());
    for (bad, why) in [
        (BANK.replace("bank = 1", "bank = 0"), "銀行番号 0 (全銀ネットの番号)"),
        (BANK.replace("bank = 1", "bank = 70000"), "u16 を超える銀行番号"),
        (BANK.replace("ｷﾞﾝｺｳ", "abc"), "使えない文字の銀行名"),
        (BANK.replace("ｷﾞﾝｺｳ", "ｷﾞﾝｺｳｷﾞﾝｺｳｷﾞﾝｺｳｷﾞﾝｺｳ"), "15文字を超える銀行名"),
    ] {
        assert!(bank(&format!("{bad}{COMPANY}")).is_err(), "{why}");
    }
    for (from, to, why) in [
        ("branch = 10", "branch = 70000", "u16 を超える支店番号"),
        ("branch = 10", "branch = 1000", "3桁を超える支店番号"),
        ("kind = 1", "kind = 10", "預金種目が1桁を超える"),
        ("account = 1", "account = 10000000", "口座番号が7桁を超える"),
        ("code = 1", "code = 10000000000", "企業コードが10桁を超える"),
        ("code = 1", "code = -1", "負の企業コード"),
    ] {
        assert!(bank(&format!("{BANK}{}", COMPANY.replace(from, to))).is_err(), "{why}");
    }
    assert!(
        bank(&format!("{BANK}{}", COMPANY.replace("[company A]", "[company net]"))).is_err(),
        "企業名 net"
    );
    assert!(
        bank(&format!(
            "{BANK}{COMPANY}balance = 99999999999999999999
"
        ))
        .is_err(),
        "残高が範囲外"
    );
}

#[test]
fn net_config_is_validated() {
    let ok = "center = 00000000000001
[bank 0001]
center = 00000000010001
password = P
key = K
addr = 127.0.0.1:1
limit = 5000
";
    assert!(net(ok).is_ok());
    for (from, to, why) in [
        ("[bank 0001]", "[bank 0000]", "銀行番号 0"),
        ("[bank 0001]", "[bank 10000]", "銀行番号が5桁"),
        ("[bank 0001]", "[bank x]", "数字でない銀行番号"),
        (
            "limit = 5000",
            "limit = 18446744073709551615",
            "限度額が大きすぎる (i64 を超える計算になる)",
        ),
        ("limit = 5000", "limit = -1", "負の限度額"),
    ] {
        assert!(net(&ok.replace(from, to)).is_err(), "{why}");
    }
}
