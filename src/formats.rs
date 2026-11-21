//! 別冊の業務のレコード・フォーマット (振込入金通知・給与振込・総合振込を除く)。
//! scripts/gen-formats.py が formats/*.json から生成する。手で直さないこと。

use crate::record::{Agg, Check, FileKind, Fmt, Layout, Var, c, n};

/// 入出金取引明細 header
static L02_HEADER: Layout = Layout {
    fields: &[
        n("data_kind", 1),     // データ区分
        n("type", 2),          // 種別コード
        n("code", 1),          // コード区分
        n("created", 6),       // 作成日
        n("from", 6),          // 勘定日（自）
        n("to", 6),            // 勘定日（至）
        n("bank_no", 4),       // 銀行コード
        c("bank_name", 15),    // 銀行名
        n("branch_no", 3),     // 支店コード
        c("branch_name", 15),  // 支店名
        n("dummy", 3),         // ダミー
        n("acct_kind", 1),     // 預金種目
        n("acct_no", 10),      // 口座番号（注）
        c("acct_name", 40),    // 口座名
        n("od_flag", 1),       // ※貸越区分
        n("passbook_kind", 1), // ※通帳・証書区分
        n("prev_balance", 14), // ※取引前残高
        c("dummy2", 71),       // ダミー
    ],
};
/// 入出金取引明細 data 普通・当座・貯蓄
static L02_DATA_0: Layout = Layout {
    fields: &[
        n("data_kind", 1),     // データ区分
        n("inquiry_no", 8),    // ※照会番号
        n("acct_date", 6),     // 勘定日
        n("start_date", 6),    // 預入・払出日
        n("io_kind", 1),       // 入払区分（注）
        n("txn_kind", 2),      // ※取引区分
        n("amount", 12),       // 取引金額
        n("other_amount", 12), // うち他店券金額
        n("clearing_date", 6), // ※交換呈示日
        n("dishonor_date", 6), // ※不渡返還日
        n("bill_kind", 1),     // ※手形・小切手区分
        n("bill_no", 7),       // ※手形・小切手番号
        n("txn_branch", 3),    // ※僚店番号
        n("client_code", 10),  // ※振込依頼人コード
        c("client_name", 48),  // ※振込依頼人名または契約者番号
        c("src_bank", 15),     // ※仕向銀行名
        c("src_branch", 15),   // ※仕向店名
        c("summary", 20),      // ※摘要内容
        c("edi", 20),          // ※EDI 情報
        c("dummy", 1),         // ダミー
    ],
};
/// 入出金取引明細 data 通知・定期・積立定期
static L02_DATA_1: Layout = Layout {
    fields: &[
        n("data_kind", 1),            // データ区分
        n("ident_no", 8),             // ※識別番号
        n("acct_date", 6),            // 勘定日
        n("start_date", 6),           // 預入・払出日
        n("io_kind", 1),              // 入払区分（注）
        n("txn_kind", 2),             // ※取引区分
        n("amount", 12),              // 取引金額
        n("other_amount", 12),        // うち他店券金額
        n("clearing_date", 6),        // ※交換呈示日
        n("dishonor_date", 6),        // ※不渡返還日
        n("bill_kind", 1),            // ※手形・小切手区分
        n("bill_no", 7),              // ※手形・小切手番号
        n("txn_branch", 3),           // ※僚店番号
        n("orig_deposit_date", 6),    // ※当初預入日
        n("rate", 6),                 // 利率（注2）
        n("maturity_date", 6),        // ※満期日
        n("period1", 7),              // ※期間(1)
        n("period_interest", 11),     // ※期間利息
        n("mid_rate", 6),             // ※中間払利率
        n("mid_kind", 1),             // ※中間払区分
        n("post_days", 4),            // ※期後期間
        n("post_rate", 6),            // ※期後利率
        n("post_interest", 9),        // ※期後利息
        n("total_interest", 11),      // ※合計利息
        n("tax_kind", 1),             // ※税区分
        n("tax_rate", 4),             // ※税率
        n("tax_amount", 10),          // ※税額
        n("net_interest", 11),        // ※税引後利息
        c("summary", 20),             // ※摘要内容
        n("period2", 5),              // ※期間（2）
        n("period_interest_sign", 1), // ※期間利息正負表示
        c("dummy", 4),                // ダミー
    ],
};
/// 入出金取引明細 trailer
static L02_TRAILER: Layout = Layout {
    fields: &[
        n("data_kind", 1),   // データ区分
        n("in_count", 6),    // 入金件数
        n("in_amount", 13),  // 入金額合計
        n("out_count", 6),   // 出金件数
        n("out_amount", 13), // 出金額合計
        n("od_flag", 1),     // ※貸越区分
        n("balance", 14),    // ※取引後残高
        n("count", 7),       // データ・レコード件数
        c("dummy", 139),     // ダミー
    ],
};
/// 入出金取引明細 end
static L02_END: Layout = Layout {
    fields: &[
        n("data_kind", 1),      // データ区分
        n("total_records", 10), // レコード総件数
        n("acct_count", 5),     // 口座数
        c("dummy", 184),        // ダミー
    ],
};
/// 残高通知（預金） header
static L03_HEADER: Layout = Layout {
    fields: &[
        n("data_kind", 1),    // データ区分
        n("type", 2),         // 種別コード
        n("notice_kind", 1),  // 通知区分
        n("code", 1),         // コード区分
        n("created", 6),      // 作成日
        n("client_code", 10), // 会社コード
        c("client_name", 40), // 会社名
        n("bank_no", 4),      // 銀行コード
        c("bank_name", 15),   // 銀行名
        n("branch_no", 3),    // 支店コード
        c("branch_name", 15), // 支店名
        c("dummy", 102),      // ダミー
    ],
};
/// 残高通知（預金） data
static L03_DATA_0: Layout = Layout {
    fields: &[
        n("data_kind", 1),      // データ区分
        n("base_date", 6),      // 基準日
        n("base_time", 4),      // ※基準時刻
        n("branch_no", 3),      // 支店コード
        n("dummy", 3),          // ダミー
        n("acct_kind", 1),      // 預金種目
        n("acct_no", 10),       // 口座番号（注）
        n("units", 4),          // ※口数
        c("acct_name", 40),     // 口座名
        n("od_flag", 1),        // 現在残高 (1) 貸越区分
        n("balance", 14),       // 現在残高 (2) 残高
        n("other_balance", 14), // 他店券残高
        n("od_limit", 14),      // 貸越極度額
        n("avail_od_flag", 1),  // ※支払可能残高 (1) 貸越区分
        n("avail_balance", 14), // ※支払可能残高 (2) 残高
        n("prev_od_flag", 1),   // ※前日残高 (1) 貸越区分
        n("prev_balance", 14),  // ※前日残高 (2) 残高
        n("last_txn_date", 6),  // ※最新取引日
        c("dummy2", 49),        // ダミー
    ],
};
/// 残高通知（預金） trailer
static L03_TRAILER: Layout = Layout {
    fields: &[
        n("data_kind", 1), // データ区分
        n("count", 7),     // データ・レコード総件数
        c("dummy", 192),   // ダミー
    ],
};
/// 残高通知（預金） end
static L03_END: Layout = Layout {
    fields: &[
        n("data_kind", 1),      // データ区分
        n("total_records", 10), // レコード総件数
        c("dummy", 189),        // ダミー
    ],
};
/// 賞与振込（民間） header
static L05_HEADER: Layout = Layout {
    fields: &[
        n("data_kind", 1),    // データ区分
        n("type", 2),         // 種別コード
        n("code", 1),         // コード区分
        n("client_code", 10), // 会社コード
        c("client_name", 40), // 会社名
        n("date", 4),         // 振込指定日
        n("bank_no", 4),      // 仕向銀行番号
        c("bank_name", 15),   // ※仕向銀行名
        n("branch_no", 3),    // 仕向支店番号
        c("branch_name", 15), // ※仕向支店名
        n("acct_kind", 1),    // ※預金種目(企業等)
        n("acct_no", 7),      // ※口座番号(企業等)
        c("dummy", 17),       // ダミー
    ],
};
/// 賞与振込（民間） data
static L05_DATA_0: Layout = Layout {
    fields: &[
        n("data_kind", 1),    // データ区分
        n("bank_no", 4),      // 被仕向銀行番号
        c("bank_name", 15),   // 被仕向銀行名
        n("branch_no", 3),    // 被仕向支店番号
        c("branch_name", 15), // 被仕向支店名
        n("clearing", 4),     // ※手形交換所番号
        n("acct_kind", 1),    // 預金種目
        n("acct_no", 7),      // 口座番号
        c("name", 30),        // 預金者名
        n("amount", 10),      // 振込金額
        n("new_code", 1),     // 新規コード
        n("emp_no", 10),      // ※社員番号
        n("dept", 10),        // ※所属コード
        c("dummy", 9),        // ダミー
    ],
};
/// 賞与振込（民間） trailer
static L05_TRAILER: Layout = Layout {
    fields: &[
        n("data_kind", 1), // データ区分
        n("count", 6),     // 合計件数
        n("amount", 12),   // 合計金額
        c("dummy", 101),   // ダミー
    ],
};
/// 賞与振込（民間） end
static L05_END: Layout = Layout {
    fields: &[
        n("data_kind", 1), // データ区分
        c("dummy", 119),   // ダミー
    ],
};
/// 給与振込（地方公務員） header
static L06_HEADER: Layout = Layout {
    fields: &[
        n("data_kind", 1),    // データ区分
        n("type", 2),         // 種別コード
        n("code", 1),         // コード区分
        n("client_code", 10), // 会社コード
        c("client_name", 40), // 会社名
        n("date", 4),         // 振込指定日
        n("bank_no", 4),      // 仕向銀行番号
        c("bank_name", 15),   // ※仕向銀行名
        n("branch_no", 3),    // 仕向支店番号
        c("branch_name", 15), // ※仕向支店名
        n("acct_kind", 1),    // ※預金種目(企業等)
        n("acct_no", 7),      // ※口座番号(企業等)
        c("dummy", 17),       // ダミー
    ],
};
/// 給与振込（地方公務員） data
static L06_DATA_0: Layout = Layout {
    fields: &[
        n("data_kind", 1),    // データ区分
        n("bank_no", 4),      // 被仕向銀行番号
        c("bank_name", 15),   // 被仕向銀行名
        n("branch_no", 3),    // 被仕向支店番号
        c("branch_name", 15), // 被仕向支店名
        n("clearing", 4),     // ※手形交換所番号
        n("acct_kind", 1),    // 預金種目
        n("acct_no", 7),      // 口座番号
        c("name", 30),        // 預金者名
        n("amount", 10),      // 振込金額
        n("new_code", 1),     // 新規コード
        n("emp_no", 10),      // ※社員番号
        n("dept", 10),        // ※所属コード
        c("dummy", 9),        // ダミー
    ],
};
/// 給与振込（地方公務員） trailer
static L06_TRAILER: Layout = Layout {
    fields: &[
        n("data_kind", 1), // データ区分
        n("count", 6),     // 合計件数
        n("amount", 12),   // 合計金額
        c("dummy", 101),   // ダミー
    ],
};
/// 給与振込（地方公務員） end
static L06_END: Layout = Layout {
    fields: &[
        n("data_kind", 1), // データ区分
        c("dummy", 119),   // ダミー
    ],
};
/// 賞与振込（地方公務員） header
static L07_HEADER: Layout = Layout {
    fields: &[
        n("data_kind", 1),    // データ区分
        n("type", 2),         // 種別コード
        n("code", 1),         // コード区分
        n("client_code", 10), // 会社コード
        c("client_name", 40), // 会社名
        n("date", 4),         // 振込指定日
        n("bank_no", 4),      // 仕向銀行番号
        c("bank_name", 15),   // ※仕向銀行名
        n("branch_no", 3),    // 仕向支店番号
        c("branch_name", 15), // ※仕向支店名
        n("acct_kind", 1),    // ※預金種目(企業等)
        n("acct_no", 7),      // ※口座番号(企業等)
        c("dummy", 17),       // ダミー
    ],
};
/// 賞与振込（地方公務員） data
static L07_DATA_0: Layout = Layout {
    fields: &[
        n("data_kind", 1),    // データ区分
        n("bank_no", 4),      // 被仕向銀行番号
        c("bank_name", 15),   // 被仕向銀行名
        n("branch_no", 3),    // 被仕向支店番号
        c("branch_name", 15), // 被仕向支店名
        n("clearing", 4),     // ※手形交換所番号
        n("acct_kind", 1),    // 預金種目
        n("acct_no", 7),      // 口座番号
        c("name", 30),        // 預金者名
        n("amount", 10),      // 振込金額
        n("new_code", 1),     // 新規コード
        n("emp_no", 10),      // ※社員番号
        n("dept", 10),        // ※所属コード
        c("dummy", 9),        // ダミー
    ],
};
/// 賞与振込（地方公務員） trailer
static L07_TRAILER: Layout = Layout {
    fields: &[
        n("data_kind", 1), // データ区分
        n("count", 6),     // 合計件数
        n("amount", 12),   // 合計金額
        c("dummy", 101),   // ダミー
    ],
};
/// 賞与振込（地方公務員） end
static L07_END: Layout = Layout {
    fields: &[
        n("data_kind", 1), // データ区分
        c("dummy", 119),   // ダミー
    ],
};
/// 株式配当金振込 header
static L09_HEADER: Layout = Layout {
    fields: &[
        n("data_kind", 1),    // データ区分
        n("type", 2),         // 種別コード
        n("code", 1),         // コード区分
        n("client_code", 10), // 依頼人コード
        c("client_name", 40), // 依頼人名
        n("date", 4),         // 支払開始日
        n("bank_no", 4),      // 仕向銀行番号
        c("bank_name", 15),   // 仕向銀行名
        n("branch_no", 3),    // 仕向支店番号
        c("branch_name", 15), // 仕向支店名
        c("dummy", 8),        // ダミー
        n("pay_period", 4),   // 支払期
        c("kind_detail", 13), // ※種類内訳
    ],
};
/// 株式配当金振込 data
static L09_DATA_0: Layout = Layout {
    fields: &[
        n("data_kind", 1),     // データ区分
        n("bank_no", 4),       // 被仕向銀行番号
        c("bank_name", 15),    // 被仕向銀行名
        n("branch_no", 3),     // 被仕向支店番号
        c("branch_name", 15),  // 被仕向支店名
        n("clearing", 4),      // ※交換所番号
        n("acct_kind", 1),     // 預金種目
        n("acct_no", 7),       // 口座番号
        c("name", 30),         // 預金者名
        n("amount", 10),       // 振込金額
        n("new_code", 1),      // 新規コード
        n("recipient_no", 10), // 受取人照会番号
        c("dummy", 19),        // ダミー
    ],
};
/// 株式配当金振込 trailer
static L09_TRAILER: Layout = Layout {
    fields: &[
        n("data_kind", 1), // データ区分
        n("count", 6),     // 合計件数
        n("amount", 12),   // 合計金額
        c("dummy", 101),   // ダミー
    ],
};
/// 株式配当金振込 end
static L09_END: Layout = Layout {
    fields: &[
        n("data_kind", 1), // データ区分
        c("dummy", 119),   // ダミー
    ],
};
/// 年金信託契約に係る年金・一時金給付金振込 header
static L10_HEADER: Layout = Layout {
    fields: &[
        n("data_kind", 1),    // データ区分
        n("type", 2),         // 種別コード
        n("code", 1),         // コード区分
        n("client_code", 10), // 依頼人コード
        c("client_name", 40), // 依頼人名
        n("date", 4),         // 支払開始日
        n("bank_no", 4),      // 仕向銀行番号
        c("bank_name", 15),   // 仕向銀行名
        n("branch_no", 3),    // 仕向支店番号
        c("branch_name", 15), // 仕向支店名
        c("dummy", 8),        // ダミー
        n("pay_period", 4),   // 支払期
        c("kind_detail", 13), // ※種類内訳
    ],
};
/// 年金信託契約に係る年金・一時金給付金振込 data
static L10_DATA_0: Layout = Layout {
    fields: &[
        n("data_kind", 1),     // データ区分
        n("bank_no", 4),       // 被仕向銀行番号
        c("bank_name", 15),    // 被仕向銀行名
        n("branch_no", 3),     // 被仕向支店番号
        c("branch_name", 15),  // 被仕向支店名
        n("clearing", 4),      // ※交換所番号
        n("acct_kind", 1),     // 預金種目
        n("acct_no", 7),       // 口座番号
        c("name", 30),         // 預金者名
        n("amount", 10),       // 振込金額
        n("new_code", 1),      // 新規コード
        n("recipient_no", 20), // 受取人照会番号
        c("dummy", 9),         // ダミー
    ],
};
/// 年金信託契約に係る年金・一時金給付金振込 trailer
static L10_TRAILER: Layout = Layout {
    fields: &[
        n("data_kind", 1), // データ区分
        n("count", 6),     // 合計件数
        n("amount", 12),   // 合計金額
        c("dummy", 101),   // ダミー
    ],
};
/// 年金信託契約に係る年金・一時金給付金振込 end
static L10_END: Layout = Layout {
    fields: &[
        n("data_kind", 1), // データ区分
        c("dummy", 119),   // ダミー
    ],
};
/// 公的年金保険の年金・一時金給付金振込 header
static L11_HEADER: Layout = Layout {
    fields: &[
        n("data_kind", 1),    // データ区分
        n("type", 2),         // 種別コード
        n("code", 1),         // コード区分
        n("client_code", 10), // 依頼人コード
        c("client_name", 40), // 依頼人名
        n("date", 4),         // 支払開始日
        n("bank_no", 4),      // 仕向銀行番号
        c("bank_name", 15),   // 仕向銀行名
        n("branch_no", 3),    // 仕向支店番号
        c("branch_name", 15), // 仕向支店名
        c("dummy", 8),        // ダミー
        n("pay_period", 4),   // 支払期
        c("kind_detail", 13), // ※種類内訳
    ],
};
/// 公的年金保険の年金・一時金給付金振込 data
static L11_DATA_0: Layout = Layout {
    fields: &[
        n("data_kind", 1),     // データ区分
        n("bank_no", 4),       // 被仕向銀行番号
        c("bank_name", 15),    // 被仕向銀行名
        n("branch_no", 3),     // 被仕向支店番号
        c("branch_name", 15),  // 被仕向支店名
        n("clearing", 4),      // ※交換所番号
        n("acct_kind", 1),     // 預金種目
        n("acct_no", 7),       // 口座番号
        c("name", 30),         // 預金者名
        n("amount", 10),       // 振込金額
        n("new_code", 1),      // 新規コード
        n("recipient_no", 20), // 受取人照会番号
        c("dummy", 9),         // ダミー
    ],
};
/// 公的年金保険の年金・一時金給付金振込 trailer
static L11_TRAILER: Layout = Layout {
    fields: &[
        n("data_kind", 1), // データ区分
        n("count", 6),     // 合計件数
        n("amount", 12),   // 合計金額
        c("dummy", 101),   // ダミー
    ],
};
/// 公的年金保険の年金・一時金給付金振込 end
static L11_END: Layout = Layout {
    fields: &[
        n("data_kind", 1), // データ区分
        c("dummy", 119),   // ダミー
    ],
};
/// 医療保険の給付金振込 header
static L12_HEADER: Layout = Layout {
    fields: &[
        n("data_kind", 1),    // データ区分
        n("type", 2),         // 種別コード
        n("code", 1),         // コード区分
        n("client_code", 10), // 依頼人コード
        c("client_name", 40), // 依頼人名
        n("date", 4),         // 支払開始日
        n("bank_no", 4),      // 仕向銀行番号
        c("bank_name", 15),   // 仕向銀行名
        n("branch_no", 3),    // 仕向支店番号
        c("branch_name", 15), // 仕向支店名
        c("dummy", 8),        // ダミー
        n("pay_period", 4),   // 支払期
        c("kind_detail", 13), // ※種類内訳
    ],
};
/// 医療保険の給付金振込 data
static L12_DATA_0: Layout = Layout {
    fields: &[
        n("data_kind", 1),     // データ区分
        n("bank_no", 4),       // 被仕向銀行番号
        c("bank_name", 15),    // 被仕向銀行名
        n("branch_no", 3),     // 被仕向支店番号
        c("branch_name", 15),  // 被仕向支店名
        n("clearing", 4),      // ※交換所番号
        n("acct_kind", 1),     // 預金種目
        n("acct_no", 7),       // 口座番号
        c("name", 30),         // 預金者名
        n("amount", 10),       // 振込金額
        n("new_code", 1),      // 新規コード
        n("recipient_no", 20), // 受取人照会番号
        c("dummy", 9),         // ダミー
    ],
};
/// 医療保険の給付金振込 trailer
static L12_TRAILER: Layout = Layout {
    fields: &[
        n("data_kind", 1), // データ区分
        n("count", 6),     // 合計件数
        n("amount", 12),   // 合計金額
        c("dummy", 101),   // ダミー
    ],
};
/// 医療保険の給付金振込 end
static L12_END: Layout = Layout {
    fields: &[
        n("data_kind", 1), // データ区分
        c("dummy", 119),   // ダミー
    ],
};
/// 振込口座照会（依頼明細） header
static L13_HEADER: Layout = Layout {
    fields: &[
        n("data_kind", 1),    // データ区分
        n("type", 2),         // 種別コード
        n("code", 1),         // コード区分
        n("client_code", 10), // 口座照会依頼人コード
        c("client_name", 40), // 口座照会依頼人名
        n("date", 4),         // 依頼日
        n("bank_no", 4),      // 仕向銀行番号
        c("bank_name", 15),   // ※仕向銀行名
        n("branch_no", 3),    // 仕向支店番号
        c("branch_name", 15), // ※仕向支店名
        n("acct_kind", 1),    // ※預金種目(依頼人)
        n("acct_no", 7),      // ※口座番号(依頼人)
        c("dummy", 17),       // ダミー
    ],
};
/// 振込口座照会（依頼明細） data
static L13_DATA_0: Layout = Layout {
    fields: &[
        n("data_kind", 1),     // データ区分
        n("bank_no", 4),       // 被仕向銀行番号
        c("bank_name", 15),    // ※被仕向銀行名
        n("branch_no", 3),     // 被仕向支店番号
        c("branch_name", 15),  // ※被仕向支店名
        n("clearing", 4),      // ※手形交換所番号
        n("acct_kind", 1),     // 預金種目
        n("acct_no", 7),       // 口座番号
        c("name", 30),         // 受取人名
        n("amount", 10),       // ※振込金額
        n("new_code", 1),      // ※新規コード
        c("cust1", 10),        // ※顧客コード1
        c("cust2", 10),        // ※顧客コード2
        n("transfer_kind", 1), // ※振込指定区分
        c("edi_flag", 1),      // ※識別表示
        c("dummy", 6),         // ダミー
        n("result_code", 1),   // 照会回答コード
    ],
};
/// 振込口座照会（依頼明細） trailer
static L13_TRAILER: Layout = Layout {
    fields: &[
        n("data_kind", 1), // データ区分
        n("count", 6),     // 合計件数
        c("dummy", 113),   // ダミー
    ],
};
/// 振込口座照会（依頼明細） end
static L13_END: Layout = Layout {
    fields: &[
        n("data_kind", 1), // データ区分
        c("dummy", 119),   // ダミー
    ],
};
/// 振込口座照会（処理結果明細） header
static L14_HEADER: Layout = Layout {
    fields: &[
        n("data_kind", 1),    // データ区分
        n("type", 2),         // 種別コード
        n("code", 1),         // コード区分
        n("client_code", 10), // 口座照会依頼人コード
        c("client_name", 40), // 口座照会依頼人名
        n("date", 4),         // 依頼日
        n("bank_no", 4),      // 仕向銀行番号
        c("bank_name", 15),   // ※仕向銀行名
        n("branch_no", 3),    // 仕向支店番号
        c("branch_name", 15), // ※仕向支店名
        n("acct_kind", 1),    // ※預金種目(依頼人)
        n("acct_no", 7),      // ※口座番号(依頼人)
        c("dummy", 17),       // ダミー
    ],
};
/// 振込口座照会（処理結果明細） data
static L14_DATA_0: Layout = Layout {
    fields: &[
        n("data_kind", 1),     // データ区分
        n("bank_no", 4),       // 被仕向銀行番号
        c("bank_name", 15),    // ※被仕向銀行名
        n("branch_no", 3),     // 被仕向支店番号
        c("branch_name", 15),  // ※被仕向支店名
        n("clearing", 4),      // ※手形交換所番号
        n("acct_kind", 1),     // 預金種目
        n("acct_no", 7),       // 口座番号
        c("name", 30),         // 受取人名
        n("amount", 10),       // ※振込金額
        n("new_code", 1),      // ※新規コード
        c("cust1", 10),        // ※顧客コード1
        c("cust2", 10),        // ※顧客コード2
        n("transfer_kind", 1), // ※振込指定区分
        c("edi_flag", 1),      // ※識別表示
        c("dummy", 6),         // ダミー
        n("result_code", 1),   // 照会回答コード
    ],
};
/// 振込口座照会（処理結果明細） trailer
static L14_TRAILER: Layout = Layout {
    fields: &[
        n("data_kind", 1), // データ区分
        n("count", 6),     // 合計件数
        c("dummy", 113),   // ダミー
    ],
};
/// 振込口座照会（処理結果明細） end
static L14_END: Layout = Layout {
    fields: &[
        n("data_kind", 1), // データ区分
        c("dummy", 119),   // ダミー
    ],
};
/// 預金口座振替（依頼明細） header
static L15_HEADER: Layout = Layout {
    fields: &[
        n("data_kind", 1),    // データ区分
        n("type", 2),         // 種別コード
        n("code", 1),         // コード区分
        n("client_code", 10), // 委託者コード
        c("client_name", 40), // 委託者名
        n("date", 4),         // 引落日
        n("bank_no", 4),      // 取引銀行番号
        c("bank_name", 15),   // ※取引銀行名
        n("branch_no", 3),    // 取引支店番号
        c("branch_name", 15), // ※取引支店名
        n("acct_kind", 1),    // 預金種目(委託者)
        n("acct_no", 7),      // 口座番号(委託者)
        c("dummy", 17),       // ダミー
    ],
};
/// 預金口座振替（依頼明細） data
static L15_DATA_0: Layout = Layout {
    fields: &[
        n("data_kind", 1),    // データ区分
        n("bank_no", 4),      // 引落銀行番号
        c("bank_name", 15),   // ※引落銀行名
        n("branch_no", 3),    // 引落支店番号
        c("branch_name", 15), // ※引落支店名
        c("dummy", 4),        // ダミー
        n("acct_kind", 1),    // 預金種目
        n("acct_no", 7),      // 口座番号
        c("name", 30),        // 預金者名
        n("amount", 10),      // 引落金額
        n("new_code", 1),     // 新規コード
        n("cust_no", 20),     // 顧客番号
        n("result_code", 1),  // 振替結果コード
        c("dummy2", 8),       // ダミー
    ],
};
/// 預金口座振替（依頼明細） trailer
static L15_TRAILER: Layout = Layout {
    fields: &[
        n("data_kind", 1),  // データ区分
        n("count", 6),      // 合計件数
        n("amount", 12),    // 合計金額
        n("ok_count", 6),   // 振替済件数
        n("ok_amount", 12), // 振替済金額
        n("ng_count", 6),   // 振替不能件数
        n("ng_amount", 12), // 振替不能金額
        c("dummy", 65),     // ダミー
    ],
};
/// 預金口座振替（依頼明細） end
static L15_END: Layout = Layout {
    fields: &[
        n("data_kind", 1), // データ区分
        c("dummy", 119),   // ダミー
    ],
};
/// 預金口座振替（処理結果明細） header
static L16_HEADER: Layout = Layout {
    fields: &[
        n("data_kind", 1),    // データ区分
        n("type", 2),         // 種別コード
        n("code", 1),         // コード区分
        n("client_code", 10), // 委託者コード
        c("client_name", 40), // 委託者名
        n("date", 4),         // 引落日
        n("bank_no", 4),      // 取引銀行番号
        c("bank_name", 15),   // ※取引銀行名
        n("branch_no", 3),    // 取引支店番号
        c("branch_name", 15), // ※取引支店名
        n("acct_kind", 1),    // 預金種目(委託者)
        n("acct_no", 7),      // 口座番号(委託者)
        c("dummy", 17),       // ダミー
    ],
};
/// 預金口座振替（処理結果明細） data
static L16_DATA_0: Layout = Layout {
    fields: &[
        n("data_kind", 1),    // データ区分
        n("bank_no", 4),      // 引落銀行番号
        c("bank_name", 15),   // ※引落銀行名
        n("branch_no", 3),    // 引落支店番号
        c("branch_name", 15), // ※引落支店名
        c("dummy", 4),        // ダミー
        n("acct_kind", 1),    // 預金種目
        n("acct_no", 7),      // 口座番号
        c("name", 30),        // 預金者名
        n("amount", 10),      // 引落金額
        n("new_code", 1),     // 新規コード
        n("cust_no", 20),     // 顧客番号
        n("result_code", 1),  // 振替結果コード
        c("dummy2", 8),       // ダミー
    ],
};
/// 預金口座振替（処理結果明細） trailer
static L16_TRAILER: Layout = Layout {
    fields: &[
        n("data_kind", 1),  // データ区分
        n("count", 6),      // 合計件数
        n("amount", 12),    // 合計金額
        n("ok_count", 6),   // 振替済件数
        n("ok_amount", 12), // 振替済金額
        n("ng_count", 6),   // 振替不能件数
        n("ng_amount", 12), // 振替不能金額
        c("dummy", 65),     // ダミー
    ],
};
/// 預金口座振替（処理結果明細） end
static L16_END: Layout = Layout {
    fields: &[
        n("data_kind", 1), // データ区分
        c("dummy", 119),   // ダミー
    ],
};
/// 預金口座振替変更通知 header
static L17_HEADER: Layout = Layout {
    fields: &[
        n("data_kind", 1),    // データ区分
        n("type", 2),         // 種別コード
        n("code", 1),         // コード区分
        n("client_code", 10), // 委託者コード
        c("client_name", 40), // 委託者名
        n("date", 4),         // 作成月日
        n("bank_no", 4),      // 取引銀行番号
        c("bank_name", 15),   // ※取引銀行名
        n("branch_no", 3),    // 取引支店番号
        c("branch_name", 15), // ※取引支店名
        n("acct_kind", 1),    // 預金種目(委託者)
        n("acct_no", 7),      // 口座番号(委託者)
        c("dummy", 17),       // ダミー
    ],
};
/// 預金口座振替変更通知 data
static L17_DATA_0: Layout = Layout {
    fields: &[
        n("data_kind", 1),        // データ区分
        n("bank_no", 4),          // 引落銀行番号
        n("branch_no", 3),        // 引落支店番号
        n("acct_kind", 1),        // 預金種目
        n("acct_no", 7),          // 口座番号
        c("name", 30),            // 預金者名
        n("cust_no", 20),         // ※顧客番号
        n("new_branch_no", 3),    // 新引落支店番号
        c("new_branch_name", 15), // 新引落支店名
        n("new_acct_kind", 1),    // 新預金種目
        n("new_acct_no", 7),      // 新口座番号
        c("dummy", 28),           // ダミー
    ],
};
/// 預金口座振替変更通知 trailer
static L17_TRAILER: Layout = Layout {
    fields: &[
        n("data_kind", 1), // データ区分
        n("count", 7),     // 合計件数
        c("dummy", 112),   // ダミー
    ],
};
/// 預金口座振替変更通知 end
static L17_END: Layout = Layout {
    fields: &[
        n("data_kind", 1), // データ区分
        c("dummy", 119),   // ダミー
    ],
};
/// 住宅ローン関係（財形ローン） header
static L18_HEADER: Layout = Layout {
    fields: &[
        n("data_kind", 1),    // データ区分
        n("type", 2),         // 種別コード
        n("code", 1),         // コード区分
        n("client_code", 12), // 会社コード
        c("client_name", 40), // 会社名
        n("bank_no", 4),      // 持出銀行コード
        c("bank_name", 15),   // 持出銀行名
        n("date", 8),         // 作成基準年月日
        c("dummy", 467),      // ダミー
    ],
};
/// 住宅ローン関係（財形ローン） data
static L18_DATA_0: Layout = Layout {
    fields: &[
        n("data_kind", 1),           // データ区分
        n("office_code", 5),         // ※事業所コード
        n("dept_code", 10),          // ※所属コード
        n("borrower_code", 10),      // ※借入者コード
        c("borrower_name", 30),      // 借入者氏名
        n("birth_date", 8),          // ※借入者生年月日
        n("retire_date", 8),         // ※定年退職日
        n("group_life", 1),          // ※団体生命加入コード
        n("loan_bank_no", 4),        // 貸出銀行コード
        c("loan_bank_name", 15),     // 貸出銀行名
        n("loan_branch_no", 3),      // 貸出支店コード
        c("loan_branch_name", 15),   // 貸出支店名
        n("loan_code", 23),          // 貸出コード
        n("debit_branch_no", 3),     // ※引落し支店コード
        c("debit_branch_name", 15),  // ※引落し支店名
        n("acct_kind", 1),           // ※預金種目
        n("acct_no", 7),             // ※口座番号
        n("guarantor_code", 12),     // ※保証会社コード
        c("guarantor_name", 40),     // ※保証会社名
        n("guarantee_code", 10),     // ※保証コード
        c("purpose", 2),             // ※資金使途
        n("loan_date", 8),           // 貸出日
        n("final_date", 8),          // 最終期限
        n("loan_amount", 9),         // 貸出金額
        n("loan_amount_monthly", 9), // (毎月返済分)
        n("loan_amount_bonus", 9),   // (ボーナス返済分)
        n("loan_amount_other", 9),   // ※(その他返済分)
        n("loan_months", 3),         // 貸出期間
        n("initial_rate", 7),        // ※当初約定金利
        n("current_rate", 7),        // 現行約定金利
        n("repay_day", 2),           // 約定返済日
        n("first_repay_date", 8),    // 初回約定返済年月日
        n("bonus_months", 4),        // ボーナス返済月
        n("repay_method", 2),        // 返済方式
        n("balance", 9),             // 残高
        n("balance_monthly", 9),     // (毎月返済分)
        n("balance_bonus", 9),       // (ボーナス返済分)
        n("balance_other", 9),       // ※(その他返済分)
        n("data_type", 1),           // データ種類
        n("repay_date", 8),          // 約定返済年月日
        n("collect_type", 1),        // 回収種類
        n("collect_amount", 9),      // 回収(延滞)金額
        n("principal", 9),           // 回収(延滞)元金
        n("principal_monthly", 9),   // (毎月返済分)
        n("principal_bonus", 9),     // (ボーナス返済分)
        n("principal_other", 9),     // ※(その他返済分)
        n("interest", 9),            // 回収(延滞)利息
        n("interest_monthly", 9),    // (毎月返済分)
        n("interest_bonus", 9),      // (ボーナス返済分)
        n("interest_other", 9),      // ※(その他返済分)
        n("other_amount", 9),        // その他回収金額
        n("collect_date", 8),        // 回収年月日
        c("subsidy_method", 2),      // ※利子補給方式
        n("subsidy_amount", 9),      // ※利子補給金額
        n("subsidy_monthly", 9),     // ※(毎月返済分)
        n("subsidy_bonus", 9),       // ※(ボーナス返済分)
        n("subsidy_rate", 7),        // ※利子補給率
        c("dummy", 62),              // ダミー
    ],
};
/// 住宅ローン関係（財形ローン） trailer
static L18_TRAILER: Layout = Layout {
    fields: &[
        n("data_kind", 1),       // データ区分
        n("count", 7),           // 総データ件数
        n("balance_total", 12),  // 残高合計
        n("collect_count", 7),   // 回収(データ種類「1」)件数
        n("collect_amount", 12), // 回収(データ種類「1」)金額
        n("plan_count", 7),      // 回収予定(データ種類「2」)件数
        n("plan_amount", 12),    // 回収予定(データ種類「2」)金額
        n("overdue_count", 7),   // 延滞(データ種類「3」)件数
        n("overdue_amount", 12), // 延滞(データ種類「3」)金額
        n("other_count", 7),     // その他(データ種類「4」)件数
        n("other_amount", 12),   // その他(データ種類「4」)金額
        c("dummy", 454),         // ダミー
    ],
};
/// 住宅ローン関係（財形ローン） end
static L18_END: Layout = Layout {
    fields: &[
        n("data_kind", 1), // データ区分
        c("dummy", 549),   // ダミー
    ],
};
/// 住宅ローン関係（その他） header
static L19_HEADER: Layout = Layout {
    fields: &[
        n("data_kind", 1),    // データ区分
        n("type", 2),         // 種別コード
        n("code", 1),         // コード区分
        n("client_code", 12), // 会社コード
        c("client_name", 40), // 会社名
        n("bank_no", 4),      // 持出銀行コード
        c("bank_name", 15),   // 持出銀行名
        n("date", 8),         // 作成基準年月日
        c("dummy", 467),      // ダミー
    ],
};
/// 住宅ローン関係（その他） data
static L19_DATA_0: Layout = Layout {
    fields: &[
        n("data_kind", 1),           // データ区分
        n("office_code", 5),         // ※事業所コード
        n("dept_code", 10),          // ※所属コード
        n("borrower_code", 10),      // ※借入者コード
        c("borrower_name", 30),      // 借入者氏名
        n("birth_date", 8),          // ※借入者生年月日
        n("retire_date", 8),         // ※定年退職日
        n("group_life", 1),          // ※団体生命加入コード
        n("loan_bank_no", 4),        // 貸出銀行コード
        c("loan_bank_name", 15),     // 貸出銀行名
        n("loan_branch_no", 3),      // 貸出支店コード
        c("loan_branch_name", 15),   // 貸出支店名
        n("loan_code", 23),          // 貸出コード
        n("debit_branch_no", 3),     // ※引落し支店コード
        c("debit_branch_name", 15),  // ※引落し支店名
        n("acct_kind", 1),           // ※預金種目
        n("acct_no", 7),             // ※口座番号
        n("guarantor_code", 12),     // ※保証会社コード
        c("guarantor_name", 40),     // ※保証会社名
        n("guarantee_code", 10),     // ※保証コード
        c("purpose", 2),             // ※資金使途
        n("loan_date", 8),           // 貸出日
        n("final_date", 8),          // 最終期限
        n("loan_amount", 9),         // 貸出金額
        n("loan_amount_monthly", 9), // (毎月返済分)
        n("loan_amount_bonus", 9),   // (ボーナス返済分)
        n("loan_amount_other", 9),   // ※(その他返済分)
        n("loan_months", 3),         // 貸出期間
        n("initial_rate", 7),        // ※当初約定金利
        n("current_rate", 7),        // 現行約定金利
        n("repay_day", 2),           // 約定返済日
        n("first_repay_date", 8),    // 初回約定返済年月日
        n("bonus_months", 4),        // ボーナス返済月
        n("repay_method", 2),        // 返済方式
        n("balance", 9),             // 残高
        n("balance_monthly", 9),     // (毎月返済分)
        n("balance_bonus", 9),       // (ボーナス返済分)
        n("balance_other", 9),       // ※(その他返済分)
        n("data_type", 1),           // データ種類
        n("repay_date", 8),          // 約定返済年月日
        n("collect_type", 1),        // 回収種類
        n("collect_amount", 9),      // 回収(延滞)金額
        n("principal", 9),           // 回収(延滞)元金
        n("principal_monthly", 9),   // (毎月返済分)
        n("principal_bonus", 9),     // (ボーナス返済分)
        n("principal_other", 9),     // ※(その他返済分)
        n("interest", 9),            // 回収(延滞)利息
        n("interest_monthly", 9),    // (毎月返済分)
        n("interest_bonus", 9),      // (ボーナス返済分)
        n("interest_other", 9),      // ※(その他返済分)
        n("other_amount", 9),        // その他回収金額
        n("collect_date", 8),        // 回収年月日
        c("subsidy_method", 2),      // ※利子補給方式
        n("subsidy_amount", 9),      // ※利子補給金額
        n("subsidy_monthly", 9),     // ※(毎月返済分)
        n("subsidy_bonus", 9),       // ※(ボーナス返済分)
        n("subsidy_rate", 7),        // ※利子補給率
        c("dummy", 62),              // ダミー
    ],
};
/// 住宅ローン関係（その他） trailer
static L19_TRAILER: Layout = Layout {
    fields: &[
        n("data_kind", 1),       // データ区分
        n("count", 7),           // 総データ件数
        n("balance_total", 12),  // 残高合計
        n("collect_count", 7),   // 回収(データ種類「1」)件数
        n("collect_amount", 12), // 回収(データ種類「1」)金額
        n("plan_count", 7),      // 回収予定(データ種類「2」)件数
        n("plan_amount", 12),    // 回収予定(データ種類「2」)金額
        n("overdue_count", 7),   // 延滞(データ種類「3」)件数
        n("overdue_amount", 12), // 延滞(データ種類「3」)金額
        n("other_count", 7),     // その他(データ種類「4」)件数
        n("other_amount", 12),   // その他(データ種類「4」)金額
        c("dummy", 454),         // ダミー
    ],
};
/// 住宅ローン関係（その他） end
static L19_END: Layout = Layout {
    fields: &[
        n("data_kind", 1), // データ区分
        c("dummy", 549),   // ダミー
    ],
};
/// 財形貯蓄関係（依頼明細） header
static L20_HEADER: Layout = Layout {
    fields: &[
        n("data_kind", 1),            // データ区分
        n("type", 1),                 // 種別コード
        n("tsumitate_kbn_salary", 1), // 積立区分 給与
        n("tsumitate_kbn_bonus", 1),  // 積立区分 賞与
        n("tsumitate_kbn_other1", 1), // 積立区分 その他(1)
        n("tsumitate_kbn_other2", 1), // 積立区分 その他(2)
        n("code", 1),                 // コード区分
        n("client_code", 12),         // 委託先コード
        c("client_name", 40),         // 委託先名
        n("bank_no", 4),              // 取りまとめ銀行コード
        c("bank_name", 15),           // 取りまとめ銀行名
        n("branch_no", 3),            // 取りまとめ支店コード
        c("branch_name", 15),         // 取りまとめ支店名
        n("direction_code", 1),       // 仕向コード
        n("base_date", 8),            // 作成基準年月日
        n("tsumitate_date", 8),       // 積立年月日
        c("dummy", 187),              // ダミー
    ],
};
/// 財形貯蓄関係（依頼明細） data
static L20_DATA_0: Layout = Layout {
    fields: &[
        n("data_kind", 1),         // データ区分
        n("office_code", 5),       // 事業所コード
        n("dept_code", 10),        // 所属コード
        n("member_code", 10),      // 加入者コード
        c("purpose_kbn", 1),       // 目的区分
        c("member_name", 30),      // 加入者氏名
        n("bank_no", 4),           // 預入銀行コード
        c("bank_name", 15),        // 預入銀行名
        n("branch_no", 3),         // 預入支店コード
        c("branch_name", 15),      // 預入支店名
        n("deposit_kbn", 1),       // 入金区分
        n("new_kbn", 1),           // 新規区分
        n("change_kbn", 1),        // 変更区分
        n("deposit_kind", 1),      // ◎預金種目
        n("acct_no", 10),          // ※◎○(注)口座番号
        n("start_date", 8),        // 積立(預入)開始年月日
        n("sex", 1),               // ※性別
        n("birth_date", 8),        // 生年月日
        n("salary_amount", 9),     // 給与積立額
        n("bonus_amount", 9),      // 賞与積立額
        n("other1_amount", 9),     // その他積立額(1)
        n("other2_amount", 9),     // その他積立額(2)
        c("dummy", 9),             // ダミー
        n("maturity_date", 8),     // ◎○満期日
        n("last_deposit_date", 8), // 〇最終預入日
        n("period_months", 3),     // 〇積立期間
        n("tax_kbn", 1),           // ◎税区分
        n("ipn_kbn", 1),           // 一般・住宅・年金区分
        c("dummy2", 8),            // ダミー
        c("dummy3", 8),            // ダミー
        n("tax_free_limit", 9),    // ◎口座の非課税限度額
        c("dummy4", 2),            // ダミー
        c("dummy5", 9),            // ダミー
        c("dummy6", 9),            // ダミー
        c("dummy7", 9),            // ダミー
        n("pension_bank_no", 4),   // ※年金受給銀行コード
        n("pension_branch_no", 3), // ※年金受給支店コード
        n("pension_acct_kind", 1), // ※年金受給預金種目
        n("pension_acct_no", 10),  // ※年金受給(注1)口座番号
        n("new_office_code", 5),   // 新事業所コード
        n("new_dept_code", 10),    // 新所属コード
        n("pay_start_date", 8),    // 支払開始年月日
        n("pay_times", 2),         // 支払方法(回数)(注2)
        n("pay_end_ym", 6),        // ※支払終了年月(注3)
        n("pay_pattern", 1),       // ※支払パターン(注3)
        c("pay_interval", 1),      // 支払間隔
        n("increase_rate", 2),     // ※逓増定率(注3)
        c("dummy8", 2),            // ダミー
    ],
};
/// 財形貯蓄関係（依頼明細） trailer
static L20_TRAILER: Layout = Layout {
    fields: &[
        n("data_kind", 1),     // データ区分
        n("count", 7),         // データ件数
        c("dummy", 12),        // ダミー
        n("salary_total", 12), // 給与積立額合計
        n("bonus_total", 12),  // 賞与積立額合計
        n("other1_total", 12), // その他積立額合計(1)
        n("other2_total", 12), // その他積立額合計(2)
        c("dummy2", 232),      // ダミー
    ],
};
/// 財形貯蓄関係（依頼明細） end
static L20_END: Layout = Layout {
    fields: &[
        n("data_kind", 1), // データ区分
        c("dummy", 299),   // ダミー
    ],
};
/// 財形貯蓄関係（処理結果明細：積立内容） header
static L21_HEADER: Layout = Layout {
    fields: &[
        n("data_kind", 1),            // データ区分
        n("type", 1),                 // 種別コード
        n("tsumitate_kbn_salary", 1), // 積立区分 給与
        n("tsumitate_kbn_bonus", 1),  // 積立区分 賞与
        n("tsumitate_kbn_other1", 1), // 積立区分 その他(1)
        n("tsumitate_kbn_other2", 1), // 積立区分 その他(2)
        n("code", 1),                 // コード区分
        n("client_code", 12),         // 委託先コード
        c("client_name", 40),         // 委託先名
        n("bank_no", 4),              // 取りまとめ銀行コード
        c("bank_name", 15),           // 取りまとめ銀行名
        n("branch_no", 3),            // 取りまとめ支店コード
        c("branch_name", 15),         // 取りまとめ支店名
        n("direction_code", 1),       // 仕向コード
        n("base_date", 8),            // 作成基準年月日
        n("tsumitate_date", 8),       // 積立年月日
        c("dummy", 187),              // ダミー
    ],
};
/// 財形貯蓄関係（処理結果明細：積立内容） data
static L21_DATA_0: Layout = Layout {
    fields: &[
        n("data_kind", 1),            // データ区分
        n("office_code", 5),          // 事業所コード
        n("dept_code", 10),           // 所属コード
        n("member_code", 10),         // 加入者コード
        c("purpose_kbn", 1),          // 目的区分
        c("member_name", 30),         // 加入者氏名
        n("bank_no", 4),              // 預入銀行コード
        c("bank_name", 15),           // 預入銀行名
        n("branch_no", 3),            // 預入支店コード
        c("branch_name", 15),         // 預入支店名
        c("dummy", 1),                // ダミー
        c("dummy2", 1),               // ダミー
        c("dummy3", 1),               // ダミー
        n("deposit_kind", 1),         // ◎預金種目
        n("acct_no", 10),             // ※◎○(注1)口座番号
        n("new_date", 8),             // 新規日
        n("sex", 1),                  // ※性別
        n("birth_date", 8),           // ※生年月日
        n("salary_amount", 9),        // 給与積立額
        n("bonus_amount", 9),         // 賞与積立額
        n("other1_amount", 9),        // その他積立額(1)
        n("other2_amount", 9),        // その他積立額(2)
        n("balance", 9),              // ◎当月残高
        n("maturity_date", 8),        // ◎○満期日
        n("last_deposit_date", 8),    // 〇最終預入日
        n("period_months", 3),        // 〇積立期間
        n("tax_kbn", 1),              // ◎税区分
        n("ipn_kbn", 1),              // 一般・住宅・年金区分
        c("dummy4", 8),               // ダミー
        c("dummy5", 8),               // ダミー
        n("tax_free_limit", 9),       // ◎口座の非課税限度額
        c("subsidy_kbn", 2),          // ※利子補給区分
        n("subsidy_base_balance", 9), // ※利子補給対象残高
        n("subsidy_amount", 9),       // 今回利子補給金
        n("subsidy_cumulative", 9),   // ※利子補給金累計額
        c("dummy6", 4),               // ダミー
        c("dummy7", 3),               // ダミー
        c("dummy8", 1),               // ダミー
        c("dummy9", 10),              // ダミー
        c("dummy10", 5),              // ダミー
        c("dummy11", 10),             // ダミー
        n("pay_start_date", 8),       // 支払開始年月日
        n("pay_times", 2),            // 支払方法(回数)(注2)
        n("pay_end_ym", 6),           // ※支払終了年月(注3)
        n("pay_pattern", 1),          // ※支払パターン(注3)
        c("pay_interval", 1),         // 支払間隔
        n("increase_rate", 2),        // ※逓増定率(注3)
        c("dummy12", 2),              // ダミー
    ],
};
/// 財形貯蓄関係（処理結果明細：積立内容） trailer
static L21_TRAILER: Layout = Layout {
    fields: &[
        n("data_kind", 1),                   // データ区分
        n("count", 7),                       // データ件数
        n("balance_total", 12),              // 当月残高合計
        n("salary_total", 12),               // 給与積立額合計
        n("bonus_total", 12),                // 賞与積立額合計
        n("other1_total", 12),               // その他積立額合計(1)
        n("other2_total", 12),               // その他積立額合計(2)
        n("subsidy_base_balance_total", 12), // ※利子補給対象残高合計
        n("subsidy_amount_total", 12),       // 今回利子補給金合計
        n("subsidy_cumulative_total", 12),   // ※利子補給金累計額合計
        c("dummy", 196),                     // ダミー
    ],
};
/// 財形貯蓄関係（処理結果明細：積立内容） end
static L21_END: Layout = Layout {
    fields: &[
        n("data_kind", 1), // データ区分
        c("dummy", 299),   // ダミー
    ],
};
/// 財形貯蓄関係（処理結果明細：利子補給） header
static L22_HEADER: Layout = Layout {
    fields: &[
        n("data_kind", 1),            // データ区分
        n("type", 1),                 // 種別コード
        n("tsumitate_kbn_salary", 1), // 積立区分 給与
        n("tsumitate_kbn_bonus", 1),  // 積立区分 賞与
        n("tsumitate_kbn_other1", 1), // 積立区分 その他(1)
        n("tsumitate_kbn_other2", 1), // 積立区分 その他(2)
        n("code", 1),                 // コード区分
        n("client_code", 12),         // 委託先コード
        c("client_name", 40),         // 委託先名
        n("bank_no", 4),              // 取りまとめ銀行コード
        c("bank_name", 15),           // 取りまとめ銀行名
        n("branch_no", 3),            // 取りまとめ支店コード
        c("branch_name", 15),         // 取りまとめ支店名
        n("direction_code", 1),       // 仕向コード
        n("base_date", 8),            // 作成基準年月日
        n("tsumitate_date", 8),       // 積立年月日
        c("dummy", 187),              // ダミー
    ],
};
/// 財形貯蓄関係（処理結果明細：利子補給） data
static L22_DATA_0: Layout = Layout {
    fields: &[
        n("data_kind", 1),            // データ区分
        n("office_code", 5),          // 事業所コード
        n("dept_code", 10),           // 所属コード
        n("member_code", 10),         // 加入者コード
        c("purpose_kbn", 1),          // 目的区分
        c("member_name", 30),         // 加入者氏名
        n("bank_no", 4),              // 預入銀行コード
        c("bank_name", 15),           // 預入銀行名
        n("branch_no", 3),            // 預入支店コード
        c("branch_name", 15),         // 預入支店名
        c("dummy", 1),                // ダミー
        c("dummy2", 1),               // ダミー
        c("dummy3", 1),               // ダミー
        n("deposit_kind", 1),         // ◎預金種目
        n("acct_no", 10),             // ※◎○(注1)口座番号
        n("new_date", 8),             // 新規日
        n("sex", 1),                  // ※性別
        n("birth_date", 8),           // ※生年月日
        n("salary_amount", 9),        // 給与積立額
        n("bonus_amount", 9),         // 賞与積立額
        n("other1_amount", 9),        // その他積立額(1)
        n("other2_amount", 9),        // その他積立額(2)
        n("balance", 9),              // ◎当月残高
        n("maturity_date", 8),        // ◎○満期日
        n("last_deposit_date", 8),    // 〇最終預入日
        n("period_months", 3),        // 〇積立期間
        n("tax_kbn", 1),              // ◎税区分
        n("ipn_kbn", 1),              // 一般・住宅・年金区分
        c("dummy4", 8),               // ダミー
        c("dummy5", 8),               // ダミー
        n("tax_free_limit", 9),       // ◎口座の非課税限度額
        c("subsidy_kbn", 2),          // ※利子補給区分
        n("subsidy_base_balance", 9), // ※利子補給対象残高
        n("subsidy_amount", 9),       // 今回利子補給金
        n("subsidy_cumulative", 9),   // ※利子補給金累計額
        c("dummy6", 4),               // ダミー
        c("dummy7", 3),               // ダミー
        c("dummy8", 1),               // ダミー
        c("dummy9", 10),              // ダミー
        c("dummy10", 5),              // ダミー
        c("dummy11", 10),             // ダミー
        n("pay_start_date", 8),       // 支払開始年月日
        n("pay_times", 2),            // 支払方法(回数)(注2)
        n("pay_end_ym", 6),           // ※支払終了年月(注3)
        n("pay_pattern", 1),          // ※支払パターン(注3)
        c("pay_interval", 1),         // 支払間隔
        n("increase_rate", 2),        // ※逓増定率(注3)
        c("dummy12", 2),              // ダミー
    ],
};
/// 財形貯蓄関係（処理結果明細：利子補給） trailer
static L22_TRAILER: Layout = Layout {
    fields: &[
        n("data_kind", 1),                   // データ区分
        n("count", 7),                       // データ件数
        n("balance_total", 12),              // 当月残高合計
        n("salary_total", 12),               // 給与積立額合計
        n("bonus_total", 12),                // 賞与積立額合計
        n("other1_total", 12),               // その他積立額合計(1)
        n("other2_total", 12),               // その他積立額合計(2)
        n("subsidy_base_balance_total", 12), // ※利子補給対象残高合計
        n("subsidy_amount_total", 12),       // 今回利子補給金合計
        n("subsidy_cumulative_total", 12),   // ※利子補給金累計額合計
        c("dummy", 196),                     // ダミー
    ],
};
/// 財形貯蓄関係（処理結果明細：利子補給） end
static L22_END: Layout = Layout {
    fields: &[
        n("data_kind", 1), // データ区分
        c("dummy", 299),   // ダミー
    ],
};
/// 財形貯蓄関係（処理結果明細：その他） header
static L23_HEADER: Layout = Layout {
    fields: &[
        n("data_kind", 1),            // データ区分
        n("type", 1),                 // 種別コード
        n("tsumitate_kbn_salary", 1), // 積立区分 給与
        n("tsumitate_kbn_bonus", 1),  // 積立区分 賞与
        n("tsumitate_kbn_other1", 1), // 積立区分 その他(1)
        n("tsumitate_kbn_other2", 1), // 積立区分 その他(2)
        n("code", 1),                 // コード区分
        n("client_code", 12),         // 委託先コード
        c("client_name", 40),         // 委託先名
        n("bank_no", 4),              // 取りまとめ銀行コード
        c("bank_name", 15),           // 取りまとめ銀行名
        n("branch_no", 3),            // 取りまとめ支店コード
        c("branch_name", 15),         // 取りまとめ支店名
        n("direction_code", 1),       // 仕向コード
        n("base_date", 8),            // 作成基準年月日
        n("tsumitate_date", 8),       // 積立年月日
        c("dummy", 187),              // ダミー
    ],
};
/// 財形貯蓄関係（処理結果明細：その他） data
static L23_DATA_0: Layout = Layout {
    fields: &[
        n("data_kind", 1),            // データ区分
        n("office_code", 5),          // 事業所コード
        n("dept_code", 10),           // 所属コード
        n("member_code", 10),         // 加入者コード
        c("purpose_kbn", 1),          // 目的区分
        c("member_name", 30),         // 加入者氏名
        n("bank_no", 4),              // 預入銀行コード
        c("bank_name", 15),           // 預入銀行名
        n("branch_no", 3),            // 預入支店コード
        c("branch_name", 15),         // 預入支店名
        c("dummy", 1),                // ダミー
        c("dummy2", 1),               // ダミー
        c("dummy3", 1),               // ダミー
        n("deposit_kind", 1),         // ◎預金種目
        n("acct_no", 10),             // ※◎○(注1)口座番号
        n("new_date", 8),             // 新規日
        n("sex", 1),                  // ※性別
        n("birth_date", 8),           // ※生年月日
        n("salary_amount", 9),        // 給与積立額
        n("bonus_amount", 9),         // 賞与積立額
        n("other1_amount", 9),        // その他積立額(1)
        n("other2_amount", 9),        // その他積立額(2)
        n("balance", 9),              // ◎当月残高
        n("maturity_date", 8),        // ◎○満期日
        n("last_deposit_date", 8),    // 〇最終預入日
        n("period_months", 3),        // 〇積立期間
        n("tax_kbn", 1),              // ◎税区分
        n("ipn_kbn", 1),              // 一般・住宅・年金区分
        c("dummy4", 8),               // ダミー
        c("dummy5", 8),               // ダミー
        n("tax_free_limit", 9),       // ◎口座の非課税限度額
        c("subsidy_kbn", 2),          // ※利子補給区分
        n("subsidy_base_balance", 9), // ※利子補給対象残高
        n("subsidy_amount", 9),       // 今回利子補給金
        n("subsidy_cumulative", 9),   // ※利子補給金累計額
        c("dummy6", 4),               // ダミー
        c("dummy7", 3),               // ダミー
        c("dummy8", 1),               // ダミー
        c("dummy9", 10),              // ダミー
        c("dummy10", 5),              // ダミー
        c("dummy11", 10),             // ダミー
        n("pay_start_date", 8),       // 支払開始年月日
        n("pay_times", 2),            // 支払方法(回数)(注2)
        n("pay_end_ym", 6),           // ※支払終了年月(注3)
        n("pay_pattern", 1),          // ※支払パターン(注3)
        c("pay_interval", 1),         // 支払間隔
        n("increase_rate", 2),        // ※逓増定率(注3)
        c("dummy12", 2),              // ダミー
    ],
};
/// 財形貯蓄関係（処理結果明細：その他） trailer
static L23_TRAILER: Layout = Layout {
    fields: &[
        n("data_kind", 1),                   // データ区分
        n("count", 7),                       // データ件数
        n("balance_total", 12),              // 当月残高合計
        n("salary_total", 12),               // 給与積立額合計
        n("bonus_total", 12),                // 賞与積立額合計
        n("other1_total", 12),               // その他積立額合計(1)
        n("other2_total", 12),               // その他積立額合計(2)
        n("subsidy_base_balance_total", 12), // ※利子補給対象残高合計
        n("subsidy_amount_total", 12),       // 今回利子補給金合計
        n("subsidy_cumulative_total", 12),   // ※利子補給金累計額合計
        c("dummy", 196),                     // ダミー
    ],
};
/// 財形貯蓄関係（処理結果明細：その他） end
static L23_END: Layout = Layout {
    fields: &[
        n("data_kind", 1), // データ区分
        c("dummy", 299),   // ダミー
    ],
};
/// 借入金計算書 header
static L24_HEADER: Layout = Layout {
    fields: &[
        n("data_kind", 1),    // データ区分
        n("type", 2),         // 種別コード
        n("code", 1),         // コード区分
        n("created_date", 6), // 作成日
        n("notice_date", 6),  // ※連絡日
        n("notice_seq", 2),   // ※連絡回数
        n("period_from", 6),  // 勘定日(自)
        n("period_to", 6),    // 勘定日(至)
        n("bank_no", 4),      // 銀行コード
        c("bank_name", 15),   // 銀行名
        n("branch_no", 3),    // 支店コード
        c("branch_name", 15), // 支店名
        n("client_code", 10), // 会社コード
        c("client_name", 50), // 会社名
        c("dummy", 123),      // ダミー
    ],
};
/// 借入金計算書 data loan
static L24_DATA_0: Layout = Layout {
    fields: &[
        n("data_kind", 1),           // データ区分
        n("record_kind", 2),         // データ・レコード区分
        n("cancel_code", 1),         // 取消コード
        n("ident_no", 20),           // 識別番号
        n("account_date", 6),        // 勘定日
        n("value_date", 6),          // 起算日
        n("txn_kind", 2),            // 取引種類
        n("amount", 15),             // 取引金額
        n("balance_after", 15),      // 取引後金額
        n("interest_kbn", 1),        // 利息(1)徴求・戻し区分
        n("interest_amount", 13),    // 利息(2)利息金額
        n("rate_kbn", 1),            // 利息(3)利率区分
        n("rate", 7),                // 利息(4)利率
        n("days", 5),                // 利息(5)日数
        n("interest_from", 6),       // 利息(6)始期
        n("interest_to", 6),         // 利息(7)終期
        n("interest_principal", 15), // 利息(8)利息計算元本
        n("due_date", 6),            // 期日
        n("interest_io_date", 6),    // ※利息入出金日
        n("interest_acct_dummy", 3), // ※利息入出金口座(1)ダミー
        n("interest_acct_kind", 1),  // ※利息入出金口座(2)預金種目
        n("interest_acct_no", 10),   // ※利息入出金口座(3)口座番号（注）
        n("stamp_fee", 8),           // ※印紙代
        n("other_fee", 8),           // ※その他手数料
        c("dummy", 86),              // ダミー
    ],
};
/// 借入金計算書 data discount_detail
static L24_DATA_1: Layout = Layout {
    fields: &[
        n("data_kind", 1),          // データ区分
        n("record_kind", 2),        // データ・レコード区分
        c("dummy", 1),              // ダミー
        n("ident_no", 20),          // 識別番号
        c("dummy2", 6),             // ダミー
        c("dummy3", 6),             // ダミー
        n("txn_kind", 2),           // 取引種類
        n("bill_amount", 15),       // 手形金額
        c("dummy4", 15),            // ダミー
        n("discount_kbn", 1),       // 割引料(1)徴求・戻し区分
        n("discount_amount", 13),   // 割引料(2)割引料金額
        c("dummy5", 1),             // 割引料(3)ダミー
        n("discount_rate", 7),      // 割引料(4)割引料率
        n("days", 5),               // 割引料(5)日数
        c("dummy6", 27),            // 割引料(6)ダミー
        n("due_date", 6),           // 期日
        c("dummy7", 6),             // ダミー
        c("dummy8", 14),            // ダミー
        n("collection_fee", 8),     // ※取立手数料
        c("dummy9", 8),             // ダミー
        c("dummy10", 4),            // ダミー
        n("payer_bank_no", 4),      // ※支払銀行コード
        c("payer_bank_name", 15),   // ※支払銀行名
        n("payer_branch_no", 3),    // ※支払支店コード
        c("payer_branch_name", 15), // ※支払支店名
        c("payer_name", 40),        // ※支払人名
        c("dummy11", 5),            // ダミー
    ],
};
/// 借入金計算書 data discount_total
static L24_DATA_2: Layout = Layout {
    fields: &[
        n("data_kind", 1),            // データ区分
        n("record_kind", 2),          // データ・レコード区分
        c("cancel_code", 1),          // 取消コード（注1）
        n("dummy", 20),               // ダミー
        n("account_date", 6),         // 勘定日
        n("value_date", 6),           // 起算日
        n("txn_kind", 2),             // 取引種類
        n("bill_amount", 15),         // 手形金額
        c("dummy2", 15),              // ダミー
        n("discount_kbn", 1),         // 割引料合計(1)徴求・戻し区分
        n("discount_total", 13),      // 割引料合計(2)割引料合計金額
        c("dummy3", 40),              // 割引料合計(3)ダミー
        c("dummy4", 6),               // ダミー
        n("discount_io_date", 6),     // ※割引料等入出金日
        n("discount_acct_dummy", 3),  // ※割引料等入出金口座(1)ダミー
        n("discount_acct_kind", 1),   // ※割引料等入出金口座(2)預金種目
        n("discount_acct_no", 10),    // ※割引料等入出金口座(3)口座番号（注）
        n("collection_fee_total", 8), // ※取立手数料合計
        c("dummy5", 8),               // ダミー
        n("bill_count", 4),           // ※手形枚数
        c("dummy6", 82),              // ダミー
    ],
};
/// 借入金計算書 trailer
static L24_TRAILER: Layout = Layout {
    fields: &[
        n("data_kind", 1), // データ区分
        n("count", 6),     // データ・レコード件数
        c("dummy", 243),   // ダミー
    ],
};
/// 借入金計算書 end
static L24_END: Layout = Layout {
    fields: &[
        n("data_kind", 1), // データ区分
        c("dummy", 249),   // ダミー
    ],
};
/// 借入金残高明細 header
static L25_HEADER: Layout = Layout {
    fields: &[
        n("data_kind", 1),    // データ区分
        n("type", 2),         // 種別コード
        n("code", 1),         // コード区分
        n("created_date", 6), // 作成日
        n("notice_date", 6),  // ※連絡日
        n("notice_seq", 2),   // ※連絡回数
        c("dummy", 6),        // ダミー
        c("dummy2", 6),       // ダミー
        n("bank_no", 4),      // 銀行コード
        c("bank_name", 15),   // 銀行名
        n("branch_no", 3),    // 支店コード
        c("branch_name", 15), // 支店名
        n("client_code", 10), // 会社コード
        c("client_name", 50), // 会社名
        c("dummy3", 123),     // ダミー
    ],
};
/// 借入金残高明細 data loan
static L25_DATA_0: Layout = Layout {
    fields: &[
        n("data_kind", 1),            // データ区分
        n("record_kind", 2),          // データ・レコード区分
        c("dummy", 1),                // ダミー
        n("ident_no", 20),            // 識別番号
        n("base_date", 6),            // 基準日
        n("base_time", 4),            // ※基準時刻
        n("branch_no", 3),            // 支店コード
        c("dummy2", 1),               // ダミー
        n("balance", 15),             // 残高
        c("dummy3", 15),              // ダミー
        n("interest_collect_kbn", 1), // 利息徴求区分
        c("dummy4", 13),              // ダミー
        n("rate_kbn", 1),             // 利率区分
        n("rate", 7),                 // 利率
        c("dummy5", 32),              // ダミー
        n("due_date", 6),             // 期日
        n("initial_date", 6),         // ※当初借入日
        n("initial_amount", 15),      // ※当初借入額
        n("next_repay_date", 6),      // ※次回返済日
        n("next_repay_amount", 15),   // ※次回返済額
        c("dummy6", 80),              // ダミー
    ],
};
/// 借入金残高明細 data discount
static L25_DATA_1: Layout = Layout {
    fields: &[
        n("data_kind", 1),            // データ区分
        n("record_kind", 2),          // データ・レコード区分
        c("dummy", 1),                // ダミー
        n("ident_no", 20),            // 識別番号
        n("base_date", 6),            // 基準日
        n("base_time", 4),            // ※基準時刻
        n("branch_no", 3),            // 支店コード
        c("dummy2", 1),               // ダミー
        n("bill_amount", 15),         // 手形金額
        c("dummy3", 15),              // ダミー
        n("discount_collect_kbn", 1), // 割引料徴求区分
        c("dummy4", 13),              // ダミー
        c("dummy5", 1),               // ダミー
        n("discount_rate", 7),        // 割引料率
        c("dummy6", 32),              // ダミー
        n("due_date", 6),             // 期日
        n("discount_date", 6),        // 割引日
        c("dummy7", 34),              // ダミー
        n("payer_bank_no", 4),        // ※支払銀行コード
        c("payer_bank_name", 15),     // ※支払銀行名
        n("payer_branch_no", 3),      // ※支払支店コード
        c("payer_branch_name", 15),   // ※支払支店名
        c("payer_name", 40),          // ※支払人名
        c("dummy8", 5),               // ダミー
    ],
};
/// 借入金残高明細 trailer
static L25_TRAILER: Layout = Layout {
    fields: &[
        n("data_kind", 1), // データ区分
        n("count", 6),     // データ・レコード件数
        c("dummy", 243),   // ダミー
    ],
};
/// 借入金残高明細 end
static L25_END: Layout = Layout {
    fields: &[
        n("data_kind", 1), // データ区分
        c("dummy", 249),   // ダミー
    ],
};
/// 借入金残高合計 header
static L26_HEADER: Layout = Layout {
    fields: &[
        n("data_kind", 1),    // データ区分
        n("type", 2),         // 種別コード
        n("code", 1),         // コード区分
        n("created_date", 6), // 作成日
        n("notice_date", 6),  // ※連絡日
        n("notice_seq", 2),   // ※連絡回数
        c("dummy", 6),        // ダミー
        c("dummy2", 6),       // ダミー
        n("bank_no", 4),      // 銀行コード
        c("bank_name", 15),   // 銀行名
        n("branch_no", 3),    // 支店コード
        c("branch_name", 15), // 支店名
        n("client_code", 10), // 会社コード
        c("client_name", 50), // 会社名
        c("dummy3", 123),     // ダミー
    ],
};
/// 借入金残高合計 data
static L26_DATA_0: Layout = Layout {
    fields: &[
        n("data_kind", 1),          // データ区分
        c("dummy", 2),              // ダミー
        c("dummy2", 1),             // ダミー
        c("dummy3", 20),            // ダミー
        n("base_date", 6),          // 基準日
        n("base_time", 4),          // ※基準時刻
        n("branch_no", 3),          // 支店コード
        c("dummy4", 1),             // ダミー
        n("bill_loan_balance", 15), // 手形貸付残高
        n("deed_loan_balance", 15), // 証書貸付残高
        n("discount_balance", 15),  // 商業手形割引残高
        c("dummy5", 135),           // ダミー
        n("total_balance", 15),     // 借入金合計残高
        c("dummy6", 17),            // ダミー
    ],
};
/// 借入金残高合計 trailer
static L26_TRAILER: Layout = Layout {
    fields: &[
        n("data_kind", 1), // データ区分
        n("count", 6),     // データ・レコード件数
        c("dummy", 243),   // ダミー
    ],
};
/// 借入金残高合計 end
static L26_END: Layout = Layout {
    fields: &[
        n("data_kind", 1), // データ区分
        c("dummy", 249),   // ダミー
    ],
};
/// 外国為替取引明細（会計性） header
static L27_HEADER: Layout = Layout {
    fields: &[
        n("data_kind", 1),          // データ区分
        n("type", 2),               // 種別コード
        n("code", 1),               // コード区分
        n("trade_info_kind", 3),    // 取引情報区分
        n("created_date", 8),       // 作成日
        n("notice_date", 8),        // ※連絡日
        n("notice_count", 2),       // ※連絡回数
        n("acct_date_from", 8),     // 勘定日（自）
        n("acct_date_to", 8),       // 勘定日（至）
        n("bank_code", 4),          // 銀行コード
        c("bank_name", 15),         // 銀行名
        n("agent_branch_code", 3),  // ※支店（取次店）コード
        c("agent_branch_name", 15), // ※支店（取次店）名
        n("branch_code", 3),        // 支店（取扱店）コード
        c("branch_name", 15),       // 支店（取扱店）名
        n("company_code", 10),      // 会社コード
        c("company_name", 70),      // 会社名
        c("dummy", 74),             // ダミー
    ],
};
/// 外国為替取引明細（会計性） data 01 本体情報
static L27_DATA_0: Layout = Layout {
    fields: &[
        n("data_kind", 1),        // データ区分
        n("record_kind", 2),      // データ・レコード区分
        n("composition_kind", 2), // データ構成区分
        n("orig_trade_kind", 2),  // ※原取引区分
        n("cancel_code", 1),      // 取消コード
        n("seq_no", 4),           // 処理通番
        c("bank_ref", 16),        // ※銀行照会番号（銀行レファレンス）
        c("customer_ref", 32),    // ※取引先照会番号（取引先レファレンス）
        c("lc_no", 32),           // ※信用状番号
        c("trade_currency", 3),   // ※取引金額(1)幣種
        n("trade_decimals", 1),   // ※取引金額(2)小数点位置
        n("trade_amount", 15),    // ※取引金額(3)金額
        c("bill_currency", 3),    // ※手形金額(1)幣種
        n("bill_decimals", 1),    // ※手形金額(2)小数点位置
        n("bill_amount", 15),     // ※手形金額(3)金額
        c("message", 50),         // ※メッセージ
        c("dummy", 70),           // ダミー
    ],
};
/// 外国為替取引明細（会計性） data 11 取引情報［輸出手形買取］
static L27_DATA_1: Layout = Layout {
    fields: &[
        n("data_kind", 1),            // データ区分
        n("record_kind", 2),          // データ・レコード区分
        n("acct_date", 8),            // 勘定日
        n("value_date", 8),           // バリュー（起算）日
        n("bill_kind", 1),            // 手形種別区分
        n("bill_kind_detail", 1),     // ※手形種別内訳
        n("insured_kind", 1),         // ※付保区分
        n("rate_kind", 2),            // 相場区分
        c("purchase_currency", 3),    // 買取内訳金額(1)幣種
        n("purchase_decimals", 1),    // 買取内訳金額(2)小数点位置
        n("purchase_amount", 15),     // 買取内訳金額(3)金額
        c("contract_no", 16),         // ※※予約番号
        n("conv_rate_unit", 1),       // ※※換算相場(1)通貨単位
        n("conv_rate_decimals", 1),   // ※※換算相場(2)小数点位置
        n("conv_rate", 8),            // ※※換算相場(3)相場
        c("conv_currency", 3),        // ※※換算額(1)幣種
        n("conv_decimals", 1),        // ※※換算額(2)小数点位置
        n("conv_amount", 15),         // ※※換算額(3)金額
        n("rate_type", 2),            // ※相場種類
        n("tenor_code", 2),           // ※テナー・コード
        n("tenor_days", 3),           // ※テナー日数
        n("due_date", 8),             // ※期日
        n("interest_bearer", 1),      // ※利息負担区分
        c("credit1_currency", 3),     // ※入金内訳(1)(1)幣種
        n("credit1_decimals", 1),     // ※入金内訳(1)(2)小数点位置
        n("credit1_acct_dummy", 3),   // ※入金内訳(1)(3)入金口座①ダミー
        n("credit1_deposit_type", 1), // ※入金内訳(1)(3)入金口座②預金種目
        n("credit1_account_no", 10),  // ※入金内訳(1)(3)入金口座③口座番号
        n("credit1_amount", 15),      // ※入金内訳(1)(4)金額
        c("credit2_currency", 3),     // ※入金内訳(2)(1)幣種
        n("credit2_decimals", 1),     // ※入金内訳(2)(2)小数点位置
        n("credit2_acct_dummy", 3),   // ※入金内訳(2)(3)入金口座①ダミー
        n("credit2_deposit_type", 1), // ※入金内訳(2)(3)入金口座②預金種目
        n("credit2_account_no", 10),  // ※入金内訳(2)(3)入金口座③口座番号
        n("credit2_amount", 15),      // ※入金内訳(2)(4)金額
        n("fee1_kind", 1),            // ※利息・手数料合計(1)(1)利息・手数料区分
        c("fee1_currency", 3),        // ※利息・手数料合計(1)(2)幣種
        n("fee1_decimals", 1),        // ※利息・手数料合計(1)(3)小数点位置
        n("fee1_amount", 13),         // ※利息・手数料合計(1)(4)金額
        n("fee1_collect_kind", 1),    // ※利息・手数料合計(1)(5)徴求区分
        n("fee2_kind", 1),            // ※利息・手数料合計(2)(1)利息・手数料区分
        c("fee2_currency", 3),        // ※利息・手数料合計(2)(2)幣種
        n("fee2_decimals", 1),        // ※利息・手数料合計(2)(3)小数点位置
        n("fee2_amount", 13),         // ※利息・手数料合計(2)(4)金額
        n("fee2_collect_kind", 1),    // ※利息・手数料合計(2)(5)徴求区分
        n("fee_debit_date", 8),       // ※利息・手数料引落予定日
        n("debit_request_no", 11),    // ※引落請求番号
        c("dummy", 23),               // ダミー
    ],
};
/// 外国為替取引明細（会計性） data 21 取引情報［輸出手形買取内訳］
static L27_DATA_2: Layout = Layout {
    fields: &[
        n("data_kind", 1),             // データ区分
        n("record_kind", 2),           // データ・レコード区分
        n("same_kind_count", 2),       // 同一種類データ・レコード数
        n("same_kind_no", 2),          // 同一種類データ・レコード番号
        n("d1_rate_kind", 2),          // 買取内訳[項番5](1)相場区分
        c("d1_purchase_currency", 3),  // 買取内訳[項番5](2)買取内訳金額①幣種
        n("d1_purchase_decimals", 1),  // 買取内訳[項番5](2)買取内訳金額②小数点位置
        n("d1_purchase_amount", 15),   // 買取内訳[項番5](2)買取内訳金額③金額
        c("d1_contract_no", 16),       // ※※買取内訳[項番5](3)予約番号
        n("d1_conv_unit", 1),          // ※※買取内訳[項番5](4)換算相場①通貨単位
        n("d1_conv_rate_decimals", 1), // ※※買取内訳[項番5](4)換算相場②小数点位置
        n("d1_conv_rate", 8),          // ※※買取内訳[項番5](4)換算相場③相場
        c("d1_conv_currency", 3),      // ※※買取内訳[項番5](5)換算額①幣種
        n("d1_conv_decimals", 1),      // ※※買取内訳[項番5](5)換算額②小数点位置
        n("d1_conv_amount", 15),       // ※※買取内訳[項番5](5)換算額③金額
        n("d2_rate_kind", 2),          // 買取内訳[項番6](1)相場区分
        c("d2_purchase_currency", 3),  // 買取内訳[項番6](2)買取内訳金額①幣種
        n("d2_purchase_decimals", 1),  // 買取内訳[項番6](2)買取内訳金額②小数点位置
        n("d2_purchase_amount", 15),   // 買取内訳[項番6](2)買取内訳金額③金額
        c("d2_contract_no", 16),       // ※※買取内訳[項番6](3)予約番号
        n("d2_conv_unit", 1),          // ※※買取内訳[項番6](4)換算相場①通貨単位
        n("d2_conv_rate_decimals", 1), // ※※買取内訳[項番6](4)換算相場②小数点位置
        n("d2_conv_rate", 8),          // ※※買取内訳[項番6](4)換算相場③相場
        c("d2_conv_currency", 3),      // ※※買取内訳[項番6](5)換算額①幣種
        n("d2_conv_decimals", 1),      // ※※買取内訳[項番6](5)換算額②小数点位置
        n("d2_conv_amount", 15),       // ※※買取内訳[項番6](5)換算額③金額
        n("d3_rate_kind", 2),          // 買取内訳[項番7](1)相場区分
        c("d3_purchase_currency", 3),  // 買取内訳[項番7](2)買取内訳金額①幣種
        n("d3_purchase_decimals", 1),  // 買取内訳[項番7](2)買取内訳金額②小数点位置
        n("d3_purchase_amount", 15),   // 買取内訳[項番7](2)買取内訳金額③金額
        c("d3_contract_no", 16),       // ※※買取内訳[項番7](3)予約番号
        n("d3_conv_unit", 1),          // ※※買取内訳[項番7](4)換算相場①通貨単位
        n("d3_conv_rate_decimals", 1), // ※※買取内訳[項番7](4)換算相場②小数点位置
        n("d3_conv_rate", 8),          // ※※買取内訳[項番7](4)換算相場③相場
        c("d3_conv_currency", 3),      // ※※買取内訳[項番7](5)換算額①幣種
        n("d3_conv_decimals", 1),      // ※※買取内訳[項番7](5)換算額②小数点位置
        n("d3_conv_amount", 15),       // ※※買取内訳[項番7](5)換算額③金額
        c("dummy", 45),                // ダミー
    ],
};
/// 外国為替取引明細（会計性） data 12 取引情報［輸入手形決済］
static L27_DATA_3: Layout = Layout {
    fields: &[
        n("data_kind", 1),           // データ区分
        n("record_kind", 2),         // データ・レコード区分
        n("acct_date", 8),           // 勘定日
        n("value_date", 8),          // バリュー（起算）日
        n("bill_kind", 1),           // 手形種別区分
        n("usance_code", 2),         // ユーザンス・コード
        n("rate_kind", 2),           // 相場区分
        c("settle_currency", 3),     // 決済内訳金額(1)幣種
        n("settle_decimals", 1),     // 決済内訳金額(2)小数点位置
        n("settle_amount", 15),      // 決済内訳金額(3)金額
        c("contract_no", 16),        // ※※予約番号
        n("conv_rate_unit", 1),      // ※※換算相場(1)通貨単位
        n("conv_rate_decimals", 1),  // ※※換算相場(2)小数点位置
        n("conv_rate", 8),           // ※※換算相場(3)相場
        c("conv_currency", 3),       // 換算額(1)幣種
        n("conv_decimals", 1),       // 換算額(2)小数点位置
        n("conv_amount", 15),        // 換算額(3)金額
        n("rate_type", 2),           // ※相場種類
        n("settle_kind", 1),         // ※手形決済区分
        c("debit1_currency", 3),     // ※引落内訳(1)(1)幣種
        n("debit1_decimals", 1),     // ※引落内訳(1)(2)小数点位置
        n("debit1_acct_dummy", 3),   // ※引落内訳(1)(3)引落口座①ダミー
        n("debit1_deposit_type", 1), // ※引落内訳(1)(3)引落口座②預金種目
        n("debit1_account_no", 10),  // ※引落内訳(1)(3)引落口座③口座番号
        n("debit1_amount", 15),      // ※引落内訳(1)(4)金額
        c("debit2_currency", 3),     // ※引落内訳(2)(1)幣種
        n("debit2_decimals", 1),     // ※引落内訳(2)(2)小数点位置
        n("debit2_acct_dummy", 3),   // ※引落内訳(2)(3)引落口座①ダミー
        n("debit2_deposit_type", 1), // ※引落内訳(2)(3)引落口座②預金種目
        n("debit2_account_no", 10),  // ※引落内訳(2)(3)引落口座③口座番号
        n("debit2_amount", 15),      // ※引落内訳(2)(4)金額
        n("fee1_kind", 1),           // ※利息・手数料合計(1)(1)利息・手数料区分
        c("fee1_currency", 3),       // ※利息・手数料合計(1)(2)幣種
        n("fee1_decimals", 1),       // ※利息・手数料合計(1)(3)小数点位置
        n("fee1_amount", 13),        // ※利息・手数料合計(1)(4)金額
        n("fee1_collect_kind", 1),   // ※利息・手数料合計(1)(5)徴求区分
        n("fee2_kind", 1),           // ※利息・手数料合計(2)(1)利息・手数料区分
        c("fee2_currency", 3),       // ※利息・手数料合計(2)(2)幣種
        n("fee2_decimals", 1),       // ※利息・手数料合計(2)(3)小数点位置
        n("fee2_amount", 13),        // ※利息・手数料合計(2)(4)金額
        n("fee2_collect_kind", 1),   // ※利息・手数料合計(2)(5)徴求区分
        n("fee_debit_date", 8),      // ※利息・手数料引落予定日
        n("debit_request_no", 11),   // ※引落請求番号
        c("dummy", 36),              // ダミー
    ],
};
/// 外国為替取引明細（会計性） data 22 取引情報［輸入手形決済内訳］
static L27_DATA_4: Layout = Layout {
    fields: &[
        n("data_kind", 1),             // データ区分
        n("record_kind", 2),           // データ・レコード区分
        n("same_kind_count", 2),       // 同一種類データ・レコード数
        n("same_kind_no", 2),          // 同一種類データ・レコード番号
        n("d1_rate_kind", 2),          // 決済内訳[項番5](1)相場区分
        c("d1_settle_currency", 3),    // 決済内訳[項番5](2)決済内訳金額①幣種
        n("d1_settle_decimals", 1),    // 決済内訳[項番5](2)決済内訳金額②小数点位置
        n("d1_settle_amount", 15),     // 決済内訳[項番5](2)決済内訳金額③金額
        c("d1_contract_no", 16),       // ※※決済内訳[項番5](3)予約番号
        n("d1_conv_unit", 1),          // ※※決済内訳[項番5](4)換算相場①通貨単位
        n("d1_conv_rate_decimals", 1), // ※※決済内訳[項番5](4)換算相場②小数点位置
        n("d1_conv_rate", 8),          // ※※決済内訳[項番5](4)換算相場③相場
        c("d1_conv_currency", 3),      // ※※決済内訳[項番5](5)換算額①幣種
        n("d1_conv_decimals", 1),      // ※※決済内訳[項番5](5)換算額②小数点位置
        n("d1_conv_amount", 15),       // ※※決済内訳[項番5](5)換算額③金額
        n("d2_rate_kind", 2),          // 決済内訳[項番6](1)相場区分
        c("d2_settle_currency", 3),    // 決済内訳[項番6](2)決済内訳金額①幣種
        n("d2_settle_decimals", 1),    // 決済内訳[項番6](2)決済内訳金額②小数点位置
        n("d2_settle_amount", 15),     // 決済内訳[項番6](2)決済内訳金額③金額
        c("d2_contract_no", 16),       // ※※決済内訳[項番6](3)予約番号
        n("d2_conv_unit", 1),          // ※※決済内訳[項番6](4)換算相場①通貨単位
        n("d2_conv_rate_decimals", 1), // ※※決済内訳[項番6](4)換算相場②小数点位置
        n("d2_conv_rate", 8),          // ※※決済内訳[項番6](4)換算相場③相場
        c("d2_conv_currency", 3),      // ※※決済内訳[項番6](5)換算額①幣種
        n("d2_conv_decimals", 1),      // ※※決済内訳[項番6](5)換算額②小数点位置
        n("d2_conv_amount", 15),       // ※※決済内訳[項番6](5)換算額③金額
        n("d3_rate_kind", 2),          // 決済内訳[項番7](1)相場区分
        c("d3_settle_currency", 3),    // 決済内訳[項番7](2)決済内訳金額①幣種
        n("d3_settle_decimals", 1),    // 決済内訳[項番7](2)決済内訳金額②小数点位置
        n("d3_settle_amount", 15),     // 決済内訳[項番7](2)決済内訳金額③金額
        c("d3_contract_no", 16),       // ※※決済内訳[項番7](3)予約番号
        n("d3_conv_unit", 1),          // ※※決済内訳[項番7](4)換算相場①通貨単位
        n("d3_conv_rate_decimals", 1), // ※※決済内訳[項番7](4)換算相場②小数点位置
        n("d3_conv_rate", 8),          // ※※決済内訳[項番7](4)換算相場③相場
        c("d3_conv_currency", 3),      // ※※決済内訳[項番7](5)換算額①幣種
        n("d3_conv_decimals", 1),      // ※※決済内訳[項番7](5)換算額②小数点位置
        n("d3_conv_amount", 15),       // ※※決済内訳[項番7](5)換算額③金額
        c("dummy", 45),                // ダミー
    ],
};
/// 外国為替取引明細（会計性） data 13 取引情報［輸出手形取立支払］
static L27_DATA_5: Layout = Layout {
    fields: &[
        n("data_kind", 1),            // データ区分
        n("record_kind", 2),          // データ・レコード区分
        n("acct_date", 8),            // 勘定日
        n("value_date", 8),           // バリュー（起算）日
        n("bill_kind", 1),            // 手形種別区分
        n("bill_kind_detail", 1),     // ※手形種別内訳
        c("dummy", 1),                // ダミー
        n("rate_kind", 2),            // 相場区分
        c("collect_currency", 3),     // 取立支払内訳金額(1)幣種
        n("collect_decimals", 1),     // 取立支払内訳金額(2)小数点位置
        n("collect_amount", 15),      // 取立支払内訳金額(3)金額
        c("contract_no", 16),         // ※※予約番号
        n("conv_rate_unit", 1),       // ※※換算相場(1)通貨単位
        n("conv_rate_decimals", 1),   // ※※換算相場(2)小数点位置
        n("conv_rate", 8),            // ※※換算相場(3)相場
        c("conv_currency", 3),        // ※※換算額(1)幣種
        n("conv_decimals", 1),        // ※※換算額(2)小数点位置
        n("conv_amount", 15),         // ※※換算額(3)金額
        n("rate_type", 2),            // ※相場種類
        c("dummy2", 2),               // ダミー
        c("dummy3", 3),               // ダミー
        n("due_date", 8),             // ※期日
        n("pay_kind", 1),             // ※支払区分
        c("credit1_currency", 3),     // ※入金内訳(1)(1)幣種
        n("credit1_decimals", 1),     // ※入金内訳(1)(2)小数点位置
        n("credit1_acct_dummy", 3),   // ※入金内訳(1)(3)入金口座①ダミー
        n("credit1_deposit_type", 1), // ※入金内訳(1)(3)入金口座②預金種目
        n("credit1_account_no", 10),  // ※入金内訳(1)(3)入金口座③口座番号
        n("credit1_amount", 15),      // ※入金内訳(1)(4)金額
        c("credit2_currency", 3),     // ※入金内訳(2)(1)幣種
        n("credit2_decimals", 1),     // ※入金内訳(2)(2)小数点位置
        n("credit2_acct_dummy", 3),   // ※入金内訳(2)(3)入金口座①ダミー
        n("credit2_deposit_type", 1), // ※入金内訳(2)(3)入金口座②預金種目
        n("credit2_account_no", 10),  // ※入金内訳(2)(3)入金口座③口座番号
        n("credit2_amount", 15),      // ※入金内訳(2)(4)金額
        n("fee1_kind", 1),            // ※利息・手数料合計(1)(1)利息・手数料区分
        c("fee1_currency", 3),        // ※利息・手数料合計(1)(2)幣種
        n("fee1_decimals", 1),        // ※利息・手数料合計(1)(3)小数点位置
        n("fee1_amount", 13),         // ※利息・手数料合計(1)(4)金額
        n("fee1_collect_kind", 1),    // ※利息・手数料合計(1)(5)徴求区分
        n("fee2_kind", 1),            // ※利息・手数料合計(2)(1)利息・手数料区分
        c("fee2_currency", 3),        // ※利息・手数料合計(2)(2)幣種
        n("fee2_decimals", 1),        // ※利息・手数料合計(2)(3)小数点位置
        n("fee2_amount", 13),         // ※利息・手数料合計(2)(4)金額
        n("fee2_collect_kind", 1),    // ※利息・手数料合計(2)(5)徴求区分
        n("fee_debit_date", 8),       // ※利息・手数料引落予定日
        n("debit_request_no", 11),    // ※引落請求番号
        c("dummy4", 23),              // ダミー
    ],
};
/// 外国為替取引明細（会計性） data 23 取引情報［輸出手形取立支払内訳］
static L27_DATA_6: Layout = Layout {
    fields: &[
        n("data_kind", 1),             // データ区分
        n("record_kind", 2),           // データ・レコード区分
        n("same_kind_count", 2),       // 同一種類データ・レコード数
        n("same_kind_no", 2),          // 同一種類データ・レコード番号
        n("d1_rate_kind", 2),          // 取立支払内訳[項番5](1)相場区分
        c("d1_collect_currency", 3),   // 取立支払内訳[項番5](2)取立支払内訳金額①幣種
        n("d1_collect_decimals", 1),   // 取立支払内訳[項番5](2)取立支払内訳金額②小数点位置
        n("d1_collect_amount", 15),    // 取立支払内訳[項番5](2)取立支払内訳金額③金額
        c("d1_contract_no", 16),       // ※※取立支払内訳[項番5](3)予約番号
        n("d1_conv_unit", 1),          // ※※取立支払内訳[項番5](4)換算相場①通貨単位
        n("d1_conv_rate_decimals", 1), // ※※取立支払内訳[項番5](4)換算相場②小数点位置
        n("d1_conv_rate", 8),          // ※※取立支払内訳[項番5](4)換算相場③相場
        c("d1_conv_currency", 3),      // ※※取立支払内訳[項番5](5)換算額①幣種
        n("d1_conv_decimals", 1),      // ※※取立支払内訳[項番5](5)換算額②小数点位置
        n("d1_conv_amount", 15),       // ※※取立支払内訳[項番5](5)換算額③金額
        n("d2_rate_kind", 2),          // 取立支払内訳[項番6](1)相場区分
        c("d2_collect_currency", 3),   // 取立支払内訳[項番6](2)取立支払内訳金額①幣種
        n("d2_collect_decimals", 1),   // 取立支払内訳[項番6](2)取立支払内訳金額②小数点位置
        n("d2_collect_amount", 15),    // 取立支払内訳[項番6](2)取立支払内訳金額③金額
        c("d2_contract_no", 16),       // ※※取立支払内訳[項番6](3)予約番号
        n("d2_conv_unit", 1),          // ※※取立支払内訳[項番6](4)換算相場①通貨単位
        n("d2_conv_rate_decimals", 1), // ※※取立支払内訳[項番6](4)換算相場②小数点位置
        n("d2_conv_rate", 8),          // ※※取立支払内訳[項番6](4)換算相場③相場
        c("d2_conv_currency", 3),      // ※※取立支払内訳[項番6](5)換算額①幣種
        n("d2_conv_decimals", 1),      // ※※取立支払内訳[項番6](5)換算額②小数点位置
        n("d2_conv_amount", 15),       // ※※取立支払内訳[項番6](5)換算額③金額
        n("d3_rate_kind", 2),          // 取立支払内訳[項番7](1)相場区分
        c("d3_collect_currency", 3),   // 取立支払内訳[項番7](2)取立支払内訳金額①幣種
        n("d3_collect_decimals", 1),   // 取立支払内訳[項番7](2)取立支払内訳金額②小数点位置
        n("d3_collect_amount", 15),    // 取立支払内訳[項番7](2)取立支払内訳金額③金額
        c("d3_contract_no", 16),       // ※※取立支払内訳[項番7](3)予約番号
        n("d3_conv_unit", 1),          // ※※取立支払内訳[項番7](4)換算相場①通貨単位
        n("d3_conv_rate_decimals", 1), // ※※取立支払内訳[項番7](4)換算相場②小数点位置
        n("d3_conv_rate", 8),          // ※※取立支払内訳[項番7](4)換算相場③相場
        c("d3_conv_currency", 3),      // ※※取立支払内訳[項番7](5)換算額①幣種
        n("d3_conv_decimals", 1),      // ※※取立支払内訳[項番7](5)換算額②小数点位置
        n("d3_conv_amount", 15),       // ※※取立支払内訳[項番7](5)換算額③金額
        c("dummy", 45),                // ダミー
    ],
};
/// 外国為替取引明細（会計性） data 14 取引情報［仕向送金取組］
static L27_DATA_7: Layout = Layout {
    fields: &[
        n("data_kind", 1),           // データ区分
        n("record_kind", 2),         // データ・レコード区分
        n("acct_date", 8),           // 勘定日
        n("value_date", 8),          // バリュー（起算）日
        n("remit_kind", 1),          // ※送金種類区分
        n("telex_kind", 1),          // ※電信種類区分
        n("pay_kind", 1),            // ※支払区分
        n("rate_kind", 2),           // 相場区分
        c("remit_currency", 3),      // 取組内訳金額(1)幣種
        n("remit_decimals", 1),      // 取組内訳金額(2)小数点位置
        n("remit_amount", 15),       // 取組内訳金額(3)金額
        c("contract_no", 16),        // ※※予約番号
        n("conv_rate_unit", 1),      // ※※換算相場(1)通貨単位
        n("conv_rate_decimals", 1),  // ※※換算相場(2)小数点位置
        n("conv_rate", 8),           // ※※換算相場(3)相場
        c("conv_currency", 3),       // ※※換算額(1)幣種
        n("conv_decimals", 1),       // ※※換算額(2)小数点位置
        n("conv_amount", 15),        // ※※換算額(3)金額
        n("rate_type", 2),           // ※相場種類
        c("dummy", 1),               // ダミー
        c("debit1_currency", 3),     // ※引落内訳(1)(1)幣種
        n("debit1_decimals", 1),     // ※引落内訳(1)(2)小数点位置
        n("debit1_acct_dummy", 3),   // ※引落内訳(1)(3)引落口座①ダミー
        n("debit1_deposit_type", 1), // ※引落内訳(1)(3)引落口座②預金種目
        n("debit1_account_no", 10),  // ※引落内訳(1)(3)引落口座③口座番号
        n("debit1_amount", 15),      // ※引落内訳(1)(4)金額
        c("debit2_currency", 3),     // ※引落内訳(2)(1)幣種
        n("debit2_decimals", 1),     // ※引落内訳(2)(2)小数点位置
        n("debit2_acct_dummy", 3),   // ※引落内訳(2)(3)引落口座①ダミー
        n("debit2_deposit_type", 1), // ※引落内訳(2)(3)引落口座②預金種目
        n("debit2_account_no", 10),  // ※引落内訳(2)(3)引落口座③口座番号
        n("debit2_amount", 15),      // ※引落内訳(2)(4)金額
        n("fee1_kind", 1),           // ※利息・手数料合計(1)(1)利息・手数料区分
        c("fee1_currency", 3),       // ※利息・手数料合計(1)(2)幣種
        n("fee1_decimals", 1),       // ※利息・手数料合計(1)(3)小数点位置
        n("fee1_amount", 13),        // ※利息・手数料合計(1)(4)金額
        n("fee1_collect_kind", 1),   // ※利息・手数料合計(1)(5)徴求区分
        n("fee2_kind", 1),           // ※利息・手数料合計(2)(1)利息・手数料区分
        c("fee2_currency", 3),       // ※利息・手数料合計(2)(2)幣種
        n("fee2_decimals", 1),       // ※利息・手数料合計(2)(3)小数点位置
        n("fee2_amount", 13),        // ※利息・手数料合計(2)(4)金額
        n("fee2_collect_kind", 1),   // ※利息・手数料合計(2)(5)徴求区分
        n("fee_debit_date", 8),      // ※利息・手数料引落予定日
        n("debit_request_no", 11),   // ※引落請求番号
        c("dummy2", 36),             // ダミー
    ],
};
/// 外国為替取引明細（会計性） data 24 取引情報［仕向送金取組内訳］
static L27_DATA_8: Layout = Layout {
    fields: &[
        n("data_kind", 1),             // データ区分
        n("record_kind", 2),           // データ・レコード区分
        n("same_kind_count", 2),       // 同一種類データ・レコード数
        n("same_kind_no", 2),          // 同一種類データ・レコード番号
        n("d1_rate_kind", 2),          // 取組内訳[項番5](1)相場区分
        c("d1_remit_currency", 3),     // 取組内訳[項番5](2)取組内訳金額①幣種
        n("d1_remit_decimals", 1),     // 取組内訳[項番5](2)取組内訳金額②小数点位置
        n("d1_remit_amount", 15),      // 取組内訳[項番5](2)取組内訳金額③金額
        c("d1_contract_no", 16),       // ※※取組内訳[項番5](3)予約番号
        n("d1_conv_unit", 1),          // ※※取組内訳[項番5](4)換算相場①通貨単位
        n("d1_conv_rate_decimals", 1), // ※※取組内訳[項番5](4)換算相場②小数点位置
        n("d1_conv_rate", 8),          // ※※取組内訳[項番5](4)換算相場③相場
        c("d1_conv_currency", 3),      // ※※取組内訳[項番5](5)換算額①幣種
        n("d1_conv_decimals", 1),      // ※※取組内訳[項番5](5)換算額②小数点位置
        n("d1_conv_amount", 15),       // ※※取組内訳[項番5](5)換算額③金額
        n("d2_rate_kind", 2),          // 取組内訳[項番6](1)相場区分
        c("d2_remit_currency", 3),     // 取組内訳[項番6](2)取組内訳金額①幣種
        n("d2_remit_decimals", 1),     // 取組内訳[項番6](2)取組内訳金額②小数点位置
        n("d2_remit_amount", 15),      // 取組内訳[項番6](2)取組内訳金額③金額
        c("d2_contract_no", 16),       // ※※取組内訳[項番6](3)予約番号
        n("d2_conv_unit", 1),          // ※※取組内訳[項番6](4)換算相場①通貨単位
        n("d2_conv_rate_decimals", 1), // ※※取組内訳[項番6](4)換算相場②小数点位置
        n("d2_conv_rate", 8),          // ※※取組内訳[項番6](4)換算相場③相場
        c("d2_conv_currency", 3),      // ※※取組内訳[項番6](5)換算額①幣種
        n("d2_conv_decimals", 1),      // ※※取組内訳[項番6](5)換算額②小数点位置
        n("d2_conv_amount", 15),       // ※※取組内訳[項番6](5)換算額③金額
        n("d3_rate_kind", 2),          // 取組内訳[項番7](1)相場区分
        c("d3_remit_currency", 3),     // 取組内訳[項番7](2)取組内訳金額①幣種
        n("d3_remit_decimals", 1),     // 取組内訳[項番7](2)取組内訳金額②小数点位置
        n("d3_remit_amount", 15),      // 取組内訳[項番7](2)取組内訳金額③金額
        c("d3_contract_no", 16),       // ※※取組内訳[項番7](3)予約番号
        n("d3_conv_unit", 1),          // ※※取組内訳[項番7](4)換算相場①通貨単位
        n("d3_conv_rate_decimals", 1), // ※※取組内訳[項番7](4)換算相場②小数点位置
        n("d3_conv_rate", 8),          // ※※取組内訳[項番7](4)換算相場③相場
        c("d3_conv_currency", 3),      // ※※取組内訳[項番7](5)換算額①幣種
        n("d3_conv_decimals", 1),      // ※※取組内訳[項番7](5)換算額②小数点位置
        n("d3_conv_amount", 15),       // ※※取組内訳[項番7](5)換算額③金額
        c("dummy", 45),                // ダミー
    ],
};
/// 外国為替取引明細（会計性） data 15 取引情報［被仕向送金支払］
static L27_DATA_9: Layout = Layout {
    fields: &[
        n("data_kind", 1),            // データ区分
        n("record_kind", 2),          // データ・レコード区分
        n("acct_date", 8),            // 勘定日
        n("value_date", 8),           // バリュー（起算）日
        n("remit_kind", 1),           // ※送金種類区分
        c("dummy", 2),                // ダミー
        n("rate_kind", 2),            // 相場区分
        c("pay_currency", 3),         // 支払内訳金額(1)幣種
        n("pay_decimals", 1),         // 支払内訳金額(2)小数点位置
        n("pay_amount", 15),          // 支払内訳金額(3)金額
        c("contract_no", 16),         // ※※予約番号
        n("conv_rate_unit", 1),       // ※※換算相場(1)通貨単位
        n("conv_rate_decimals", 1),   // ※※換算相場(2)小数点位置
        n("conv_rate", 8),            // ※※換算相場(3)相場
        c("conv_currency", 3),        // ※※換算額(1)幣種
        n("conv_decimals", 1),        // ※※換算額(2)小数点位置
        n("conv_amount", 15),         // ※※換算額(3)金額
        n("rate_type", 2),            // ※相場種類
        c("dummy2", 14),              // ダミー
        c("credit1_currency", 3),     // ※入金内訳(1)(1)幣種
        n("credit1_decimals", 1),     // ※入金内訳(1)(2)小数点位置
        n("credit1_acct_dummy", 3),   // ※入金内訳(1)(3)入金口座①ダミー
        n("credit1_deposit_type", 1), // ※入金内訳(1)(3)入金口座②預金種目
        n("credit1_account_no", 10),  // ※入金内訳(1)(3)入金口座③口座番号
        n("credit1_amount", 15),      // ※入金内訳(1)(4)金額
        c("credit2_currency", 3),     // ※入金内訳(2)(1)幣種
        n("credit2_decimals", 1),     // ※入金内訳(2)(2)小数点位置
        n("credit2_acct_dummy", 3),   // ※入金内訳(2)(3)入金口座①ダミー
        n("credit2_deposit_type", 1), // ※入金内訳(2)(3)入金口座②預金種目
        n("credit2_account_no", 10),  // ※入金内訳(2)(3)入金口座③口座番号
        n("credit2_amount", 15),      // ※入金内訳(2)(4)金額
        n("fee1_kind", 1),            // ※利息・手数料合計(1)(1)利息・手数料区分
        c("fee1_currency", 3),        // ※利息・手数料合計(1)(2)幣種
        n("fee1_decimals", 1),        // ※利息・手数料合計(1)(3)小数点位置
        n("fee1_amount", 13),         // ※利息・手数料合計(1)(4)金額
        n("fee1_collect_kind", 1),    // ※利息・手数料合計(1)(5)徴求区分
        n("fee2_kind", 1),            // ※利息・手数料合計(2)(1)利息・手数料区分
        c("fee2_currency", 3),        // ※利息・手数料合計(2)(2)幣種
        n("fee2_decimals", 1),        // ※利息・手数料合計(2)(3)小数点位置
        n("fee2_amount", 13),         // ※利息・手数料合計(2)(4)金額
        n("fee2_collect_kind", 1),    // ※利息・手数料合計(2)(5)徴求区分
        n("fee_debit_date", 8),       // ※利息・手数料引落予定日
        n("debit_request_no", 11),    // ※引落請求番号
        c("dummy3", 23),              // ダミー
    ],
};
/// 外国為替取引明細（会計性） data 25 取引情報［被仕向送金支払内訳］
static L27_DATA_10: Layout = Layout {
    fields: &[
        n("data_kind", 1),             // データ区分
        n("record_kind", 2),           // データ・レコード区分
        n("same_kind_count", 2),       // 同一種類データ・レコード数
        n("same_kind_no", 2),          // 同一種類データ・レコード番号
        n("d1_rate_kind", 2),          // 支払内訳[項番5](1)相場区分
        c("d1_pay_currency", 3),       // 支払内訳[項番5](2)支払内訳金額①幣種
        n("d1_pay_decimals", 1),       // 支払内訳[項番5](2)支払内訳金額②小数点位置
        n("d1_pay_amount", 15),        // 支払内訳[項番5](2)支払内訳金額③金額
        c("d1_contract_no", 16),       // ※※支払内訳[項番5](3)予約番号
        n("d1_conv_unit", 1),          // ※※支払内訳[項番5](4)換算相場①通貨単位
        n("d1_conv_rate_decimals", 1), // ※※支払内訳[項番5](4)換算相場②小数点位置
        n("d1_conv_rate", 8),          // ※※支払内訳[項番5](4)換算相場③相場
        c("d1_conv_currency", 3),      // ※※支払内訳[項番5](5)換算額①幣種
        n("d1_conv_decimals", 1),      // ※※支払内訳[項番5](5)換算額②小数点位置
        n("d1_conv_amount", 15),       // ※※支払内訳[項番5](5)換算額③金額
        n("d2_rate_kind", 2),          // 支払内訳[項番6](1)相場区分
        c("d2_pay_currency", 3),       // 支払内訳[項番6](2)支払内訳金額①幣種
        n("d2_pay_decimals", 1),       // 支払内訳[項番6](2)支払内訳金額②小数点位置
        n("d2_pay_amount", 15),        // 支払内訳[項番6](2)支払内訳金額③金額
        c("d2_contract_no", 16),       // ※※支払内訳[項番6](3)予約番号
        n("d2_conv_unit", 1),          // ※※支払内訳[項番6](4)換算相場①通貨単位
        n("d2_conv_rate_decimals", 1), // ※※支払内訳[項番6](4)換算相場②小数点位置
        n("d2_conv_rate", 8),          // ※※支払内訳[項番6](4)換算相場③相場
        c("d2_conv_currency", 3),      // ※※支払内訳[項番6](5)換算額①幣種
        n("d2_conv_decimals", 1),      // ※※支払内訳[項番6](5)換算額②小数点位置
        n("d2_conv_amount", 15),       // ※※支払内訳[項番6](5)換算額③金額
        n("d3_rate_kind", 2),          // 支払内訳[項番7](1)相場区分
        c("d3_pay_currency", 3),       // 支払内訳[項番7](2)支払内訳金額①幣種
        n("d3_pay_decimals", 1),       // 支払内訳[項番7](2)支払内訳金額②小数点位置
        n("d3_pay_amount", 15),        // 支払内訳[項番7](2)支払内訳金額③金額
        c("d3_contract_no", 16),       // ※※支払内訳[項番7](3)予約番号
        n("d3_conv_unit", 1),          // ※※支払内訳[項番7](4)換算相場①通貨単位
        n("d3_conv_rate_decimals", 1), // ※※支払内訳[項番7](4)換算相場②小数点位置
        n("d3_conv_rate", 8),          // ※※支払内訳[項番7](4)換算相場③相場
        c("d3_conv_currency", 3),      // ※※支払内訳[項番7](5)換算額①幣種
        n("d3_conv_decimals", 1),      // ※※支払内訳[項番7](5)換算額②小数点位置
        n("d3_conv_amount", 15),       // ※※支払内訳[項番7](5)換算額③金額
        c("dummy", 45),                // ダミー
    ],
};
/// 外国為替取引明細（会計性） data 91 取引情報［利息・手数料情報（都度請求）］
static L27_DATA_11: Layout = Layout {
    fields: &[
        n("data_kind", 1),           // データ区分
        n("record_kind", 2),         // データ・レコード区分
        n("acct_date", 8),           // 勘定日
        n("value_date", 8),          // バリュー（起算）日
        n("fee1_kind", 1),           // ※利息・手数料合計(1)(1)利息・手数料区分
        c("fee1_currency", 3),       // ※利息・手数料合計(1)(2)幣種
        n("fee1_decimals", 1),       // ※利息・手数料合計(1)(3)小数点位置
        n("fee1_amount", 13),        // ※利息・手数料合計(1)(4)金額
        n("fee1_collect_kind", 1),   // ※利息・手数料合計(1)(5)徴求区分
        n("fee2_kind", 1),           // ※利息・手数料合計(2)(1)利息・手数料区分
        c("fee2_currency", 3),       // ※利息・手数料合計(2)(2)幣種
        n("fee2_decimals", 1),       // ※利息・手数料合計(2)(3)小数点位置
        n("fee2_amount", 13),        // ※利息・手数料合計(2)(4)金額
        n("fee2_collect_kind", 1),   // ※利息・手数料合計(2)(5)徴求区分
        n("fee_debit_date", 8),      // ※引落予定日
        n("debit_request_no", 11),   // ※引落請求番号
        c("debit1_currency", 3),     // ※引落内訳(1)(1)幣種
        n("debit1_decimals", 1),     // ※引落内訳(1)(2)小数点位置
        n("debit1_acct_dummy", 3),   // ※引落内訳(1)(3)引落口座①ダミー
        n("debit1_deposit_type", 1), // ※引落内訳(1)(3)引落口座②預金種目
        n("debit1_account_no", 10),  // ※引落内訳(1)(3)引落口座③口座番号
        n("debit1_amount", 13),      // ※引落内訳(1)(4)金額
        c("debit2_currency", 3),     // ※入金内訳(2)(1)幣種
        n("debit2_decimals", 1),     // ※入金内訳(2)(2)小数点位置
        n("debit2_acct_dummy", 3),   // ※入金内訳(2)(3)引落口座①ダミー
        n("debit2_deposit_type", 1), // ※入金内訳(2)(3)引落口座②預金種目
        n("debit2_account_no", 10),  // ※入金内訳(2)(3)引落口座③口座番号
        n("debit2_amount", 13),      // ※入金内訳(2)(4)金額
        c("dummy", 112),             // ダミー
    ],
};
/// 外国為替取引明細（会計性） data 92 利息・手数料
static L27_DATA_12: Layout = Layout {
    fields: &[
        n("data_kind", 1),          // データ区分
        n("record_kind", 2),        // データ・レコード区分
        n("same_kind_count", 2),    // 同一種類データ・レコード数
        n("same_kind_no", 2),       // 同一種類データ・レコード番号
        n("int_code", 2),           // 利息(1)利息区分
        n("int_rate_kind", 1),      // ※利息(2)利率・料率区分
        n("int_rate", 8),           // ※利息(3)利率・料率
        n("int_year_days", 1),      // ※利息(4)年日数区分
        n("int_calc_kind", 1),      // ※利息(5)利息計算区分
        n("int_days", 4),           // ※利息(6)利息日数
        n("int_from", 8),           // ※利息(7)利息始期
        n("int_to", 8),             // ※利息(8)利息終期
        c("int_fc_currency", 3),    // ※利息(9)利息外貨額①幣種
        n("int_fc_decimals", 1),    // ※利息(9)利息外貨額②小数点位置
        n("int_fc_amount", 12),     // ※利息(9)利息外貨額③金額
        n("int_conv_unit", 1),      // ※利息(10)換算相場①通貨単位
        n("int_conv_decimals", 1),  // ※利息(10)換算相場②小数点位置
        n("int_conv_rate", 8),      // ※利息(10)換算相場③相場
        n("int_rounding", 1),       // ※利息(11)端数処理
        n("int_jpy_amount", 12),    // ※利息(12)利息円貨額
        n("int_collect_kind", 1),   // ※利息(13)徴求区分
        n("fee1_code", 2),          // 手数料(1)(1)手数料区分
        n("fee1_rate_kind", 1),     // ※手数料(1)(2)利率・料率区分
        n("fee1_rate", 8),          // ※手数料(1)(3)利率・料率
        c("fee1_fc_currency", 3),   // ※手数料(1)(4)手数料外貨額①幣種
        n("fee1_fc_decimals", 1),   // ※手数料(1)(4)手数料外貨額②小数点位置
        n("fee1_fc_amount", 10),    // ※手数料(1)(4)手数料外貨額③金額
        n("fee1_conv_unit", 1),     // ※手数料(1)(5)換算相場①通貨単位
        n("fee1_conv_decimals", 1), // ※手数料(1)(5)換算相場②小数点位置
        n("fee1_conv_rate", 8),     // ※手数料(1)(5)換算相場③相場
        n("fee1_jpy_amount", 10),   // ※手数料(1)(6)手数料円貨額
        n("fee1_collect_kind", 1),  // ※手数料(1)(7)徴求区分
        n("fee2_code", 2),          // 手数料(2)(1)手数料区分
        n("fee2_rate_kind", 1),     // ※手数料(2)(2)利率・料率区分
        n("fee2_rate", 8),          // ※手数料(2)(3)利率・料率
        c("fee2_fc_currency", 3),   // ※手数料(2)(4)手数料外貨額①幣種
        n("fee2_fc_decimals", 1),   // ※手数料(2)(4)手数料外貨額②小数点位置
        n("fee2_fc_amount", 10),    // ※手数料(2)(4)手数料外貨額③金額
        n("fee2_conv_unit", 1),     // ※手数料(2)(5)換算相場①通貨単位
        n("fee2_conv_decimals", 1), // ※手数料(2)(5)換算相場②小数点位置
        n("fee2_conv_rate", 8),     // ※手数料(2)(5)換算相場③相場
        n("fee2_jpy_amount", 10),   // ※手数料(2)(6)手数料円貨額
        n("fee2_collect_kind", 1),  // ※手数料(2)(7)徴求区分
        n("fee3_code", 2),          // 手数料(3)(1)手数料区分
        n("fee3_rate_kind", 1),     // ※手数料(3)(2)利率・料率区分
        n("fee3_rate", 8),          // ※手数料(3)(3)利率・料率
        c("fee3_fc_currency", 3),   // ※手数料(3)(4)手数料外貨額①幣種
        n("fee3_fc_decimals", 1),   // ※手数料(3)(4)手数料外貨額②小数点位置
        n("fee3_fc_amount", 10),    // ※手数料(3)(4)手数料外貨額③金額
        n("fee3_conv_unit", 1),     // ※手数料(3)(5)換算相場①通貨単位
        n("fee3_conv_decimals", 1), // ※手数料(3)(5)換算相場②小数点位置
        n("fee3_conv_rate", 8),     // ※手数料(3)(5)換算相場③相場
        n("fee3_jpy_amount", 10),   // ※手数料(3)(6)手数料円貨額
        n("fee3_collect_kind", 1),  // ※手数料(3)(7)徴求区分
        c("dummy", 32),             // ダミー
    ],
};
/// 外国為替取引明細（会計性） data 93 利息・手数料一括請求［合計］
static L27_DATA_13: Layout = Layout {
    fields: &[
        n("data_kind", 1),          // データ区分
        n("record_kind", 2),        // データ・レコード区分
        n("seq_no", 4),             // 処理通番
        n("claim_kind", 1),         // 請求・引落区分
        n("cutoff_date", 8),        // ※締切日
        n("debit_plan_date", 8),    // ※引落予定日
        n("debit_acct_date", 8),    // ※引落勘定日
        n("debit_value_date", 8),   // ※引落バリュー（起算）日
        n("debit_acct_dummy", 3),   // ※引落口座(1)ダミー
        n("debit_deposit_type", 1), // ※引落口座(2)預金種目
        n("debit_account_no", 10),  // ※引落口座(3)口座番号
        c("total_currency", 3),     // 請求・引落合計金額①幣種
        n("total_decimals", 1),     // 請求・引落合計金額②小数点位置
        n("total_amount", 13),      // 請求・引落合計金額③金額
        n("total_sign", 1),         // 請求・引落合計金額④差引金額表示
        c("dummy", 178),            // ダミー
    ],
};
/// 外国為替取引明細（会計性） data 94 利息・手数料一括請求［内訳］
static L27_DATA_14: Layout = Layout {
    fields: &[
        n("data_kind", 1),               // データ区分
        n("record_kind", 2),             // データ・レコード区分
        n("same_kind_count", 4),         // 同一種類データ・レコード数
        n("same_kind_no", 4),            // 同一種類データ・レコード番号
        n("item1_trade_date", 8),        // ※利息・手数料内訳(1)(1)取引日
        n("item1_debit_request_no", 11), // ※利息・手数料内訳(1)(2)引落請求番号
        n("item1_cancel_code", 1),       // 利息・手数料内訳(1)(3)取消コード
        c("item1_currency", 3),          // 利息・手数料内訳(1)(4)請求・引落金額①幣種
        n("item1_decimals", 1),          // 利息・手数料内訳(1)(4)請求・引落金額②小数点位置
        n("item1_amount", 13),           // 利息・手数料内訳(1)(4)請求・引落金額③金額
        n("item1_fee_kind", 1),          // 利息・手数料内訳(1)(5)利息・手数料区分
        n("item1_fee_code", 2),          // 利息・手数料内訳(1)(6)利息・手数料コード
        c("item1_bank_ref", 16),         // ※利息・手数料内訳(1)(7)銀行照会番号（銀行レファレンス）
        c("item1_customer_ref", 32),     // ※利息・手数料内訳(1)(8)取引先照会番号（取引先レファレンス）
        n("item2_trade_date", 8),        // ※利息・手数料内訳(2)(1)取引日
        n("item2_debit_request_no", 11), // ※利息・手数料内訳(2)(2)引落請求番号
        n("item2_cancel_code", 1),       // 利息・手数料内訳(2)(3)取消コード
        c("item2_currency", 3),          // 利息・手数料内訳(2)(4)請求・引落金額①幣種
        n("item2_decimals", 1),          // 利息・手数料内訳(2)(4)請求・引落金額②小数点位置
        n("item2_amount", 13),           // 利息・手数料内訳(2)(4)請求・引落金額③金額
        n("item2_fee_kind", 1),          // 利息・手数料内訳(2)(5)利息・手数料区分
        n("item2_fee_code", 2),          // 利息・手数料内訳(2)(6)利息・手数料コード
        c("item2_bank_ref", 16),         // ※利息・手数料内訳(2)(7)銀行照会番号（銀行レファレンス）
        c("item2_customer_ref", 32),     // ※利息・手数料内訳(2)(8)取引先照会番号（取引先レファレンス）
        c("dummy", 63),                  // ダミー
    ],
};
/// 外国為替取引明細（会計性） data 51 為替予約［先物予約締結］
static L27_DATA_15: Layout = Layout {
    fields: &[
        n("data_kind", 1),              // データ区分
        n("record_kind", 2),            // データ・レコード区分
        n("cancel_code", 1),            // 取消コード
        n("seq_no", 4),                 // 処理通番
        c("bank_ref", 16),              // 銀行照会番号（銀行レファレンス）
        c("customer_ref", 32),          // 取引先照会番号（取引先レファレンス）
        n("book_date", 8),              // 記帳日
        n("value_date", 8),             // 起算日
        n("trade_kind", 1),             // 取引区分
        n("buy_sell", 1),               // 売買区分
        n("delivery_kind", 1),          // ※引渡区分
        n("contract_date", 8),          // 締結日
        c("dummy", 19),                 // ダミー
        n("delivery_from", 8),          // 引渡開始日
        n("delivery_to", 8),            // 引渡終期日
        c("contract_currency", 3),      // 予約締結金額(1)幣種
        n("contract_decimals", 1),      // 予約締結金額(2)小数点位置
        n("contract_amount", 15),       // 予約締結金額(3)金額
        n("contract_rate_unit", 1),     // 予約相場(1)通貨単位
        n("contract_rate_decimals", 1), // 予約相場(2)小数点位置
        n("contract_rate", 8),          // 予約相場(3)相場
        c("counter_currency", 3),       // ※対価金額(1)幣種
        n("counter_decimals", 1),       // ※対価金額(2)小数点位置
        n("counter_amount", 15),        // ※対価金額(3)金額
        n("counter_kind", 1),           // ※対価区分
        c("message", 50),               // ※メッセージ
        c("dummy2", 33),                // ダミー
    ],
};
/// 外国為替取引明細（会計性） data 52 為替予約［先物予約履行］
static L27_DATA_16: Layout = Layout {
    fields: &[
        n("data_kind", 1),              // データ区分
        n("record_kind", 2),            // データ・レコード区分
        n("cancel_code", 1),            // 取消コード
        n("seq_no", 4),                 // 処理通番
        c("bank_ref", 16),              // 銀行照会番号（銀行レファレンス）
        c("customer_ref", 32),          // ※取引先照会番号（取引先レファレンス）
        n("book_date", 8),              // 記帳日
        n("value_date", 8),             // 起算日
        n("trade_kind", 1),             // 取引区分
        n("buy_sell", 1),               // 売買区分
        n("delivery_kind", 1),          // ※引渡区分
        c("exec_currency", 3),          // 履行金額(1)幣種
        n("exec_decimals", 1),          // 履行金額(2)小数点位置
        n("exec_amount", 15),           // 履行金額(3)金額
        n("applied_rate_unit", 1),      // ※適用相場(1)通貨単位
        n("applied_rate_decimals", 1),  // ※適用相場(2)小数点位置
        n("applied_rate", 8),           // ※適用相場(3)相場
        c("counter_currency", 3),       // ※対価金額(1)幣種
        n("counter_decimals", 1),       // ※対価金額(2)小数点位置
        n("counter_amount", 15),        // ※対価金額(3)金額
        n("counter_kind", 1),           // ※対価区分
        n("contract_rate_unit", 1),     // 予約相場(1)通貨単位
        n("contract_rate_decimals", 1), // 予約相場(2)小数点位置
        n("contract_rate", 8),          // 予約相場(3)相場
        n("related_trade_kind", 2),     // ※関連取引区分
        c("related_bank_ref", 16),      // ※関連取引銀行照会番号
        c("related_customer_ref", 32),  // ※関連取引取引先照会番号
        c("message", 50),               // ※メッセージ
        c("dummy", 16),                 // ダミー
    ],
};
/// 外国為替取引明細（会計性） data 53 為替予約［先物予約残高明細］
static L27_DATA_17: Layout = Layout {
    fields: &[
        n("data_kind", 1),              // データ区分
        n("record_kind", 2),            // データ・レコード区分
        c("dummy", 1),                  // ダミー
        n("seq_no", 4),                 // 処理通番
        c("bank_ref", 16),              // 銀行照会番号（銀行レファレンス）
        c("customer_ref", 32),          // ※取引先照会番号（取引先レファレンス）
        n("base_date", 8),              // 基準日
        n("base_time", 4),              // ※基準時刻
        c("dummy2", 4),                 // ダミー
        c("dummy3", 1),                 // ダミー
        n("buy_sell", 1),               // 売買区分
        n("delivery_kind", 1),          // ※引渡区分
        n("contract_date", 8),          // 締結日
        c("orig_contract_currency", 3), // ※当初予約締結金額(1)幣種
        n("orig_contract_decimals", 1), // ※当初予約締結金額(2)小数点位置
        n("orig_contract_amount", 15),  // ※当初予約締結金額(3)金額
        n("delivery_from", 8),          // 引渡開始日
        n("delivery_to", 8),            // 引渡終期日
        c("balance_currency", 3),       // 予約残高(1)幣種
        n("balance_decimals", 1),       // 予約残高(2)小数点位置
        n("balance_amount", 15),        // 予約残高(3)金額
        n("contract_rate_unit", 1),     // 予約相場(1)通貨単位
        n("contract_rate_decimals", 1), // 予約相場(2)小数点位置
        n("contract_rate", 8),          // 予約相場(3)相場
        c("counter_currency", 3),       // ※対価金額(1)幣種
        n("counter_decimals", 1),       // ※対価金額(2)小数点位置
        n("counter_amount", 15),        // ※対価金額(3)金額
        n("counter_kind", 1),           // ※対価区分
        c("message", 50),               // ※メッセージ
        c("dummy4", 33),                // ダミー
    ],
};
/// 外国為替取引明細（会計性） trailer
static L27_TRAILER: Layout = Layout {
    fields: &[
        n("data_kind", 1),    // データ区分
        n("record_count", 6), // データ・レコード件数
        c("dummy", 243),      // ダミー
    ],
};
/// 外国為替取引明細（会計性） end
static L27_END: Layout = Layout {
    fields: &[
        n("data_kind", 1),   // データ区分
        n("total_count", 6), // レコード総件数
        c("dummy", 243),     // ダミー
    ],
};
/// 外国為替取引明細（非会計性） header
static L28_HEADER: Layout = Layout {
    fields: &[
        n("data_kind", 1),          // データ区分
        n("type", 2),               // 種別コード
        n("code", 1),               // コード区分
        n("trade_info_kind", 3),    // 取引情報区分
        n("created_date", 8),       // 作成日
        n("notice_date", 8),        // ※連絡日
        n("notice_count", 2),       // ※連絡回数
        c("dummy", 8),              // ダミー
        c("dummy2", 8),             // ダミー
        n("bank_code", 4),          // 銀行コード
        c("bank_name", 15),         // 銀行名
        n("agent_branch_code", 3),  // ※支店（取次店）コード
        c("agent_branch_name", 15), // ※支店（取次店）名
        n("branch_code", 3),        // ※支店（取扱店）コード
        c("branch_name", 15),       // ※支店（取扱店）名
        n("company_code", 10),      // 会社コード
        c("company_name", 70),      // 会社名
        c("dummy3", 74),            // ダミー
    ],
};
/// 外国為替取引明細（非会計性） data 02 輸出信用状到着案内（固定レコード）
static L28_DATA_0: Layout = Layout {
    fields: &[
        n("data_kind", 1),          // データ区分
        n("record_kind", 2),        // データ・レコード区分
        n("composition_kind", 2),   // データ構成区分
        n("cancel_code", 1),        // 取消コード
        n("seq_no", 4),             // 処理通番
        c("bank_ref", 16),          // ※銀行照会番号（銀行レファレンス）
        n("receipt_date", 8),       // 接受日
        n("issue_date", 8),         // ※発行日
        n("lc_kind", 1),            // 種別
        n("amend_kind", 1),         // ※AMENDMENT区分
        n("amend_count", 3),        // ※AMENDMENT回数
        n("amount_change_kind", 1), // ※金額変更内容区分
        n("expiry_change_kind", 1), // ※有効期限内容区分
        c("lc_no", 32),             // 信用状番号
        c("lc_currency", 3),        // 信用状金額(1)幣種
        n("lc_decimals", 1),        // 信用状金額(2)小数点位置
        n("lc_amount", 15),         // 信用状金額(3)金額
        c("lc_change_currency", 3), // ※信用状増減金額(1)幣種
        n("lc_change_decimals", 1), // ※信用状増減金額(2)小数点位置
        n("lc_change_amount", 15),  // ※信用状増減金額(3)金額
        n("expiry_date", 8),        // 有効期限
        n("restricted", 1),         // ※リストリクト有無
        n("confirmed", 1),          // ※確認有無
        n("advice_fee_payer", 1),   // ※通知手数料負担者
        n("confirm_fee_payer", 1),  // ※確認手数料負担者
        n("receipt_method", 1),     // ※接受方法
        c("dummy", 118),            // ダミー
    ],
};
/// 外国為替取引明細（非会計性） data 62 輸出信用状到着案内（可変レコード）
static L28_DATA_1: Layout = Layout {
    fields: &[
        n("data_kind", 1),       // データ区分
        n("record_kind", 2),     // データ・レコード区分
        n("same_kind_count", 4), // 同一種類データ・レコード数
        n("same_kind_no", 4),    // 同一種類データ・レコード番号
        n("var_item_count", 2),  // 可変項目数
        n("var_item_len", 3),    // 可変項目有効桁数
        c("variable_area", 234), // 可変項目（項番7～17）＋ダミー（項番18、残り桁数）
    ],
};
/// 外国為替取引明細（非会計性） data 03 輸入船積書類到着案内（固定レコード）
static L28_DATA_2: Layout = Layout {
    fields: &[
        n("data_kind", 1),          // データ区分
        n("record_kind", 2),        // データ・レコード区分
        n("composition_kind", 2),   // データ構成区分
        n("cancel_code", 1),        // 取消コード
        n("seq_no", 4),             // 処理通番
        c("bank_ref", 16),          // ※銀行照会番号（銀行レファレンス）
        n("receipt_date", 8),       // 接受日
        n("receipt_kind", 1),       // ※接受区分
        n("delivery_terms", 1),     // ※書類引渡条件
        n("draft_date", 8),         // ※手形振出日
        c("drawer_name_addr1", 90), // ※手形振出人名・住所（1）
        n("bill_kind", 1),          // ※手形種別区分
        n("usance_code", 2),        // ※ユーザンス・コード
        n("settle_date", 8),        // ※対外決済日
        c("trade_currency", 3),     // 取引金額(1)幣種
        n("trade_decimals", 1),     // 取引金額(2)小数点位置
        n("trade_amount", 15),      // 取引金額(3)金額
        c("bill_currency", 3),      // ※手形金額(1)幣種
        n("bill_decimals", 1),      // ※手形金額(2)小数点位置
        n("bill_amount", 15),       // ※手形金額(3)金額
        c("tenor_unit", 1),         // ※手形テナー(1)表示区分
        n("tenor_count", 3),        // ※手形テナー(2)日数または月数
        c("tenor", 2),              // ※手形テナー(3)テナー
        n("due_date", 8),           // ※期日
        n("nego_date", 8),          // ※買取日
        n("ship_date", 8),          // ※船積日
        n("trade_terms", 1),        // ※建値
        c("dummy", 36),             // ダミー
    ],
};
/// 外国為替取引明細（非会計性） data 63 輸入船積書類到着案内（可変レコード）
static L28_DATA_3: Layout = Layout {
    fields: &[
        n("data_kind", 1),       // データ区分
        n("record_kind", 2),     // データ・レコード区分
        n("same_kind_count", 4), // 同一種類データ・レコード数
        n("same_kind_no", 4),    // 同一種類データ・レコード番号
        n("var_item_count", 2),  // 可変項目数
        n("var_item_len", 3),    // 可変項目有効桁数
        c("variable_area", 234), // 可変項目（項番7～11）＋ダミー（項番12、残り桁数）
    ],
};
/// 外国為替取引明細（非会計性） data 06 被仕向送金到着案内（固定レコード）
static L28_DATA_4: Layout = Layout {
    fields: &[
        n("data_kind", 1),        // データ区分
        n("record_kind", 2),      // データ・レコード区分
        n("composition_kind", 2), // データ構成区分
        n("cancel_code", 1),      // 取消コード
        n("seq_no", 4),           // 処理通番
        c("bank_ref", 16),        // ※銀行照会番号（銀行レファレンス）
        n("send_date", 8),        // ※仕向日
        n("receipt_date", 8),     // ※接受日
        n("remit_kind", 1),       // ※送金種類区分
        n("value_date", 8),       // ※バリュー日
        c("remit_currency", 3),   // 金額(1)幣種
        n("remit_decimals", 1),   // 金額(2)小数点位置
        n("remit_amount", 15),    // 金額(3)金額
        c("remitter_name", 140),  // 依頼人名
        c("dummy", 40),           // ダミー
    ],
};
/// 外国為替取引明細（非会計性） data 66 被仕向送金到着案内（可変レコード）
static L28_DATA_5: Layout = Layout {
    fields: &[
        n("data_kind", 1),       // データ区分
        n("record_kind", 2),     // データ・レコード区分
        n("same_kind_count", 4), // 同一種類データ・レコード数
        n("same_kind_no", 4),    // 同一種類データ・レコード番号
        n("var_item_count", 2),  // 可変項目数
        n("var_item_len", 3),    // 可変項目有効桁数
        c("variable_area", 234), // 可変項目（項番7～16）＋ダミー（項番17、残り桁数）
    ],
};
/// 外国為替取引明細（非会計性） data 07 輸出手形経過情報（固定レコード）
static L28_DATA_6: Layout = Layout {
    fields: &[
        n("data_kind", 1),          // データ区分
        n("record_kind", 2),        // データ・レコード区分
        n("composition_kind", 2),   // データ構成区分
        n("cancel_code", 1),        // 取消コード
        n("seq_no", 4),             // 処理通番
        c("bank_ref", 16),          // ※銀行照会番号（銀行レファレンス）
        c("customer_ref", 32),      // ※取引先照会番号（取引先レファレンス）
        c("orig_bill_currency", 3), // ※原手形金額(1)幣種
        n("orig_bill_decimals", 1), // ※原手形金額(2)小数点位置
        n("orig_bill_amount", 15),  // ※原手形金額(3)金額
        n("orig_nego_date", 8),     // ※原手形買取日
        c("tenor_unit", 1),         // ※手形テナー(1)表示区分
        n("tenor_count", 3),        // ※手形テナー(2)日数または月数
        c("tenor", 2),              // ※手形テナー(3)テナー
        n("est_due_date", 8),       // ※推定期日
        c("dummy", 9),              // ダミー
        c("lc_no", 32),             // ※信用状番号
        n("sent_date", 8),          // ※発信日
        n("input_date", 8),         // 入力日
        n("receipt_method", 1),     // ※接受方法
        n("notice_kind", 1),        // 種別
        n("pay_accept_kind", 1),    // ※支払／引受区分
        n("refusal_kind", 1),       // ※支払／引受拒絶区分
        n("pay_date", 8),           // ※支払実行日
        c("pay_currency", 3),       // ※支払金額(1)幣種
        n("pay_decimals", 1),       // ※支払金額(2)小数点位置
        n("pay_amount", 15),        // ※支払金額(3)金額
        n("accept_date", 8),        // ※引受日
        n("fixed_due_date", 8),     // ※確定期日
        n("settle_date", 8),        // ※決済日
        c("settle_currency", 3),    // ※決済金額(1)幣種
        n("settle_decimals", 1),    // ※決済金額(2)小数点位置
        n("settle_amount", 15),     // ※決済金額(3)金額
        n("buyer_code", 10),        // ※バイヤー・コード
        c("dummy2", 10),            // ダミー
    ],
};
/// 外国為替取引明細（非会計性） data 67 輸出手形経過情報（可変レコード）
static L28_DATA_7: Layout = Layout {
    fields: &[
        n("data_kind", 1),       // データ区分
        n("record_kind", 2),     // データ・レコード区分
        n("same_kind_count", 4), // 同一種類データ・レコード数
        n("same_kind_no", 4),    // 同一種類データ・レコード番号
        n("var_item_count", 2),  // 可変項目数
        n("var_item_len", 3),    // 可変項目有効桁数
        c("variable_area", 234), // 可変項目（項番7～9）＋ダミー（項番10、残り桁数）
    ],
};
/// 外国為替取引明細（非会計性） data 92 利息・手数料
static L28_DATA_8: Layout = Layout {
    fields: &[
        n("data_kind", 1),          // データ区分
        n("record_kind", 2),        // データ・レコード区分
        n("same_kind_count", 2),    // 同一種類データ・レコード数
        n("same_kind_no", 2),       // 同一種類データ・レコード番号
        n("int_code", 2),           // 利息(1)取引区分
        n("int_rate_kind", 1),      // ※利息(2)利率・料率区分
        n("int_rate", 8),           // ※利息(3)利率・料率
        n("int_year_days", 1),      // ※利息(4)年日数区分
        n("int_calc_kind", 1),      // ※利息(5)利息計算区分
        n("int_days", 4),           // ※利息(6)利息日数
        n("int_from", 8),           // ※利息(7)利息始期
        n("int_to", 8),             // ※利息(8)利息終期
        c("int_fc_currency", 3),    // ※利息(9)利息外貨額①幣種
        n("int_fc_decimals", 1),    // ※利息(9)利息外貨額②小数点位置
        n("int_fc_amount", 12),     // ※利息(9)利息外貨額③金額
        n("int_conv_unit", 1),      // ※利息(10)換算相場①通貨単位
        n("int_conv_decimals", 1),  // ※利息(10)換算相場②小数点位置
        n("int_conv_rate", 8),      // ※利息(10)換算相場③相場
        n("int_rounding", 1),       // ※利息(11)端数処理
        n("int_jpy_amount", 12),    // ※利息(12)利息円貨額
        n("int_collect_kind", 1),   // ※利息(13)徴求区分
        n("fee1_code", 2),          // 手数料(1)(1)手数料区分
        n("fee1_rate_kind", 1),     // ※手数料(1)(2)利率・料率区分
        n("fee1_rate", 8),          // ※手数料(1)(3)利率・料率
        c("fee1_fc_currency", 3),   // ※手数料(1)(4)手数料外貨額①幣種
        n("fee1_fc_decimals", 1),   // ※手数料(1)(4)手数料外貨額②小数点位置
        n("fee1_fc_amount", 10),    // ※手数料(1)(4)手数料外貨額③金額
        n("fee1_conv_unit", 1),     // ※手数料(1)(5)換算相場①通貨単位
        n("fee1_conv_decimals", 1), // ※手数料(1)(5)換算相場②小数点位置
        n("fee1_conv_rate", 8),     // ※手数料(1)(5)換算相場③相場
        n("fee1_jpy_amount", 10),   // ※手数料(1)(6)手数料円貨額
        n("fee1_collect_kind", 1),  // ※手数料(1)(7)徴求区分
        n("fee2_code", 2),          // 手数料(2)(1)手数料区分
        n("fee2_rate_kind", 1),     // ※手数料(2)(2)利率・料率区分
        n("fee2_rate", 8),          // ※手数料(2)(3)利率・料率
        c("fee2_fc_currency", 3),   // ※手数料(2)(4)手数料外貨額①幣種
        n("fee2_fc_decimals", 1),   // ※手数料(2)(4)手数料外貨額②小数点位置
        n("fee2_fc_amount", 10),    // ※手数料(2)(4)手数料外貨額③金額
        n("fee2_conv_unit", 1),     // ※手数料(2)(5)換算相場①通貨単位
        n("fee2_conv_decimals", 1), // ※手数料(2)(5)換算相場②小数点位置
        n("fee2_conv_rate", 8),     // ※手数料(2)(5)換算相場③相場
        n("fee2_jpy_amount", 10),   // ※手数料(2)(6)手数料円貨額
        n("fee2_collect_kind", 1),  // ※手数料(2)(7)徴求区分
        n("fee3_code", 2),          // 手数料(3)(1)手数料区分
        n("fee3_rate_kind", 1),     // ※手数料(3)(2)利率・料率区分
        n("fee3_rate", 8),          // ※手数料(3)(3)利率・料率
        c("fee3_fc_currency", 3),   // ※手数料(3)(4)手数料外貨額①幣種
        n("fee3_fc_decimals", 1),   // ※手数料(3)(4)手数料外貨額②小数点位置
        n("fee3_fc_amount", 10),    // ※手数料(3)(4)手数料外貨額③金額
        n("fee3_conv_unit", 1),     // ※手数料(3)(5)換算相場①通貨単位
        n("fee3_conv_decimals", 1), // ※手数料(3)(5)換算相場②小数点位置
        n("fee3_conv_rate", 8),     // ※手数料(3)(5)換算相場③相場
        n("fee3_jpy_amount", 10),   // ※手数料(3)(6)手数料円貨額
        n("fee3_collect_kind", 1),  // ※手数料(3)(7)徴求区分
        c("dummy", 32),             // ダミー
    ],
};
/// 外国為替取引明細（非会計性） trailer
static L28_TRAILER: Layout = Layout {
    fields: &[
        n("data_kind", 1),    // データ区分
        n("record_count", 6), // データ・レコード件数
        c("dummy", 243),      // ダミー
    ],
};
/// 外国為替取引明細（非会計性） end
static L28_END: Layout = Layout {
    fields: &[
        n("data_kind", 1),   // データ区分
        n("total_count", 6), // レコード総件数
        c("dummy", 243),     // ダミー
    ],
};
/// 外国為替関連情報 header
static L29_HEADER: Layout = Layout {
    fields: &[
        n("data_kind", 1),             // データ区分
        n("type", 2),                  // 種別コード
        n("code", 1),                  // コード区分
        n("info_kind", 3),             // 取引情報区分
        n("created_on", 8),            // 作成日
        n("notified_on", 8),           // ※連絡日
        n("notify_seq", 2),            // ※連絡回数
        n("base_date", 8),             // 基準日
        c("dummy", 8),                 // ダミー
        n("bank_code", 4),             // 銀行コード
        c("bank_name", 15),            // 銀行名
        n("agent_branch_code", 3),     // ※支店（取次店）コード
        c("agent_branch_name", 15),    // ※支店（取次店）名
        n("handling_branch_code", 3),  // ※支店（取扱店）コード
        c("handling_branch_name", 15), // ※支店（取扱店）名
        n("company_code", 10),         // 会社コード
        c("company_name", 70),         // 会社名
        c("dummy2", 74),               // ダミー
    ],
};
/// 外国為替関連情報 data 外国為替公示相場
static L29_DATA_0: Layout = Layout {
    fields: &[
        n("data_kind", 1),                  // データ区分
        n("record_kind", 2),                // データ・レコード区分
        n("seq", 4),                        // 処理通番
        n("publish_count", 1),              // ※公示回数
        n("publish_time", 6),               // ※公示時刻
        c("rate1_currency", 3),             // 相場1 (1)幣種
        n("rate1_unit", 1),                 // 相場1 (2)通貨単位
        n("rate1_decimal", 1),              // 相場1 (3)小数点位置
        n("rate1_tts", 8),                  // 相場1 (4)電信売相場
        n("rate1_acceptance", 8),           // 相場1 ※(5)アクセプタンス相場
        n("rate1_cash_selling", 8),         // 相場1 ※(6)現金売相場
        n("rate1_ttb", 8),                  // 相場1 (7)電信買相場
        n("rate1_lc_sight_bill_buying", 8), // 相場1 ※(8)L/C付一覧払輸出手形買相場
        n("rate1_cash_buying", 8),          // 相場1 ※(9)現金買相場
        n("rate1_no_lc_bill_buying", 8),    // 相場1 ※(10)L/Cなし輸出手形買相場
        n("rate1_lc_usance_30", 8),         // 相場1 ※(11)L/C付ユーザンス相場（30日）
        n("rate1_lc_usance_60", 8),         // 相場1 ※(12)L/C付ユーザンス相場（60日）
        n("rate1_lc_usance_90", 8),         // 相場1 ※(13)L/C付ユーザンス相場（90日）
        n("rate1_lc_usance_120", 8),        // 相場1 ※(14)L/C付ユーザンス相場（120日）
        n("rate1_lc_usance_150", 8),        // 相場1 ※(15)L/C付ユーザンス相場（150日）
        n("rate1_lc_usance_180", 8),        // 相場1 ※(16)L/C付ユーザンス相場（180日）
        n("rate1_lc_usance_over_180", 8),   // 相場1 ※(17)L/C付ユーザンス相場（180日超）
        c("rate2_currency", 3),             // 相場2 (1)幣種
        n("rate2_unit", 1),                 // 相場2 (2)通貨単位
        n("rate2_decimal", 1),              // 相場2 (3)小数点位置
        n("rate2_tts", 8),                  // 相場2 (4)電信売相場
        n("rate2_acceptance", 8),           // 相場2 ※(5)アクセプタンス相場
        n("rate2_cash_selling", 8),         // 相場2 ※(6)現金売相場
        n("rate2_ttb", 8),                  // 相場2 (7)電信買相場
        n("rate2_lc_sight_bill_buying", 8), // 相場2 ※(8)L/C付一覧払輸出手形買相場
        n("rate2_cash_buying", 8),          // 相場2 ※(9)現金買相場
        n("rate2_no_lc_bill_buying", 8),    // 相場2 ※(10)L/Cなし輸出手形買相場
        n("rate2_lc_usance_30", 8),         // 相場2 ※(11)L/C付ユーザンス相場（30日）
        n("rate2_lc_usance_60", 8),         // 相場2 ※(12)L/C付ユーザンス相場（60日）
        n("rate2_lc_usance_90", 8),         // 相場2 ※(13)L/C付ユーザンス相場（90日）
        n("rate2_lc_usance_120", 8),        // 相場2 ※(14)L/C付ユーザンス相場（120日）
        n("rate2_lc_usance_150", 8),        // 相場2 ※(15)L/C付ユーザンス相場（150日）
        n("rate2_lc_usance_180", 8),        // 相場2 ※(16)L/C付ユーザンス相場（180日）
        n("rate2_lc_usance_over_180", 8),   // 相場2 ※(17)L/C付ユーザンス相場（180日超）
        c("dummy", 2),                      // ダミー
    ],
};
/// 外国為替関連情報 trailer
static L29_TRAILER: Layout = Layout {
    fields: &[
        n("data_kind", 1), // データ区分
        n("count", 6),     // データ・レコード件数
        c("dummy", 243),   // ダミー
    ],
};
/// 外国為替関連情報 end
static L29_END: Layout = Layout {
    fields: &[
        n("data_kind", 1),          // データ区分
        n("total_record_count", 6), // レコード総件数
        c("dummy", 243),            // ダミー
    ],
};
/// 外国送金依頼 header
static L30_HEADER: Layout = Layout {
    fields: &[
        n("data_kind", 1),      // データ区分
        n("type", 2),           // 種別コード
        n("code", 1),           // コード区分
        n("remitter_code", 10), // 送金依頼人コード
        c("remitter_name", 40), // ※送金依頼人名
        n("remit_date", 4),     // 送金指定日
        n("bank_code", 4),      // 仕向銀行コード
        c("bank_name", 15),     // ※仕向銀行名
        n("branch_code", 3),    // 仕向支店（取扱店）コード
        c("branch_name", 15),   // ※仕向支店（取扱店）名
        c("dummy", 155),        // ダミー
    ],
};
/// 外国送金依頼 data 本体情報
static L30_DATA_0: Layout = Layout {
    fields: &[
        n("data_kind", 1),        // データ区分
        n("record_kind", 2),      // データ・レコード区分
        n("seq", 4),              // 処理通番
        c("customer_ref", 16),    // ※取引先照会番号
        c("remit_currency", 3),   // ＃送金金額 (1)幣種
        n("remit_decimal", 1),    // ＃送金金額 (2)小数点位置
        n("remit_amount", 15),    // ＃送金金額 (3)金額
        c("conv_currency", 3),    // ＃円貨対価金額 (1)換算幣種
        n("conv_jpy_amount", 15), // ＃円貨対価金額 (2)金額
        n("remit_kind", 1),       // 送金種類区分
        c("cable_kind", 1),       // 電信種類区分
        n("pay_method", 1),       // 支払区分
        n("charge_bearer", 1),    // 支払手数料支払区分
        n("purpose_code", 4),     // 送金目的（国際収支項目）
        c("purpose_text", 22),    // 送金目的
        c("license_no", 20),      // ※送金許可 (1)ライセンスNo.
        n("license_date", 8),     // ※送金許可 (2)ライセンス日付
        c("dummy", 132),          // ダミー
    ],
};
/// 外国送金依頼 data 会計処理内訳
static L30_DATA_1: Layout = Layout {
    fields: &[
        n("data_kind", 1),                // データ区分
        n("record_kind", 2),              // データ・レコード区分
        n("same_kind_count", 2),          // 同一種類データ・レコード数
        n("same_kind_no", 2),             // 同一種類データ・レコード番号
        n("split_count", 2),              // 分割会計内訳数
        n("acct1_rate_kind", 2),          // 会計内訳(1) (1)相場区分
        c("acct1_currency", 3),           // 会計内訳(1) (2)処理内訳金額 ①幣種
        n("acct1_decimal", 1),            // 会計内訳(1) (2)処理内訳金額 ②小数点位置
        n("acct1_amount", 15),            // 会計内訳(1) (2)処理内訳金額 ③金額
        c("acct1_dummy", 3),              // 会計内訳(1) (3)円貨対価額 ①ダミー
        c("acct1_dummy2", 1),             // 会計内訳(1) (3)円貨対価額 ②ダミー
        n("acct1_jpy_amount", 15),        // 会計内訳(1) (3)円貨対価額 ③金額
        c("acct1_contract_no", 16),       // 会計内訳(1) (4)予約番号
        c("acct1_debit_currency", 3),     // 会計内訳(1) (5)引落口座 ①幣種
        n("acct1_debit_deposit_type", 1), // 会計内訳(1) (5)引落口座 ②預金種類
        n("acct1_debit_account_no", 10),  // 会計内訳(1) (5)引落口座 ③口座番号
        c("acct1_dummy3", 10),            // 会計内訳(1) (6)ダミー
        n("acct2_rate_kind", 2),          // 会計内訳(2) (1)相場区分
        c("acct2_currency", 3),           // 会計内訳(2) (2)処理内訳金額 ①幣種
        n("acct2_decimal", 1),            // 会計内訳(2) (2)処理内訳金額 ②小数点位置
        n("acct2_amount", 15),            // 会計内訳(2) (2)処理内訳金額 ③金額
        c("acct2_dummy", 3),              // 会計内訳(2) (3)円貨対価額 ①ダミー
        c("acct2_dummy2", 1),             // 会計内訳(2) (3)円貨対価額 ②ダミー
        n("acct2_jpy_amount", 15),        // 会計内訳(2) (3)円貨対価額 ③金額
        c("acct2_contract_no", 16),       // 会計内訳(2) (4)予約番号
        c("acct2_debit_currency", 3),     // 会計内訳(2) (5)引落口座 ①幣種
        n("acct2_debit_deposit_type", 1), // 会計内訳(2) (5)引落口座 ②預金種類
        n("acct2_debit_account_no", 10),  // 会計内訳(2) (5)引落口座 ③口座番号
        c("acct2_dummy3", 10),            // 会計内訳(2) (6)ダミー
        n("acct3_rate_kind", 2),          // 会計内訳(3) (1)相場区分
        c("acct3_currency", 3),           // 会計内訳(3) (2)処理内訳金額 ①幣種
        n("acct3_decimal", 1),            // 会計内訳(3) (2)処理内訳金額 ②小数点位置
        n("acct3_amount", 15),            // 会計内訳(3) (2)処理内訳金額 ③金額
        c("acct3_dummy", 3),              // 会計内訳(3) (3)円貨対価額 ①ダミー
        c("acct3_dummy2", 1),             // 会計内訳(3) (3)円貨対価額 ②ダミー
        n("acct3_jpy_amount", 15),        // 会計内訳(3) (3)円貨対価額 ③金額
        c("acct3_contract_no", 16),       // 会計内訳(3) (4)予約番号
        c("acct3_debit_currency", 3),     // 会計内訳(3) (5)引落口座 ①幣種
        n("acct3_debit_deposit_type", 1), // 会計内訳(3) (5)引落口座 ②預金種類
        n("acct3_debit_account_no", 10),  // 会計内訳(3) (5)引落口座 ③口座番号
        c("acct3_dummy3", 10),            // 会計内訳(3) (6)ダミー
        c("dummy", 1),                    // ダミー
    ],
};
/// 外国送金依頼 data 送金指図内訳
static L30_DATA_2: Layout = Layout {
    fields: &[
        n("data_kind", 1),       // データ区分
        n("record_kind", 2),     // データ・レコード区分
        n("same_kind_count", 4), // 同一種類データ・レコード数
        n("same_kind_no", 4),    // 同一種類データ・レコード番号
        n("var_item_count", 2),  // 可変項目数
        n("var_total_len", 3),   // 可変項目有効桁数
        c("var_items", 234),     // 可変項目（項番7～11）
    ],
};
/// 外国送金依頼 trailer
static L30_TRAILER: Layout = Layout {
    fields: &[
        n("data_kind", 1),          // データ区分
        n("data_record_count", 10), // データ・レコード総件数
        n("body_count", 10),        // 本体情報総件数
        c("dummy", 229),            // ダミー
    ],
};
/// 外国送金依頼 end
static L30_END: Layout = Layout {
    fields: &[
        n("data_kind", 1), // データ区分
        c("dummy", 249),   // ダミー
    ],
};
/// 荷為替輸入信用状発行依頼 header
static L31_HEADER: Layout = Layout {
    fields: &[
        n("data_kind", 1),     // データ区分
        n("type", 2),          // 種別コード
        n("code", 1),          // コード区分
        n("created_on", 8),    // 作成日
        n("notified_on", 8),   // ※連絡日
        n("notify_seq", 2),    // 連絡回数
        n("bank_code", 4),     // 銀行コード
        c("bank_name", 15),    // 銀行名
        n("branch_code", 3),   // 支店コード
        c("branch_name", 15),  // 支店名
        n("company_code", 10), // 会社コード
        c("company_name", 40), // 会社名
        c("dummy", 141),       // ダミー
    ],
};
/// 荷為替輸入信用状発行依頼 data 本体情報(1)
static L31_DATA_0: Layout = Layout {
    fields: &[
        n("data_kind", 1),                // データ区分
        n("record_kind", 2),              // データ・レコード区分
        n("same_kind_count", 2),          // 同一種類データ・レコード数
        n("same_kind_no", 2),             // 同一種類データ・レコード番号
        n("composition_kind", 2),         // データ構成区分
        c("lc_no", 16),                   // 信用状番号(条件変更依頼のみ使用)
        n("lc_issued_on", 8),             // 信用状発行日(条件変更依頼のみ使用)
        n("issue_date", 8),               // 発行日
        c("customer_ref", 25),            // ※取引先照会番号(取引先ﾚﾌｧﾚﾝｽ)
        n("advice_method", 1),            // 信用状の通知方法
        n("cable_kind", 1),               // 電信の種類
        n("advising_bank_designated", 1), // 通知銀行指定の有無
        c("advising_bank_name", 35),      // 通知銀行 (1)銀行名
        c("advising_branch_name", 35),    // 通知銀行 (2)支店名
        c("advising_bank_address", 70),   // 通知銀行 (3)住所
        c("dummy", 41),                   // ダミー
    ],
};
/// 荷為替輸入信用状発行依頼 data 本体情報(2)
static L31_DATA_1: Layout = Layout {
    fields: &[
        n("data_kind", 1),            // データ区分
        n("record_kind", 2),          // データ・レコード区分
        n("same_kind_count", 2),      // 同一種類データ・レコード数
        n("same_kind_no", 2),         // 同一種類データ・レコード番号
        c("beneficiary_name", 70),    // 受益者 (1)受益者名
        c("beneficiary_address", 70), // 受益者 (2)受益者住所
        n("beneficiary_consent", 1),  // ※BENEFICIARY CONSENT(条件変更依頼のみ使用)
        c("dummy", 102),              // ダミー
    ],
};
/// 荷為替輸入信用状発行依頼 data 本体情報(3)
static L31_DATA_2: Layout = Layout {
    fields: &[
        n("data_kind", 1),          // データ区分
        n("record_kind", 2),        // データ・レコード区分
        n("same_kind_count", 2),    // 同一種類データ・レコード数
        n("same_kind_no", 2),       // 同一種類データ・レコード番号
        c("applicant_name", 70),    // 信用状発行依頼人 (1)依頼人名
        c("applicant_address", 70), // 信用状発行依頼人 (2)依頼人住所
        c("dummy", 103),            // ダミー
    ],
};
/// 荷為替輸入信用状発行依頼 data 取引情報(1) 発行依頼
static L31_DATA_3: Layout = Layout {
    fields: &[
        n("data_kind", 1),               // データ区分
        n("record_kind", 2),             // データ・レコード区分
        n("same_kind_count", 2),         // 同一種類データ・レコード数
        n("same_kind_no", 2),            // 同一種類データ・レコード番号
        c("lc_currency", 3),             // 信用状金額 (1)幣種
        n("lc_decimal", 1),              // 信用状金額 (2)小数点位置
        n("lc_amount", 15),              // 信用状金額 (3)金額
        n("lc_max_or_about", 1),         // 信用状金額 ※(4)MAX OR ABOUT
        n("lc_pct", 2),                  // 信用状金額 (4)MAX OR ABOUT ( )内の値
        n("amend_kind", 1),              // 修正金額(条件変更依頼のみ使用)
        c("amend_currency", 3),          // 修正金額(条件変更依頼のみ使用)
        n("amend_decimal", 1),           // 修正金額(条件変更依頼のみ使用)
        n("amend_amount", 15),           // 修正金額(条件変更依頼のみ使用)
        n("amend_max_or_about", 1),      // 修正金額(条件変更依頼のみ使用)
        n("amend_pct", 2),               // 修正金額(条件変更依頼のみ使用)
        c("new_currency", 3),            // 修正後金額(条件変更依頼のみ使用)
        n("new_decimal", 1),             // 修正後金額(条件変更依頼のみ使用)
        n("new_amount", 15),             // 修正後金額(条件変更依頼のみ使用)
        n("new_max_or_about", 1),        // 修正後金額(条件変更依頼のみ使用)
        n("new_pct", 2),                 // 修正後金額(条件変更依頼のみ使用)
        n("tenor_availability", 1),      // TENOR (1)資金化の条件
        n("tenor_draft_cond", 1),        // TENOR (2)手形の条件
        c("tenor_draft_cond_other", 25), // TENOR (2)手形の条件 OTHER詳細
        n("tenor_kind", 1),              // TENOR (3)手形TENORの種類
        c("tenor_kind_other", 35),       // TENOR (3)手形TENORの種類 OTHER詳細
        n("tenor_pct", 3),               // TENOR (4)掛率
        c("tenor_wording", 70),          // TENOR (5)記載文言
        n("tenor_drawee", 1),            // TENOR ※(6)決済方法
        n("confirmation", 1),            // 確認の要否
        n("transferable", 1),            // 譲渡可能文言 (1)譲渡可否
        c("transfer_cond", 35),          // 譲渡可能文言 (2)譲渡可能の条件
        c("dummy", 2),                   // ダミー
    ],
};
/// 荷為替輸入信用状発行依頼 data 取引情報(2) 発行依頼
static L31_DATA_4: Layout = Layout {
    fields: &[
        n("data_kind", 1),              // データ区分
        n("record_kind", 2),            // データ・レコード区分
        n("same_kind_count", 2),        // 同一種類データ・レコード数
        n("same_kind_no", 2),           // 同一種類データ・レコード番号
        n("nego_bank_designation", 1),  // ※買取銀行指定
        n("expiry_date", 8),            // 有効期限 (1)有効期限
        c("expiry_place", 29),          // 有効期限 (2)場所
        n("expiry_date_amended", 8),    // 有効期限(条件変更依頼のみ使用)
        n("payable_at_destination", 1), // 揚地払条件
        n("final_expiry_date", 8),      // ※最終有効期限
        n("shipment_date", 8),          // 船積(積出)期限
        n("shipment_date_amended", 8),  // 船積(積出)期限(条件変更依頼のみ使用)
        n("presentation_days", 2),      // 呈示期間(PRESENT DAY'S)
        n("partial_shipment", 1),       // 分割積出文言(PARTIALSHIP区分)
        n("transhipment", 1),           // 積換文言(TRANSHIP区分)
        c("ship_from", 65),             // 発送場所
        c("ship_to", 65),               // 到着場所
        c("dummy", 38),                 // ダミー
    ],
};
/// 荷為替輸入信用状発行依頼 data 取引情報(3) 発行依頼
static L31_DATA_5: Layout = Layout {
    fields: &[
        n("data_kind", 1),           // データ区分
        n("record_kind", 2),         // データ・レコード区分
        n("same_kind_count", 2),     // 同一種類データ・レコード数
        n("same_kind_no", 2),        // 同一種類データ・レコード番号
        n("invoice_copies", 2),      // 商業送り状 (1)INVOICEの通数
        c("invoice_indicating", 71), // 商業送り状 ※(2)送り状の名称等
        n("insurance_copies", 2),    // ※保険証券 ①通数
        n("insurance_pct", 3),       // ※保険証券 ②保険料率
        c("insurance_basis", 15),    // ※保険証券 ③算出基準
        c("dummy", 10),              // ダミー
        n("cargo_clause", 1),        // ※保険の区分 ①CARGO CLAUSE区分
        n("war_srcc", 1),            // ※保険の区分 ②WAR・SRCC
        c("other_insurance", 120),   // ※その他保険の形態
        c("dummy2", 18),             // ダミー
    ],
};
/// 荷為替輸入信用状発行依頼 data 取引情報(4) 発行依頼
static L31_DATA_6: Layout = Layout {
    fields: &[
        n("data_kind", 1),             // データ区分
        n("record_kind", 2),           // データ・レコード区分
        n("same_kind_count", 2),       // 同一種類データ・レコード数
        n("same_kind_no", 2),          // 同一種類データ・レコード番号
        n("transport_doc", 1),         // 運送書類
        c("transport_doc_other", 35),  // 運送書類 OTHER内容
        n("transport_mode", 1),        // 運送手段
        c("transport_mode_other", 35), // 運送手段 OTHER内容
        n("consignee", 1),             // 船荷証券等荷受人
        c("consignee_text", 105),      // 船荷証券等荷受人 ( )/OTHER内容
        n("freight", 1),               // 運賃支払
        c("freight_other", 35),        // 運賃支払 OTHER詳細
        c("dummy", 29),                // ダミー
    ],
};
/// 荷為替輸入信用状発行依頼 data 取引情報(5) 発行依頼
static L31_DATA_7: Layout = Layout {
    fields: &[
        n("data_kind", 1),            // データ区分
        n("record_kind", 2),          // データ・レコード区分
        n("same_kind_count", 2),      // 同一種類データ・レコード数
        n("same_kind_no", 2),         // 同一種類データ・レコード番号
        n("notify_party", 1),         // 船荷証券等通知先
        c("notify_party_other", 75),  // 船荷証券等通知先 OTHER詳細
        n("packing_list_copies", 2),  // ※包装明細書
        n("origin_cert_original", 2), // ※原産地証明書(1) (1)原紙の通数
        n("origin_cert_copy", 2),     // ※原産地証明書(1) (2)写しの通数
        n("gsp_cert_original", 2),    // ※原産地証明書(2) (1)原紙の通数
        n("gsp_cert_copy", 2),        // ※原産地証明書(2) (2)写しの通数
        c("other_docs1", 155),        // ※OTHER DOCUMENTS(1)
        c("dummy", 2),                // ダミー
    ],
};
/// 荷為替輸入信用状発行依頼 data 取引情報(6) 発行依頼
static L31_DATA_8: Layout = Layout {
    fields: &[
        n("data_kind", 1),             // データ区分
        n("record_kind", 2),           // データ・レコード区分
        n("same_kind_count", 2),       // 同一種類データ・レコード数
        n("same_kind_no", 2),          // 同一種類データ・レコード番号
        c("other_docs2", 120),         // ※OTHER DOCUMENTS(2)
        n("trade_terms", 1),           // 建値 (1)条件
        c("trade_terms_other", 10),    // 建値 (1)条件 OTHER内容
        c("trade_place", 30),          // 建値 (2)取引場所
        n("insured_by_applicant", 1),  // 保険会社について (1)輸入業者による付保
        c("insurer_name", 50),         // 保険会社について ※(2)保険会社名
        n("tt_reimbursement", 1),      // T.T REIMBURSEMENT
        n("charges_outside_japan", 1), // ALL BANKING CHARGE'S OUTSIDE JAPAN
        n("discount_charge", 1),       // ※DISCOUNT CHARGE(USANCE INT)
        n("acceptance_comm", 1),       // ※ACCEPTANCE COMM
        n("docs_lots", 1),             // ※書類送付方法等 (1)通数
        n("docs_dispatch", 1),         // ※書類送付方法等 (2)送付方法
        c("dummy", 25),                // ダミー
    ],
};
/// 荷為替輸入信用状発行依頼 data 取引情報(7) 発行依頼
static L31_DATA_9: Layout = Layout {
    fields: &[
        n("data_kind", 1),         // データ区分
        n("record_kind", 2),       // データ・レコード区分
        n("same_kind_count", 2),   // 同一種類データ・レコード数
        n("same_kind_no", 2),      // 同一種類データ・レコード番号
        c("request_to_bank", 150), // ※発行銀行への依頼事項
        c("dummy", 93),            // ダミー
    ],
};
/// 荷為替輸入信用状発行依頼 data 取引情報(1) 条件変更依頼
static L31_DATA_10: Layout = Layout {
    fields: &[
        n("data_kind", 1),               // データ区分
        n("record_kind", 2),             // データ・レコード区分
        n("same_kind_count", 2),         // 同一種類データ・レコード数
        n("same_kind_no", 2),            // 同一種類データ・レコード番号
        c("lc_currency", 3),             // ※信用状金額 (1)幣種
        n("lc_decimal", 1),              // ※信用状金額 (2)小数点位置
        n("lc_amount", 15),              // ※信用状金額 (3)金額
        n("lc_max_or_about", 1),         // ※信用状金額 (4)MAX OR ABOUT
        n("lc_pct", 2),                  // ※信用状金額 (4)MAX OR ABOUT ( )内の値
        n("amend_kind", 1),              // ※修正金額 (1)変更内容
        c("amend_currency", 3),          // ※修正金額 (2)幣種
        n("amend_decimal", 1),           // ※修正金額 (3)小数点位置
        n("amend_amount", 15),           // ※修正金額 (4)金額
        n("amend_max_or_about", 1),      // ※修正金額 (5)MAX OR ABOUT
        n("amend_pct", 2),               // ※修正金額 (5)MAX OR ABOUT ( )内の値
        c("new_currency", 3),            // ※修正後金額 (1)幣種
        n("new_decimal", 1),             // ※修正後金額 (2)小数点位置
        n("new_amount", 15),             // ※修正後金額 (3)金額
        n("new_max_or_about", 1),        // ※修正後金額 (4)MAX OR ABOUT
        n("new_pct", 2),                 // ※修正後金額 (4)MAX OR ABOUT ( )内の値
        n("tenor_availability", 1),      // TENORについて (1)資金化の条件
        n("tenor_draft_cond", 1),        // TENORについて (2)手形の条件
        c("tenor_draft_cond_other", 25), // TENORについて (2)手形の条件 OTHER詳細
        n("tenor_kind", 1),              // TENORについて (3)手形TENORの種類
        c("tenor_kind_other", 35),       // TENORについて (3)手形TENORの種類 OTHER詳細
        n("tenor_pct", 3),               // TENORについて (4)掛率
        n("tenor_wording", 70),          // TENORについて (5)記載文言
        n("tenor_drawee", 1),            // TENORについて ※(6)決済方法
        n("confirmation", 1),            // 確認の要否
        n("transferable", 1),            // 譲渡可能文言
        c("transfer_cond", 35),          // 譲渡可能文言
        c("dummy", 2),                   // ダミー
    ],
};
/// 荷為替輸入信用状発行依頼 data 取引情報(2) 条件変更依頼
static L31_DATA_11: Layout = Layout {
    fields: &[
        n("data_kind", 1),              // データ区分
        n("record_kind", 2),            // データ・レコード区分
        n("same_kind_count", 2),        // 同一種類データ・レコード数
        n("same_kind_no", 2),           // 同一種類データ・レコード番号
        n("nego_bank_designation", 1),  // ※買取銀行指定
        n("expiry_date", 8),            // ※有効期限 (1)有効期限
        c("expiry_place", 29),          // ※有効期限 (2)場所
        n("expiry_date_amended", 8),    // ※有効期限(条件変更時の変更後の期限)
        n("payable_at_destination", 1), // ※揚地払条件
        n("final_expiry_date", 8),      // ※最終有効期限
        n("shipment_date", 8),          // ※船積(積出)期限
        n("shipment_date_amended", 8),  // ※船積(積出)期限(条件変更時の変更後の期限)
        n("presentation_days", 2),      // 呈示期間(PRESENT DAY'S)
        n("partial_shipment", 1),       // ※分割積出文言(PARTIALSHIP区分)
        n("transhipment", 1),           // ※積換文言(TRANSHIP区分)
        c("ship_from", 65),             // ※発送場所
        c("ship_to", 65),               // ※到着場所
        c("dummy", 38),                 // ダミー
    ],
};
/// 荷為替輸入信用状発行依頼 data 内訳情報
static L31_DATA_12: Layout = Layout {
    fields: &[
        n("data_kind", 1),       // データ区分
        n("record_kind", 2),     // データ・レコード区分
        n("same_kind_count", 2), // 同一種類データ・レコード数
        n("same_kind_no", 2),    // 同一種類データ・レコード番号
        n("var_line_count", 2),  // 可変項目行数
        n("var_total_len", 3),   // レングス合計
        c("var_items", 238),     // 可変項目（項番7 ※商品情報, 項番8 ※追加条件情報, 項番9 ダミー）
    ],
};
/// 荷為替輸入信用状発行依頼 trailer
static L31_TRAILER: Layout = Layout {
    fields: &[
        n("data_kind", 1),          // データ区分
        n("data_record_count", 10), // データ・レコード総件数
        n("body_count", 10),        // 本体情報総件数
        n("issue_count", 10),       // 本体情報総件数 うち発行依頼分
        n("amend_count", 10),       // 本体情報総件数 うち条件変更依頼分
        c("dummy", 209),            // ダミー
    ],
};
/// 荷為替輸入信用状発行依頼 end
static L31_END: Layout = Layout {
    fields: &[
        n("data_kind", 1), // データ区分
        c("dummy", 249),   // ダミー
    ],
};
/// 外貨預金入出金取引明細 header
static L32_HEADER: Layout = Layout {
    fields: &[
        n("data_kind", 1),            // データ区分
        n("type", 2),                 // 種別コード
        n("code", 1),                 // コード区分
        n("created_on", 8),           // 作成日
        n("notified_on", 8),          // ※連絡日
        n("notify_seq", 2),           // ※連絡回数
        n("date_from", 8),            // 勘定日(自)
        n("date_to", 8),              // 勘定日(至)
        n("bank_code", 4),            // 銀行コード
        c("bank_name", 15),           // 銀行名
        n("agent_branch_code", 3),    // ※支店(取次店)コード
        c("agent_branch_name", 15),   // ※支店(取次店)名
        n("branch_code", 3),          // 支店(口座店)コード
        c("branch_name", 15),         // 支店(口座店)名
        n("deposit_type", 1),         // 預金種目
        n("account_no", 10),          // 口座番号
        c("account_name", 40),        // 口座名
        c("currency", 3),             // ※幣種
        n("prev_balance_decimal", 1), // 取引前残高 (1)※小数点位置
        n("prev_balance", 15),        // 取引前残高 (2)※金額
        n("prev_overdraft", 1),       // ※貸越区分
        c("dummy", 86),               // ダミー
    ],
};
/// 外貨預金入出金取引明細 data 本体情報(1) 普通・当座・貯蓄
static L32_DATA_0: Layout = Layout {
    fields: &[
        n("data_kind", 1),        // データ区分
        n("record_kind", 2),      // データ・レコード区分
        n("composition_kind", 2), // データ・レコード構成区分
        n("seq", 8),              // 処理通番
        n("account_date", 8),     // 勘定日
        n("value_date", 8),       // 起算日
        n("in_out", 1),           // 入払区分
        n("txn_kind", 2),         // 取引区分
        c("currency", 3),         // 取引金額 (1)幣種
        n("decimal", 1),          // 取引金額 (2)小数点位置
        n("amount", 15),          // 取引金額 (3)金額
        c("bank_ref", 16),        // ※銀行照会番号
        c("customer_ref", 32),    // ※取引先照会番号
        c("description", 48),     // ※摘要
        c("dummy", 103),          // ダミー
    ],
};
/// 外貨預金入出金取引明細 data 本体情報(2) 通知・定期・積立定期
static L32_DATA_1: Layout = Layout {
    fields: &[
        n("data_kind", 1),               // データ区分
        n("record_kind", 2),             // データ・レコード区分
        n("composition_kind", 2),        // データ・レコード構成区分
        n("seq", 8),                     // 処理通番
        n("account_date", 8),            // 勘定日
        n("value_date", 8),              // 起算日
        n("in_out", 1),                  // 入払区分
        n("txn_kind", 2),                // 取引区分
        c("currency", 3),                // 取引金額 (1)幣種
        n("decimal", 1),                 // 取引金額 (2)小数点位置
        n("amount", 15),                 // 取引金額 (3)金額
        n("deposit_no", 6),              // ※預入番号
        c("bank_ref", 16),               // ※銀行照会番号
        c("customer_ref", 32),           // ※取引先照会番号
        n("original_deposit_date", 8),   // ※当初預入日
        n("interest_rate", 9),           // 利率
        n("maturity_date", 8),           // ※満期日
        n("term", 8),                    // ※期間
        n("term_interest", 15),          // ※期間利息
        n("post_maturity_days", 4),      // ※期後期間
        n("post_maturity_rate", 9),      // ※期後利率
        n("post_maturity_interest", 15), // ※期後利息
        n("total_interest", 15),         // ※合計利息
        n("tax_kind", 1),                // ※税区分
        n("tax_rate", 4),                // ※税率
        n("tax_amount", 10),             // ※税額
        n("net_interest", 15),           // ※税引後利息
        c("description", 20),            // ※摘要
        c("dummy", 4),                   // ダミー
    ],
};
/// 外貨預金入出金取引明細 data 内訳情報
static L32_DATA_2: Layout = Layout {
    fields: &[
        n("data_kind", 1),                 // データ区分
        n("record_kind", 2),               // データ・レコード区分
        n("same_kind_count", 2),           // 同一種類データ・レコード数
        n("same_kind_no", 2),              // 同一種類データ・レコード番号
        n("brk1_rate_kind", 2),            // 取引金額内訳(1) 相場区分
        c("brk1_currency", 3),             // 取引金額内訳(1) 取引内訳金額 (1)幣種
        n("brk1_decimal", 1),              // 取引金額内訳(1) 取引内訳金額 (2)小数点位置
        n("brk1_amount", 15),              // 取引金額内訳(1) 取引内訳金額 (3)金額
        c("brk1_contract_no", 16),         // 取引金額内訳(1) ※予約番号
        n("brk1_rate_unit", 1),            // 取引金額内訳(1) 換算相場 (1)※通貨単位
        n("brk1_rate_decimal", 1),         // 取引金額内訳(1) 換算相場 (2)※小数点位置
        n("brk1_rate", 8),                 // 取引金額内訳(1) 換算相場 (3)※相場
        c("brk1_conv_currency", 3),        // 取引金額内訳(1) 換算額 (1)※幣種
        n("brk1_conv_decimal", 1),         // 取引金額内訳(1) 換算額 (2)※小数点位置
        n("brk1_conv_amount", 15),         // 取引金額内訳(1) 換算額 (3)※金額
        n("brk1_counter_deposit_type", 1), // 取引金額内訳(1) ※相手科目(預金種目)
        n("brk1_counter_account_no", 10),  // 取引金額内訳(1) ※相手口座番号
        n("brk2_rate_kind", 2),            // 取引金額内訳(2) 相場区分
        c("brk2_currency", 3),             // 取引金額内訳(2) 取引内訳金額 (1)幣種
        n("brk2_decimal", 1),              // 取引金額内訳(2) 取引内訳金額 (2)小数点位置
        n("brk2_amount", 15),              // 取引金額内訳(2) 取引内訳金額 (3)金額
        c("brk2_contract_no", 16),         // 取引金額内訳(2) ※予約番号
        n("brk2_rate_unit", 1),            // 取引金額内訳(2) 換算相場 (1)※通貨単位
        n("brk2_rate_decimal", 1),         // 取引金額内訳(2) 換算相場 (2)※小数点位置
        n("brk2_rate", 8),                 // 取引金額内訳(2) 換算相場 (3)※相場
        c("brk2_conv_currency", 3),        // 取引金額内訳(2) 換算額 (1)※幣種
        n("brk2_conv_decimal", 1),         // 取引金額内訳(2) 換算額 (2)※小数点位置
        n("brk2_conv_amount", 15),         // 取引金額内訳(2) 換算額 (3)※金額
        n("brk2_counter_deposit_type", 1), // 取引金額内訳(2) ※相手科目(預金種目)
        n("brk2_counter_account_no", 10),  // 取引金額内訳(2) ※相手口座番号
        n("brk3_rate_kind", 2),            // 取引金額内訳(3) 相場区分
        c("brk3_currency", 3),             // 取引金額内訳(3) 取引内訳金額 (1)幣種
        n("brk3_decimal", 1),              // 取引金額内訳(3) 取引内訳金額 (2)小数点位置
        n("brk3_amount", 15),              // 取引金額内訳(3) 取引内訳金額 (3)金額
        c("brk3_contract_no", 16),         // 取引金額内訳(3) ※予約番号
        n("brk3_rate_unit", 1),            // 取引金額内訳(3) 換算相場 (1)※通貨単位
        n("brk3_rate_decimal", 1),         // 取引金額内訳(3) 換算相場 (2)※小数点位置
        n("brk3_rate", 8),                 // 取引金額内訳(3) 換算相場 (3)※相場
        c("brk3_conv_currency", 3),        // 取引金額内訳(3) 換算額 (1)※幣種
        n("brk3_conv_decimal", 1),         // 取引金額内訳(3) 換算額 (2)※小数点位置
        n("brk3_conv_amount", 15),         // 取引金額内訳(3) 換算額 (3)※金額
        n("brk3_counter_deposit_type", 1), // 取引金額内訳(3) ※相手科目(預金種目)
        n("brk3_counter_account_no", 10),  // 取引金額内訳(3) ※相手口座番号
        c("dummy", 12),                    // ダミー
    ],
};
/// 外貨預金入出金取引明細 trailer
static L32_TRAILER: Layout = Layout {
    fields: &[
        n("data_kind", 1),           // データ区分
        n("deposit_count", 6),       // 入金件数
        c("deposit_currency", 3),    // 入金額合計 (1)幣種
        n("deposit_decimal", 1),     // 入金額合計 (2)小数点位置
        n("deposit_amount", 15),     // 入金額合計 (3)金額
        n("withdrawal_count", 6),    // 出金件数
        c("withdrawal_currency", 3), // 出金額合計 (1)幣種
        n("withdrawal_decimal", 1),  // 出金額合計 (2)小数点位置
        n("withdrawal_amount", 15),  // 出金額合計 (3)金額
        c("currency", 3),            // ※幣種
        n("balance_decimal", 1),     // 取引後残高 (1)※小数点位置
        n("balance", 15),            // 取引後残高 (2)※金額
        n("overdraft", 1),           // ※貸越区分
        n("data_record_count", 7),   // データ・レコード件数
        c("dummy", 172),             // ダミー
    ],
};
/// 外貨預金入出金取引明細 end
static L32_END: Layout = Layout {
    fields: &[
        n("data_kind", 1),          // データ区分
        n("total_record_count", 7), // レコード総件数
        n("account_count", 5),      // 口座数または通貨別口座数
        c("dummy", 237),            // ダミー
    ],
};

pub static GENERATED: [Fmt; 29] = [
    Fmt {
        kind: FileKind::Meisai,
        title: "入出金取引明細",
        rec_len: 200,
        type_code: 3,
        name_kind: "50200003",
        inquiry: true,
        header: &L02_HEADER,
        datas: &[
            Var {
                label: "普通・当座・貯蓄",
                layout: &L02_DATA_0,
                kinds: &[],
                header: Some(("acct_kind", &["1", "2", "4"])),
                ctx: None,
            },
            Var {
                label: "通知・定期・積立定期",
                layout: &L02_DATA_1,
                kinds: &[],
                header: Some(("acct_kind", &["5", "6", "7"])),
                ctx: None,
            },
        ],
        trailer: &L02_TRAILER,
        end: &L02_END,
        checks: &[
            Check {
                field: "count",
                agg: Agg::Count,
                when: &[],
            },
            Check {
                field: "in_count",
                agg: Agg::Count,
                when: &[("io_kind", true, "1")],
            },
            Check {
                field: "in_amount",
                agg: Agg::Sum("amount"),
                when: &[("io_kind", true, "1")],
            },
            Check {
                field: "out_count",
                agg: Agg::Count,
                when: &[("io_kind", true, "2")],
            },
            Check {
                field: "out_amount",
                agg: Agg::Sum("amount"),
                when: &[("io_kind", true, "2")],
            },
        ],
    },
    Fmt {
        kind: FileKind::Zandaka,
        title: "残高通知（預金）",
        rec_len: 200,
        type_code: 4,
        name_kind: "50200004",
        inquiry: true,
        header: &L03_HEADER,
        datas: &[Var {
            label: "",
            layout: &L03_DATA_0,
            kinds: &[],
            header: None,
            ctx: None,
        }],
        trailer: &L03_TRAILER,
        end: &L03_END,
        checks: &[Check {
            field: "count",
            agg: Agg::Count,
            when: &[],
        }],
    },
    Fmt {
        kind: FileKind::Shoyo,
        title: "賞与振込（民間）",
        rec_len: 120,
        type_code: 12,
        name_kind: "50200112",
        inquiry: false,
        header: &L05_HEADER,
        datas: &[Var {
            label: "",
            layout: &L05_DATA_0,
            kinds: &[],
            header: None,
            ctx: None,
        }],
        trailer: &L05_TRAILER,
        end: &L05_END,
        checks: &[
            Check {
                field: "count",
                agg: Agg::Count,
                when: &[],
            },
            Check {
                field: "amount",
                agg: Agg::Sum("amount"),
                when: &[],
            },
        ],
    },
    Fmt {
        kind: FileKind::ChihoKyuyo,
        title: "給与振込（地方公務員）",
        rec_len: 120,
        type_code: 71,
        name_kind: "50200171",
        inquiry: false,
        header: &L06_HEADER,
        datas: &[Var {
            label: "",
            layout: &L06_DATA_0,
            kinds: &[],
            header: None,
            ctx: None,
        }],
        trailer: &L06_TRAILER,
        end: &L06_END,
        checks: &[
            Check {
                field: "count",
                agg: Agg::Count,
                when: &[],
            },
            Check {
                field: "amount",
                agg: Agg::Sum("amount"),
                when: &[],
            },
        ],
    },
    Fmt {
        kind: FileKind::ChihoShoyo,
        title: "賞与振込（地方公務員）",
        rec_len: 120,
        type_code: 72,
        name_kind: "50200172",
        inquiry: false,
        header: &L07_HEADER,
        datas: &[Var {
            label: "",
            layout: &L07_DATA_0,
            kinds: &[],
            header: None,
            ctx: None,
        }],
        trailer: &L07_TRAILER,
        end: &L07_END,
        checks: &[
            Check {
                field: "count",
                agg: Agg::Count,
                when: &[],
            },
            Check {
                field: "amount",
                agg: Agg::Sum("amount"),
                when: &[],
            },
        ],
    },
    Fmt {
        kind: FileKind::Haito,
        title: "株式配当金振込",
        rec_len: 120,
        type_code: 41,
        name_kind: "50200141",
        inquiry: false,
        header: &L09_HEADER,
        datas: &[Var {
            label: "",
            layout: &L09_DATA_0,
            kinds: &[],
            header: None,
            ctx: None,
        }],
        trailer: &L09_TRAILER,
        end: &L09_END,
        checks: &[
            Check {
                field: "count",
                agg: Agg::Count,
                when: &[],
            },
            Check {
                field: "amount",
                agg: Agg::Sum("amount"),
                when: &[],
            },
        ],
    },
    Fmt {
        kind: FileKind::NenkinShintaku,
        title: "年金信託契約に係る年金・一時金給付金振込",
        rec_len: 120,
        type_code: 43,
        name_kind: "50200143",
        inquiry: false,
        header: &L10_HEADER,
        datas: &[Var {
            label: "",
            layout: &L10_DATA_0,
            kinds: &[],
            header: None,
            ctx: None,
        }],
        trailer: &L10_TRAILER,
        end: &L10_END,
        checks: &[
            Check {
                field: "count",
                agg: Agg::Count,
                when: &[],
            },
            Check {
                field: "amount",
                agg: Agg::Sum("amount"),
                when: &[],
            },
        ],
    },
    Fmt {
        kind: FileKind::KotekiNenkin,
        title: "公的年金保険の年金・一時金給付金振込",
        rec_len: 120,
        type_code: 44,
        name_kind: "50200144",
        inquiry: false,
        header: &L11_HEADER,
        datas: &[Var {
            label: "",
            layout: &L11_DATA_0,
            kinds: &[],
            header: None,
            ctx: None,
        }],
        trailer: &L11_TRAILER,
        end: &L11_END,
        checks: &[
            Check {
                field: "count",
                agg: Agg::Count,
                when: &[],
            },
            Check {
                field: "amount",
                agg: Agg::Sum("amount"),
                when: &[],
            },
        ],
    },
    Fmt {
        kind: FileKind::Iryo,
        title: "医療保険の給付金振込",
        rec_len: 120,
        type_code: 45,
        name_kind: "50200145",
        inquiry: false,
        header: &L12_HEADER,
        datas: &[Var {
            label: "",
            layout: &L12_DATA_0,
            kinds: &[],
            header: None,
            ctx: None,
        }],
        trailer: &L12_TRAILER,
        end: &L12_END,
        checks: &[
            Check {
                field: "count",
                agg: Agg::Count,
                when: &[],
            },
            Check {
                field: "amount",
                agg: Agg::Sum("amount"),
                when: &[],
            },
        ],
    },
    Fmt {
        kind: FileKind::KozaShokai,
        title: "振込口座照会（依頼明細）",
        rec_len: 120,
        type_code: 98,
        name_kind: "50200198",
        inquiry: false,
        header: &L13_HEADER,
        datas: &[Var {
            label: "",
            layout: &L13_DATA_0,
            kinds: &[],
            header: None,
            ctx: None,
        }],
        trailer: &L13_TRAILER,
        end: &L13_END,
        checks: &[Check {
            field: "count",
            agg: Agg::Count,
            when: &[],
        }],
    },
    Fmt {
        kind: FileKind::KozaShokaiKekka,
        title: "振込口座照会（処理結果明細）",
        rec_len: 120,
        type_code: 99,
        name_kind: "50200199",
        inquiry: true,
        header: &L14_HEADER,
        datas: &[Var {
            label: "",
            layout: &L14_DATA_0,
            kinds: &[],
            header: None,
            ctx: None,
        }],
        trailer: &L14_TRAILER,
        end: &L14_END,
        checks: &[Check {
            field: "count",
            agg: Agg::Count,
            when: &[],
        }],
    },
    Fmt {
        kind: FileKind::Furikae,
        title: "預金口座振替（依頼明細）",
        rec_len: 120,
        type_code: 91,
        name_kind: "50200191",
        inquiry: false,
        header: &L15_HEADER,
        datas: &[Var {
            label: "",
            layout: &L15_DATA_0,
            kinds: &[],
            header: None,
            ctx: None,
        }],
        trailer: &L15_TRAILER,
        end: &L15_END,
        checks: &[
            Check {
                field: "count",
                agg: Agg::Count,
                when: &[],
            },
            Check {
                field: "amount",
                agg: Agg::Sum("amount"),
                when: &[],
            },
        ],
    },
    Fmt {
        kind: FileKind::FurikaeKekka,
        title: "預金口座振替（処理結果明細）",
        rec_len: 120,
        type_code: 91,
        name_kind: "50200191",
        inquiry: true,
        header: &L16_HEADER,
        datas: &[Var {
            label: "",
            layout: &L16_DATA_0,
            kinds: &[],
            header: None,
            ctx: None,
        }],
        trailer: &L16_TRAILER,
        end: &L16_END,
        checks: &[
            Check {
                field: "count",
                agg: Agg::Count,
                when: &[],
            },
            Check {
                field: "amount",
                agg: Agg::Sum("amount"),
                when: &[],
            },
            Check {
                field: "ok_count",
                agg: Agg::Count,
                when: &[("result_code", true, "0")],
            },
            Check {
                field: "ok_amount",
                agg: Agg::Sum("amount"),
                when: &[("result_code", true, "0")],
            },
            Check {
                field: "ng_count",
                agg: Agg::Count,
                when: &[("result_code", false, "0")],
            },
            Check {
                field: "ng_amount",
                agg: Agg::Sum("amount"),
                when: &[("result_code", false, "0")],
            },
        ],
    },
    Fmt {
        kind: FileKind::FurikaeHenko,
        title: "預金口座振替変更通知",
        rec_len: 120,
        type_code: 93,
        name_kind: "50200193",
        inquiry: true,
        header: &L17_HEADER,
        datas: &[Var {
            label: "",
            layout: &L17_DATA_0,
            kinds: &[],
            header: None,
            ctx: None,
        }],
        trailer: &L17_TRAILER,
        end: &L17_END,
        checks: &[Check {
            field: "count",
            agg: Agg::Count,
            when: &[],
        }],
    },
    Fmt {
        kind: FileKind::LoanZaikei,
        title: "住宅ローン関係（財形ローン）",
        rec_len: 550,
        type_code: 11,
        name_kind: "50200211",
        inquiry: true,
        header: &L18_HEADER,
        datas: &[Var {
            label: "",
            layout: &L18_DATA_0,
            kinds: &[],
            header: None,
            ctx: None,
        }],
        trailer: &L18_TRAILER,
        end: &L18_END,
        checks: &[Check {
            field: "count",
            agg: Agg::Count,
            when: &[],
        }],
    },
    Fmt {
        kind: FileKind::LoanSonota,
        title: "住宅ローン関係（その他）",
        rec_len: 550,
        type_code: 12,
        name_kind: "50200212",
        inquiry: true,
        header: &L19_HEADER,
        datas: &[Var {
            label: "",
            layout: &L19_DATA_0,
            kinds: &[],
            header: None,
            ctx: None,
        }],
        trailer: &L19_TRAILER,
        end: &L19_END,
        checks: &[Check {
            field: "count",
            agg: Agg::Count,
            when: &[],
        }],
    },
    Fmt {
        kind: FileKind::Zaikei,
        title: "財形貯蓄関係（依頼明細）",
        rec_len: 300,
        type_code: 0,
        name_kind: "50200300",
        inquiry: false,
        header: &L20_HEADER,
        datas: &[Var {
            label: "",
            layout: &L20_DATA_0,
            kinds: &[],
            header: None,
            ctx: None,
        }],
        trailer: &L20_TRAILER,
        end: &L20_END,
        checks: &[
            Check {
                field: "count",
                agg: Agg::Count,
                when: &[],
            },
            Check {
                field: "salary_total",
                agg: Agg::Sum("salary_amount"),
                when: &[],
            },
            Check {
                field: "bonus_total",
                agg: Agg::Sum("bonus_amount"),
                when: &[],
            },
            Check {
                field: "other1_total",
                agg: Agg::Sum("other1_amount"),
                when: &[],
            },
            Check {
                field: "other2_total",
                agg: Agg::Sum("other2_amount"),
                when: &[],
            },
        ],
    },
    Fmt {
        kind: FileKind::ZaikeiTsumitate,
        title: "財形貯蓄関係（処理結果明細：積立内容）",
        rec_len: 300,
        type_code: 1,
        name_kind: "50200301",
        inquiry: true,
        header: &L21_HEADER,
        datas: &[Var {
            label: "",
            layout: &L21_DATA_0,
            kinds: &[],
            header: None,
            ctx: None,
        }],
        trailer: &L21_TRAILER,
        end: &L21_END,
        checks: &[
            Check {
                field: "count",
                agg: Agg::Count,
                when: &[],
            },
            Check {
                field: "balance_total",
                agg: Agg::Sum("balance"),
                when: &[],
            },
            Check {
                field: "salary_total",
                agg: Agg::Sum("salary_amount"),
                when: &[],
            },
            Check {
                field: "bonus_total",
                agg: Agg::Sum("bonus_amount"),
                when: &[],
            },
            Check {
                field: "other1_total",
                agg: Agg::Sum("other1_amount"),
                when: &[],
            },
            Check {
                field: "other2_total",
                agg: Agg::Sum("other2_amount"),
                when: &[],
            },
            Check {
                field: "subsidy_base_balance_total",
                agg: Agg::Sum("subsidy_base_balance"),
                when: &[],
            },
            Check {
                field: "subsidy_amount_total",
                agg: Agg::Sum("subsidy_amount"),
                when: &[],
            },
            Check {
                field: "subsidy_cumulative_total",
                agg: Agg::Sum("subsidy_cumulative"),
                when: &[],
            },
        ],
    },
    Fmt {
        kind: FileKind::ZaikeiRishi,
        title: "財形貯蓄関係（処理結果明細：利子補給）",
        rec_len: 300,
        type_code: 2,
        name_kind: "50200302",
        inquiry: true,
        header: &L22_HEADER,
        datas: &[Var {
            label: "",
            layout: &L22_DATA_0,
            kinds: &[],
            header: None,
            ctx: None,
        }],
        trailer: &L22_TRAILER,
        end: &L22_END,
        checks: &[
            Check {
                field: "count",
                agg: Agg::Count,
                when: &[],
            },
            Check {
                field: "balance_total",
                agg: Agg::Sum("balance"),
                when: &[],
            },
            Check {
                field: "salary_total",
                agg: Agg::Sum("salary_amount"),
                when: &[],
            },
            Check {
                field: "bonus_total",
                agg: Agg::Sum("bonus_amount"),
                when: &[],
            },
            Check {
                field: "other1_total",
                agg: Agg::Sum("other1_amount"),
                when: &[],
            },
            Check {
                field: "other2_total",
                agg: Agg::Sum("other2_amount"),
                when: &[],
            },
            Check {
                field: "subsidy_base_balance_total",
                agg: Agg::Sum("subsidy_base_balance"),
                when: &[],
            },
            Check {
                field: "subsidy_amount_total",
                agg: Agg::Sum("subsidy_amount"),
                when: &[],
            },
            Check {
                field: "subsidy_cumulative_total",
                agg: Agg::Sum("subsidy_cumulative"),
                when: &[],
            },
        ],
    },
    Fmt {
        kind: FileKind::ZaikeiSonota,
        title: "財形貯蓄関係（処理結果明細：その他）",
        rec_len: 300,
        type_code: 9,
        name_kind: "50200309",
        inquiry: true,
        header: &L23_HEADER,
        datas: &[Var {
            label: "",
            layout: &L23_DATA_0,
            kinds: &[],
            header: None,
            ctx: None,
        }],
        trailer: &L23_TRAILER,
        end: &L23_END,
        checks: &[
            Check {
                field: "count",
                agg: Agg::Count,
                when: &[],
            },
            Check {
                field: "balance_total",
                agg: Agg::Sum("balance"),
                when: &[],
            },
            Check {
                field: "salary_total",
                agg: Agg::Sum("salary_amount"),
                when: &[],
            },
            Check {
                field: "bonus_total",
                agg: Agg::Sum("bonus_amount"),
                when: &[],
            },
            Check {
                field: "other1_total",
                agg: Agg::Sum("other1_amount"),
                when: &[],
            },
            Check {
                field: "other2_total",
                agg: Agg::Sum("other2_amount"),
                when: &[],
            },
            Check {
                field: "subsidy_base_balance_total",
                agg: Agg::Sum("subsidy_base_balance"),
                when: &[],
            },
            Check {
                field: "subsidy_amount_total",
                agg: Agg::Sum("subsidy_amount"),
                when: &[],
            },
            Check {
                field: "subsidy_cumulative_total",
                agg: Agg::Sum("subsidy_cumulative"),
                when: &[],
            },
        ],
    },
    Fmt {
        kind: FileKind::KariireKeisan,
        title: "借入金計算書",
        rec_len: 250,
        type_code: 31,
        name_kind: "50200231",
        inquiry: true,
        header: &L24_HEADER,
        datas: &[
            Var {
                label: "loan",
                layout: &L24_DATA_0,
                kinds: &["10", "20"],
                header: None,
                ctx: None,
            },
            Var {
                label: "discount_detail",
                layout: &L24_DATA_1,
                kinds: &["30"],
                header: None,
                ctx: None,
            },
            Var {
                label: "discount_total",
                layout: &L24_DATA_2,
                kinds: &["31"],
                header: None,
                ctx: None,
            },
        ],
        trailer: &L24_TRAILER,
        end: &L24_END,
        checks: &[Check {
            field: "count",
            agg: Agg::Count,
            when: &[],
        }],
    },
    Fmt {
        kind: FileKind::KariireZandaka,
        title: "借入金残高明細",
        rec_len: 250,
        type_code: 32,
        name_kind: "50200232",
        inquiry: true,
        header: &L25_HEADER,
        datas: &[
            Var {
                label: "loan",
                layout: &L25_DATA_0,
                kinds: &["10", "20"],
                header: None,
                ctx: None,
            },
            Var {
                label: "discount",
                layout: &L25_DATA_1,
                kinds: &["30"],
                header: None,
                ctx: None,
            },
        ],
        trailer: &L25_TRAILER,
        end: &L25_END,
        checks: &[Check {
            field: "count",
            agg: Agg::Count,
            when: &[],
        }],
    },
    Fmt {
        kind: FileKind::KariireGokei,
        title: "借入金残高合計",
        rec_len: 250,
        type_code: 33,
        name_kind: "50200233",
        inquiry: true,
        header: &L26_HEADER,
        datas: &[Var {
            label: "",
            layout: &L26_DATA_0,
            kinds: &[],
            header: None,
            ctx: None,
        }],
        trailer: &L26_TRAILER,
        end: &L26_END,
        checks: &[Check {
            field: "count",
            agg: Agg::Count,
            when: &[],
        }],
    },
    Fmt {
        kind: FileKind::FxKaikei,
        title: "外国為替取引明細（会計性）",
        rec_len: 250,
        type_code: 25,
        name_kind: "50200425",
        inquiry: true,
        header: &L27_HEADER,
        datas: &[
            Var {
                label: "01 本体情報",
                layout: &L27_DATA_0,
                kinds: &["01"],
                header: None,
                ctx: None,
            },
            Var {
                label: "11 取引情報［輸出手形買取］",
                layout: &L27_DATA_1,
                kinds: &["11"],
                header: None,
                ctx: None,
            },
            Var {
                label: "21 取引情報［輸出手形買取内訳］",
                layout: &L27_DATA_2,
                kinds: &["21"],
                header: None,
                ctx: None,
            },
            Var {
                label: "12 取引情報［輸入手形決済］",
                layout: &L27_DATA_3,
                kinds: &["12"],
                header: None,
                ctx: None,
            },
            Var {
                label: "22 取引情報［輸入手形決済内訳］",
                layout: &L27_DATA_4,
                kinds: &["22"],
                header: None,
                ctx: None,
            },
            Var {
                label: "13 取引情報［輸出手形取立支払］",
                layout: &L27_DATA_5,
                kinds: &["13"],
                header: None,
                ctx: None,
            },
            Var {
                label: "23 取引情報［輸出手形取立支払内訳］",
                layout: &L27_DATA_6,
                kinds: &["23"],
                header: None,
                ctx: None,
            },
            Var {
                label: "14 取引情報［仕向送金取組］",
                layout: &L27_DATA_7,
                kinds: &["14"],
                header: None,
                ctx: None,
            },
            Var {
                label: "24 取引情報［仕向送金取組内訳］",
                layout: &L27_DATA_8,
                kinds: &["24"],
                header: None,
                ctx: None,
            },
            Var {
                label: "15 取引情報［被仕向送金支払］",
                layout: &L27_DATA_9,
                kinds: &["15"],
                header: None,
                ctx: None,
            },
            Var {
                label: "25 取引情報［被仕向送金支払内訳］",
                layout: &L27_DATA_10,
                kinds: &["25"],
                header: None,
                ctx: None,
            },
            Var {
                label: "91 取引情報［利息・手数料情報（都度請求）］",
                layout: &L27_DATA_11,
                kinds: &["91"],
                header: None,
                ctx: None,
            },
            Var {
                label: "92 利息・手数料",
                layout: &L27_DATA_12,
                kinds: &["92"],
                header: None,
                ctx: None,
            },
            Var {
                label: "93 利息・手数料一括請求［合計］",
                layout: &L27_DATA_13,
                kinds: &["93"],
                header: None,
                ctx: None,
            },
            Var {
                label: "94 利息・手数料一括請求［内訳］",
                layout: &L27_DATA_14,
                kinds: &["94"],
                header: None,
                ctx: None,
            },
            Var {
                label: "51 為替予約［先物予約締結］",
                layout: &L27_DATA_15,
                kinds: &["51"],
                header: None,
                ctx: None,
            },
            Var {
                label: "52 為替予約［先物予約履行］",
                layout: &L27_DATA_16,
                kinds: &["52"],
                header: None,
                ctx: None,
            },
            Var {
                label: "53 為替予約［先物予約残高明細］",
                layout: &L27_DATA_17,
                kinds: &["53"],
                header: None,
                ctx: None,
            },
        ],
        trailer: &L27_TRAILER,
        end: &L27_END,
        checks: &[Check {
            field: "record_count",
            agg: Agg::Count,
            when: &[],
        }],
    },
    Fmt {
        kind: FileKind::FxHikaikei,
        title: "外国為替取引明細（非会計性）",
        rec_len: 250,
        type_code: 26,
        name_kind: "50200426",
        inquiry: true,
        header: &L28_HEADER,
        datas: &[
            Var {
                label: "02 輸出信用状到着案内（固定レコード）",
                layout: &L28_DATA_0,
                kinds: &["02"],
                header: None,
                ctx: None,
            },
            Var {
                label: "62 輸出信用状到着案内（可変レコード）",
                layout: &L28_DATA_1,
                kinds: &["62"],
                header: None,
                ctx: None,
            },
            Var {
                label: "03 輸入船積書類到着案内（固定レコード）",
                layout: &L28_DATA_2,
                kinds: &["03"],
                header: None,
                ctx: None,
            },
            Var {
                label: "63 輸入船積書類到着案内（可変レコード）",
                layout: &L28_DATA_3,
                kinds: &["63"],
                header: None,
                ctx: None,
            },
            Var {
                label: "06 被仕向送金到着案内（固定レコード）",
                layout: &L28_DATA_4,
                kinds: &["06"],
                header: None,
                ctx: None,
            },
            Var {
                label: "66 被仕向送金到着案内（可変レコード）",
                layout: &L28_DATA_5,
                kinds: &["66"],
                header: None,
                ctx: None,
            },
            Var {
                label: "07 輸出手形経過情報（固定レコード）",
                layout: &L28_DATA_6,
                kinds: &["07"],
                header: None,
                ctx: None,
            },
            Var {
                label: "67 輸出手形経過情報（可変レコード）",
                layout: &L28_DATA_7,
                kinds: &["67"],
                header: None,
                ctx: None,
            },
            Var {
                label: "92 利息・手数料",
                layout: &L28_DATA_8,
                kinds: &["92"],
                header: None,
                ctx: None,
            },
        ],
        trailer: &L28_TRAILER,
        end: &L28_END,
        checks: &[Check {
            field: "record_count",
            agg: Agg::Count,
            when: &[],
        }],
    },
    Fmt {
        kind: FileKind::FxKanren,
        title: "外国為替関連情報",
        rec_len: 250,
        type_code: 27,
        name_kind: "50200427",
        inquiry: true,
        header: &L29_HEADER,
        datas: &[Var {
            label: "外国為替公示相場",
            layout: &L29_DATA_0,
            kinds: &["01"],
            header: None,
            ctx: None,
        }],
        trailer: &L29_TRAILER,
        end: &L29_END,
        checks: &[Check {
            field: "count",
            agg: Agg::Count,
            when: &[],
        }],
    },
    Fmt {
        kind: FileKind::Soukin,
        title: "外国送金依頼",
        rec_len: 250,
        type_code: 35,
        name_kind: "50200435",
        inquiry: false,
        header: &L30_HEADER,
        datas: &[
            Var {
                label: "本体情報",
                layout: &L30_DATA_0,
                kinds: &["01"],
                header: None,
                ctx: None,
            },
            Var {
                label: "会計処理内訳",
                layout: &L30_DATA_1,
                kinds: &["02"],
                header: None,
                ctx: None,
            },
            Var {
                label: "送金指図内訳",
                layout: &L30_DATA_2,
                kinds: &["03"],
                header: None,
                ctx: None,
            },
        ],
        trailer: &L30_TRAILER,
        end: &L30_END,
        checks: &[
            Check {
                field: "data_record_count",
                agg: Agg::Count,
                when: &[],
            },
            Check {
                field: "body_count",
                agg: Agg::Count,
                when: &[("record_kind", true, "01")],
            },
        ],
    },
    Fmt {
        kind: FileKind::ShinyoJo,
        title: "荷為替輸入信用状発行依頼",
        rec_len: 250,
        type_code: 28,
        name_kind: "50200428",
        inquiry: false,
        header: &L31_HEADER,
        datas: &[
            Var {
                label: "本体情報(1)",
                layout: &L31_DATA_0,
                kinds: &["11"],
                header: None,
                ctx: None,
            },
            Var {
                label: "本体情報(2)",
                layout: &L31_DATA_1,
                kinds: &["12"],
                header: None,
                ctx: None,
            },
            Var {
                label: "本体情報(3)",
                layout: &L31_DATA_2,
                kinds: &["13"],
                header: None,
                ctx: None,
            },
            Var {
                label: "取引情報(1) 発行依頼",
                layout: &L31_DATA_3,
                kinds: &["21"],
                header: None,
                ctx: Some(("composition_kind", &["01"])),
            },
            Var {
                label: "取引情報(2) 発行依頼",
                layout: &L31_DATA_4,
                kinds: &["22"],
                header: None,
                ctx: Some(("composition_kind", &["01"])),
            },
            Var {
                label: "取引情報(3) 発行依頼",
                layout: &L31_DATA_5,
                kinds: &["23"],
                header: None,
                ctx: None,
            },
            Var {
                label: "取引情報(4) 発行依頼",
                layout: &L31_DATA_6,
                kinds: &["24"],
                header: None,
                ctx: None,
            },
            Var {
                label: "取引情報(5) 発行依頼",
                layout: &L31_DATA_7,
                kinds: &["25"],
                header: None,
                ctx: None,
            },
            Var {
                label: "取引情報(6) 発行依頼",
                layout: &L31_DATA_8,
                kinds: &["26"],
                header: None,
                ctx: None,
            },
            Var {
                label: "取引情報(7) 発行依頼",
                layout: &L31_DATA_9,
                kinds: &["27"],
                header: None,
                ctx: None,
            },
            Var {
                label: "取引情報(1) 条件変更依頼",
                layout: &L31_DATA_10,
                kinds: &["21"],
                header: None,
                ctx: Some(("composition_kind", &["02"])),
            },
            Var {
                label: "取引情報(2) 条件変更依頼",
                layout: &L31_DATA_11,
                kinds: &["22"],
                header: None,
                ctx: Some(("composition_kind", &["02"])),
            },
            Var {
                label: "内訳情報",
                layout: &L31_DATA_12,
                kinds: &["30"],
                header: None,
                ctx: None,
            },
        ],
        trailer: &L31_TRAILER,
        end: &L31_END,
        checks: &[
            Check {
                field: "data_record_count",
                agg: Agg::Count,
                when: &[],
            },
            Check {
                field: "body_count",
                agg: Agg::Count,
                when: &[("record_kind", true, "11")],
            },
            Check {
                field: "issue_count",
                agg: Agg::Count,
                when: &[("record_kind", true, "11"), ("composition_kind", true, "01")],
            },
            Check {
                field: "amend_count",
                agg: Agg::Count,
                when: &[("record_kind", true, "11"), ("composition_kind", true, "02")],
            },
        ],
    },
    Fmt {
        kind: FileKind::GaikaMeisai,
        title: "外貨預金入出金取引明細",
        rec_len: 250,
        type_code: 9,
        name_kind: "50200009",
        inquiry: true,
        header: &L32_HEADER,
        datas: &[
            Var {
                label: "本体情報(1) 普通・当座・貯蓄",
                layout: &L32_DATA_0,
                kinds: &["01"],
                header: Some(("deposit_type", &["1", "2", "4"])),
                ctx: None,
            },
            Var {
                label: "本体情報(2) 通知・定期・積立定期",
                layout: &L32_DATA_1,
                kinds: &["01"],
                header: Some(("deposit_type", &["5", "6", "7"])),
                ctx: None,
            },
            Var {
                label: "内訳情報",
                layout: &L32_DATA_2,
                kinds: &["20"],
                header: None,
                ctx: None,
            },
        ],
        trailer: &L32_TRAILER,
        end: &L32_END,
        checks: &[
            Check {
                field: "data_record_count",
                agg: Agg::Count,
                when: &[],
            },
            Check {
                field: "deposit_count",
                agg: Agg::Count,
                when: &[("record_kind", true, "01"), ("in_out", true, "1")],
            },
            Check {
                field: "deposit_amount",
                agg: Agg::Sum("amount"),
                when: &[("record_kind", true, "01"), ("in_out", true, "1")],
            },
            Check {
                field: "withdrawal_count",
                agg: Agg::Count,
                when: &[("record_kind", true, "01"), ("in_out", true, "2")],
            },
            Check {
                field: "withdrawal_amount",
                agg: Agg::Sum("amount"),
                when: &[("record_kind", true, "01"), ("in_out", true, "2")],
            },
        ],
    },
];
