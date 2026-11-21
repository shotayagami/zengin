//! 固定長レコードと、ファイル (ヘッダ/データ/トレーラ/エンド) の組み立てと検証。
//! 業務のレイアウトは別冊。振込入金通知・給与振込・総合振込はここに、ほかは `formats.rs` (formats/*.json から生成)。
//! 為替通知 (91〜94) は docs/spec-net.md §3。

use crate::charcode::{Code, decode, encode};
use crate::util::{Result, ZError, proto};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum K {
    /// 数字。右詰め・前ゼロ
    N,
    /// 文字。左詰め・後ろスペース
    C,
}

#[derive(Debug)]
pub struct F {
    pub name: &'static str,
    pub k: K,
    pub len: usize,
}

pub(crate) const fn n(name: &'static str, len: usize) -> F {
    F { name, k: K::N, len }
}
pub(crate) const fn c(name: &'static str, len: usize) -> F {
    F { name, k: K::C, len }
}

#[derive(Debug)]
pub struct Layout {
    pub fields: &'static [F],
}

#[allow(clippy::len_without_is_empty)]
impl Layout {
    pub fn len(&self) -> usize {
        self.fields.iter().map(|f| f.len).sum()
    }
}

#[derive(Clone, Debug)]
pub struct Rec {
    pub layout: &'static Layout,
    vals: Vec<String>,
}

impl PartialEq for Rec {
    fn eq(&self, o: &Rec) -> bool {
        std::ptr::eq(self.layout, o.layout) && self.vals == o.vals
    }
}

impl Rec {
    pub fn new(layout: &'static Layout) -> Rec {
        Rec {
            layout,
            vals: layout
                .fields
                .iter()
                .map(|f| if f.k == K::N { "0".into() } else { String::new() })
                .collect(),
        }
    }

    fn idx(&self, name: &str) -> usize {
        self.layout
            .fields
            .iter()
            .position(|f| f.name == name)
            .unwrap_or_else(|| panic!("項目がない: {name}"))
    }

    /// このレコードの様式のラベル (`Fmt::datas` のどれか)。
    pub fn layout_label(&self, f: &Fmt) -> &'static str {
        f.datas.iter().find(|v| std::ptr::eq(v.layout, self.layout)).map_or("", |v| v.label)
    }

    pub fn has(&self, name: &str) -> bool {
        self.layout.fields.iter().any(|f| f.name == name)
    }

    pub fn get(&self, name: &str) -> &str {
        &self.vals[self.idx(name)]
    }

    pub fn num(&self, name: &str) -> u64 {
        self.get(name).parse().unwrap_or(0)
    }

    pub fn set(mut self, name: &str, v: impl Into<String>) -> Rec {
        let i = self.idx(name);
        self.vals[i] = v.into();
        self
    }

    pub fn set_num(self, name: &str, v: u64) -> Rec {
        self.set(name, v.to_string())
    }

    pub fn encode(&self, code: Code) -> Result<Vec<u8>> {
        let mut out = Vec::with_capacity(self.layout.len());
        for (f, v) in self.layout.fields.iter().zip(&self.vals) {
            if v.chars().count() > f.len {
                return proto(format!("{} が {} 桁を超える: {v:?}", f.name, f.len));
            }
            let pad = f.len - v.chars().count();
            let s = match f.k {
                // 空の値はスペースで埋める (N 項目でも、別冊でスペースを入れる場合がある)
                K::N if v.is_empty() => " ".repeat(f.len),
                K::N => {
                    if !v.bytes().all(|b| b.is_ascii_digit()) {
                        return proto(format!("{} が数字でない: {v:?}", f.name));
                    }
                    format!("{}{v}", "0".repeat(pad))
                }
                K::C => format!("{v}{}", " ".repeat(pad)),
            };
            out.extend(encode(code, &s)?);
        }
        Ok(out)
    }

    pub fn decode(layout: &'static Layout, b: &[u8], code: Code) -> Result<Rec> {
        if b.len() != layout.len() {
            return proto(format!("レコード長が不正: {} (期待 {})", b.len(), layout.len()));
        }
        let mut vals = Vec::new();
        let mut off = 0;
        for f in layout.fields {
            let s = decode(code, &b[off..off + f.len])?;
            off += f.len;
            vals.push(match f.k {
                K::N if s.bytes().all(|x| x == b' ') => String::new(),
                K::N => {
                    if !s.bytes().all(|x| x.is_ascii_digit()) {
                        return proto(format!("{} が数字でない: {s:?}", f.name));
                    }
                    s
                }
                K::C => s.trim_end_matches(' ').to_string(),
            });
        }
        Ok(Rec { layout, vals })
    }
}

// ---- 別冊のレイアウト -------------------------------------------------------

/// 総合振込・給与振込のヘッダ (120)。会社コード/振込依頼人コード、会社名/振込依頼人名は同じ位置。
static H120: Layout = Layout {
    fields: &[
        n("data_kind", 1),
        n("type", 2),
        n("code", 1),
        n("client_code", 10),
        c("client_name", 40),
        n("date", 4),
        n("bank_no", 4),
        c("bank_name", 15),
        n("branch_no", 3),
        c("branch_name", 15),
        n("acct_kind", 1),
        n("acct_no", 7),
        c("dummy", 17),
    ],
};
static SOGO_D: Layout = Layout {
    fields: &[
        n("data_kind", 1),
        n("bank_no", 4),
        c("bank_name", 15),
        n("branch_no", 3),
        c("branch_name", 15),
        n("clearing", 4),
        n("acct_kind", 1),
        n("acct_no", 7),
        c("name", 30),
        n("amount", 10),
        n("new_code", 1),
        // 顧客コード1・2。識別表示が Y のときは EDI 情報 (20桁) として使うため、文字として扱う
        c("cust1", 10),
        c("cust2", 10),
        n("transfer_kind", 1),
        c("edi_flag", 1),
        c("dummy", 7),
    ],
};
static KYUYO_D: Layout = Layout {
    fields: &[
        n("data_kind", 1),
        n("bank_no", 4),
        c("bank_name", 15),
        n("branch_no", 3),
        c("branch_name", 15),
        n("clearing", 4),
        n("acct_kind", 1),
        n("acct_no", 7),
        c("name", 30),
        n("amount", 10),
        n("new_code", 1),
        n("emp_no", 10),
        n("dept", 10),
        c("dummy", 9),
    ],
};
static T120: Layout = Layout {
    fields: &[n("data_kind", 1), n("count", 6), n("amount", 12), c("dummy", 101)],
};
static E120: Layout = Layout {
    fields: &[n("data_kind", 1), c("dummy", 119)],
};

static NYUKIN_H: Layout = Layout {
    fields: &[
        n("data_kind", 1),
        n("type", 2),
        n("code", 1),
        n("created", 6),
        n("from", 6),
        n("to", 6),
        n("bank_no", 4),
        c("bank_name", 15),
        n("branch_no", 3),
        c("branch_name", 15),
        n("acct_kind", 1),
        n("acct_no", 7),
        c("acct_name", 40),
        c("dummy", 93),
    ],
};
static NYUKIN_D: Layout = Layout {
    fields: &[
        n("data_kind", 1),
        n("inquiry_no", 6),
        n("acct_date", 6),
        n("start_date", 6),
        n("amount", 10),
        n("other_amount", 10),
        n("client_code", 10),
        c("client_name", 48),
        c("src_bank", 15),
        c("src_branch", 15),
        n("cancel", 1),
        c("edi", 20),
        c("dummy", 52),
    ],
};
static NYUKIN_T: Layout = Layout {
    fields: &[
        n("data_kind", 1),
        n("count", 6),
        n("amount", 12),
        n("cancel_count", 6),
        n("cancel_amount", 12),
        c("dummy", 163),
    ],
};
static E200: Layout = Layout {
    fields: &[n("data_kind", 1), c("dummy", 199)],
};

// ---- 為替通知 (独自設計。docs/spec-net.md §3) --------------------------------------

static EX_H: Layout = Layout {
    fields: &[
        n("data_kind", 1),
        n("type", 2),
        n("code", 1),
        n("from", 4),
        n("to", 4),
        n("date", 8),
        n("time", 6),
        c("dummy", 174),
    ],
};
static EX_T: Layout = Layout {
    fields: &[n("data_kind", 1), n("count", 6), n("amount", 12), c("dummy", 181)],
};
static EX91_D: Layout = Layout {
    fields: &[
        n("data_kind", 1),
        n("handling_no", 10),
        n("src_bank", 4),
        n("src_branch", 3),
        n("dst_bank", 4),
        n("dst_branch", 3),
        n("acct_kind", 1),
        n("acct_no", 7),
        c("name", 30),
        n("amount", 10),
        n("client_code", 10),
        c("client_name", 48),
        c("edi", 20),
        c("dummy", 49),
    ],
};
static EX92_D: Layout = Layout {
    fields: &[
        n("data_kind", 1),
        n("handling_no", 10),
        n("src_bank", 4),
        n("dst_bank", 4),
        n("reason", 2),
        n("amount", 10),
        c("dummy", 169),
    ],
};
static EX93_D: Layout = Layout {
    fields: &[
        n("data_kind", 1),
        n("handling_no", 10),
        n("src_bank", 4),
        n("dst_bank", 4),
        n("req_amount", 10),
        c("dummy", 171),
    ],
};
static EX94_D: Layout = Layout {
    fields: &[
        n("data_kind", 1),
        n("handling_no", 10),
        n("src_bank", 4),
        n("dst_bank", 4),
        n("reason", 2),
        c("dummy", 179),
    ],
};

// ---- ファイル -------------------------------------------------------------------

/// 業務。番号は別冊の「Ⅱ 全銀協制定レコード・フォーマット」の番号。為替通知 (Ex91〜Ex94) は独自設計。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FileKind {
    /// 1 振込入金通知 (0001、照会)
    Nyukin,
    /// 2 入出金取引明細 (0003、照会)
    Meisai,
    /// 3 残高通知 (預金) (0004、照会)
    Zandaka,
    /// 4 給与振込 (民間) (0111、連絡)
    Kyuyo,
    /// 5 賞与振込 (民間) (0112、連絡)
    Shoyo,
    /// 6 給与振込 (地方公務員) (0171、連絡)
    ChihoKyuyo,
    /// 7 賞与振込 (地方公務員) (0172、連絡)
    ChihoShoyo,
    /// 8 総合振込 (0121、連絡)
    Sogo,
    /// 9 株式配当金振込 (0141、連絡)
    Haito,
    /// 10 年金信託契約に係る年金・一時金給付金振込 (0143、連絡)
    NenkinShintaku,
    /// 11 公的年金保険の年金・一時金給付金振込 (0144、連絡)
    KotekiNenkin,
    /// 12 医療保険の給付金振込 (0145、連絡)
    Iryo,
    /// 13 振込口座照会 (依頼明細) (0198、連絡)
    KozaShokai,
    /// 14 振込口座照会 (処理結果明細) (0199、照会)
    KozaShokaiKekka,
    /// 15 預金口座振替 (依頼明細) (0191、連絡)
    Furikae,
    /// 16 預金口座振替 (処理結果明細) (0191、照会)
    FurikaeKekka,
    /// 17 預金口座振替変更通知 (0193、照会)
    FurikaeHenko,
    /// 18 住宅ローン関係 (財形ローン) (0211、照会)
    LoanZaikei,
    /// 19 住宅ローン関係 (その他) (0212、照会)
    LoanSonota,
    /// 20 財形貯蓄関係 (依頼明細) (0300、連絡)
    Zaikei,
    /// 21 財形貯蓄関係 (処理結果明細: 積立内容) (0301、照会)
    ZaikeiTsumitate,
    /// 22 財形貯蓄関係 (処理結果明細: 利子補給) (0302、照会)
    ZaikeiRishi,
    /// 23 財形貯蓄関係 (処理結果明細: その他) (0309、照会)
    ZaikeiSonota,
    /// 24 借入金計算書 (0231、照会)
    KariireKeisan,
    /// 25 借入金残高明細 (0232、照会)
    KariireZandaka,
    /// 26 借入金残高合計 (0233、照会)
    KariireGokei,
    /// 27 外国為替取引明細 (会計性) (0425、照会)
    FxKaikei,
    /// 28 外国為替取引明細 (非会計性) (0426、照会)
    FxHikaikei,
    /// 29 外国為替関連情報 (0427、照会)
    FxKanren,
    /// 30 外国送金依頼 (0435、連絡)
    Soukin,
    /// 31 荷為替輸入信用状発行依頼 (0428、連絡)
    ShinyoJo,
    /// 32 外貨預金入出金取引明細 (0009、照会)
    GaikaMeisai,
    /// 為替通知: 91 振込、92 返却、93 組戻依頼、94 組戻不能 (連絡、独自設計)
    Ex91,
    Ex92,
    Ex93,
    Ex94,
}

/// データ・レコードの様式の1つと、その選び方。条件はすべて満たしたときに、この様式とみなす。
/// 値は数として比べる (前ゼロは無視)。
pub struct Var {
    pub label: &'static str,
    pub layout: &'static Layout,
    /// データ・レコード区分 (`record_kind`) の値。空なら問わない
    pub kinds: &'static [&'static str],
    /// 直前のヘッダーの項目の値
    pub header: Option<(&'static str, &'static [&'static str])>,
    /// 同じグループで先に出たデータ・レコードの項目の値 (例: 本体情報のデータ構成区分)
    pub ctx: Option<(&'static str, &'static [&'static str])>,
}

/// トレーラの項目の求め方。
pub enum Agg {
    /// データ・レコードの件数
    Count,
    /// データ・レコードのこの項目の合計 (項目がない様式は 0)
    Sum(&'static str),
}

/// トレーラの集計項目。`when` の条件 (項目, 等しいか, 値) をすべて満たすデータ・レコードだけを数える。
pub struct Check {
    pub field: &'static str,
    pub agg: Agg,
    pub when: &'static [(&'static str, bool, &'static str)],
}

pub struct Fmt {
    pub kind: FileKind,
    pub title: &'static str,
    pub rec_len: usize,
    /// ヘッダの種別コード
    pub type_code: u64,
    /// ファイル名の産業別コード + データコード (8文字)
    pub name_kind: &'static str,
    /// 照会モードの業務か (でなければ連絡モード)
    pub inquiry: bool,
    pub header: &'static Layout,
    pub datas: &'static [Var],
    pub trailer: &'static Layout,
    pub end: &'static Layout,
    pub checks: &'static [Check],
}

const fn single(layout: &'static Layout) -> [Var; 1] {
    [Var {
        label: "",
        layout,
        kinds: &[],
        header: None,
        ctx: None,
    }]
}

static SOGO_V: [Var; 1] = single(&SOGO_D);
static KYUYO_V: [Var; 1] = single(&KYUYO_D);
static NYUKIN_V: [Var; 1] = single(&NYUKIN_D);
static EX91_V: [Var; 1] = single(&EX91_D);
static EX92_V: [Var; 1] = single(&EX92_D);
static EX93_V: [Var; 1] = single(&EX93_D);
static EX94_V: [Var; 1] = single(&EX94_D);

const COUNT_AMOUNT: &[Check] = &[
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
];

static FMTS: [Fmt; 7] = [
    Fmt {
        kind: FileKind::Sogo,
        title: "総合振込",
        rec_len: 120,
        type_code: 21,
        name_kind: "50200121",
        inquiry: false,
        header: &H120,
        datas: &SOGO_V,
        trailer: &T120,
        end: &E120,
        checks: COUNT_AMOUNT,
    },
    Fmt {
        kind: FileKind::Kyuyo,
        title: "給与振込（民間）",
        rec_len: 120,
        type_code: 11,
        name_kind: "50200111",
        inquiry: false,
        header: &H120,
        datas: &KYUYO_V,
        trailer: &T120,
        end: &E120,
        checks: COUNT_AMOUNT,
    },
    Fmt {
        kind: FileKind::Nyukin,
        title: "振込入金通知",
        rec_len: 200,
        type_code: 1,
        name_kind: "50200001",
        inquiry: true,
        header: &NYUKIN_H,
        datas: &NYUKIN_V,
        trailer: &NYUKIN_T,
        end: &E200,
        // 取消 (取消区分 1) は、合計件数・合計金額に入れず、取消件数・取消金額に入れる
        checks: &[
            Check {
                field: "count",
                agg: Agg::Count,
                when: &[("cancel", false, "1")],
            },
            Check {
                field: "amount",
                agg: Agg::Sum("amount"),
                when: &[("cancel", false, "1")],
            },
            Check {
                field: "cancel_count",
                agg: Agg::Count,
                when: &[("cancel", true, "1")],
            },
            Check {
                field: "cancel_amount",
                agg: Agg::Sum("amount"),
                when: &[("cancel", true, "1")],
            },
        ],
    },
    Fmt {
        kind: FileKind::Ex91,
        title: "為替通知 (振込)",
        rec_len: 200,
        type_code: 91,
        name_kind: "90209091",
        inquiry: false,
        header: &EX_H,
        datas: &EX91_V,
        trailer: &EX_T,
        end: &E200,
        checks: COUNT_AMOUNT,
    },
    Fmt {
        kind: FileKind::Ex92,
        title: "為替通知 (返却)",
        rec_len: 200,
        type_code: 92,
        name_kind: "90209092",
        inquiry: false,
        header: &EX_H,
        datas: &EX92_V,
        trailer: &EX_T,
        end: &E200,
        checks: COUNT_AMOUNT,
    },
    Fmt {
        kind: FileKind::Ex93,
        title: "為替通知 (組戻依頼)",
        rec_len: 200,
        type_code: 93,
        name_kind: "90209093",
        inquiry: false,
        header: &EX_H,
        datas: &EX93_V,
        trailer: &EX_T,
        end: &E200,
        checks: COUNT_AMOUNT,
    },
    Fmt {
        kind: FileKind::Ex94,
        title: "為替通知 (組戻不能)",
        rec_len: 200,
        type_code: 94,
        name_kind: "90209094",
        inquiry: false,
        header: &EX_H,
        datas: &EX94_V,
        trailer: &EX_T,
        end: &E200,
        checks: COUNT_AMOUNT,
    },
];

/// すべての業務のフォーマット。
pub fn all() -> impl Iterator<Item = &'static Fmt> {
    FMTS.iter().chain(crate::formats::GENERATED.iter())
}

pub fn fmt(kind: FileKind) -> &'static Fmt {
    all().find(|f| f.kind == kind).unwrap()
}

/// ファイル名の識別部 (8文字) から業務を引く。同じデータコードで連絡と照会がある業務 (預金口座振替 0191) は、連絡の方を返す。
pub fn kind_from_name(name_kind: &str) -> Option<&'static Fmt> {
    kind_from_name_mode(name_kind, false).or_else(|| kind_from_name_mode(name_kind, true))
}

/// ファイル名の識別部とモード (照会か) から業務を引く。
pub fn kind_from_name_mode(name_kind: &str, inquiry: bool) -> Option<&'static Fmt> {
    all().find(|f| f.name_kind == name_kind && f.inquiry == inquiry)
}

/// 数字なら数として、でなければ文字列として比べる (前ゼロの違いを無視するため)。
fn same(a: &str, b: &str) -> bool {
    match (a.parse::<u64>(), b.parse::<u64>()) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
}

fn matches(r: &Rec, when: &[(&str, bool, &str)]) -> bool {
    when.iter().all(|(f, eq, v)| (r.has(f) && same(r.get(f), v)) == *eq)
}

/// トレーラの集計項目の値を、データ・レコードから求める。
fn aggregate(c: &Check, data: &[Rec]) -> u64 {
    let rows = data.iter().filter(|d| matches(d, c.when));
    match c.agg {
        Agg::Count => rows.count() as u64,
        Agg::Sum(f) => rows.filter(|d| d.has(f)).map(|d| d.num(f)).sum(),
    }
}

/// 人名・店舗名の欄に使える文字か (別冊 付録1 の注1): カナ(ヲと小文字を除く)、濁点、半濁点、英大文字、数字、スペース、`( ) - .`。
pub fn name_chars_ok(s: &str) -> bool {
    s.chars().all(|c| {
        matches!(c, 'A'..='Z' | '0'..='9' | ' ' | '(' | ')' | '-' | '.' | '\u{ff71}'..='\u{ff9f}')
            && !('\u{ff67}'..='\u{ff6f}').contains(&c)
    })
}

/// ヘッダー・データ・トレーラの1組。1ファイルに複数の組があってよい (口座ごとの入出金取引明細など)。
#[derive(Clone, Debug)]
pub struct Group {
    pub header: Rec,
    pub data: Vec<Rec>,
    pub trailer: Rec,
}

pub struct Parsed {
    pub code: Code,
    /// 最初の組のヘッダー
    pub header: Rec,
    /// すべての組のデータ・レコード
    pub data: Vec<Rec>,
    /// 最初の組のトレーラ
    pub trailer: Rec,
    pub groups: Vec<Group>,
    pub end: Rec,
}

impl Fmt {
    pub fn new_header(&self) -> Rec {
        Rec::new(self.header)
    }
    /// 最初の様式のデータ・レコード。
    pub fn new_data(&self) -> Rec {
        Rec::new(self.datas[0].layout)
    }
    /// ラベルの様式のデータ・レコード。
    pub fn new_data_of(&self, label: &str) -> Option<Rec> {
        self.datas.iter().find(|v| v.label == label).map(|v| Rec::new(v.layout))
    }

    /// ヘッダとデータからファイルを作る (1組)。
    pub fn build(&self, code: Code, header: Rec, data: &[Rec]) -> Result<Vec<u8>> {
        self.build_groups(code, &[(header, data.to_vec())])
    }

    /// 複数の組からファイルを作る。データ区分、種別、コード区分、トレーラの集計項目、エンドは自動で設定する。
    pub fn build_groups(&self, code: Code, groups: &[(Rec, Vec<Rec>)]) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        for (header, data) in groups {
            if !std::ptr::eq(header.layout, self.header) {
                return proto("ヘッダー・レコードの種類が違う");
            }
            out.extend(
                header
                    .clone()
                    .set_num("data_kind", 1)
                    .set_num("type", self.type_code)
                    .set_num("code", code.digit() as u64)
                    .encode(code)?,
            );
            for d in data {
                if !self.datas.iter().any(|v| std::ptr::eq(d.layout, v.layout)) {
                    return proto("データレコードの種類が違う");
                }
                out.extend(d.clone().set_num("data_kind", 2).encode(code)?);
            }
            let mut t = Rec::new(self.trailer).set_num("data_kind", 8);
            for c in self.checks {
                t = t.set_num(c.field, aggregate(c, data));
            }
            out.extend(t.encode(code)?);
        }
        let mut e = Rec::new(self.end).set_num("data_kind", 9);
        // エンドに件数がある様式: レコード総件数 (エンド自身を含む) と、組 (口座) の数
        let total = (out.len() / self.rec_len + 1) as u64;
        for f in ["total_records", "total_record_count", "total_count"] {
            if e.has(f) {
                e = e.set_num(f, total);
            }
        }
        for f in ["acct_count", "account_count"] {
            if e.has(f) {
                e = e.set_num(f, groups.len() as u64);
            }
        }
        out.extend(e.encode(code)?);
        Ok(out)
    }

    /// データ・レコードの様式を選んで読む。
    fn decode_data(&self, raw: &[u8], code: Code, header: &Rec, prev: &[Rec]) -> Result<Rec> {
        let mut last_err = None;
        for v in self.datas {
            let r = match Rec::decode(v.layout, raw, code) {
                Ok(r) => r,
                Err(e) => {
                    last_err = Some(e);
                    continue;
                }
            };
            let kind_ok = v.kinds.is_empty() || (r.has("record_kind") && v.kinds.iter().any(|k| same(r.get("record_kind"), k)));
            let header_ok = v
                .header
                .is_none_or(|(f, vals)| header.has(f) && vals.iter().any(|x| same(header.get(f), x)));
            let ctx_ok = v.ctx.is_none_or(|(f, vals)| {
                prev.iter()
                    .rev()
                    .find(|p| p.has(f))
                    .is_some_and(|p| vals.iter().any(|x| same(p.get(f), x)))
            });
            if kind_ok && header_ok && ctx_ok {
                return Ok(r);
            }
        }
        Err(last_err.unwrap_or_else(|| ZError::Protocol("データ・レコードの様式が決まらない".into())))
    }

    /// ファイルを解析し、順序、種別、トレーラの集計項目を検査する。
    pub fn parse(&self, b: &[u8]) -> Result<Parsed> {
        if b.is_empty() || b.len() % self.rec_len != 0 || b.len() / self.rec_len < 3 {
            return proto(format!("ファイル長が不正: {} バイト (レコード長 {})", b.len(), self.rec_len));
        }
        let code = match b[0] {
            0x31 => Code::Jis,
            0xF1 => Code::Ebcdic,
            x => return proto(format!("先頭のデータ区分が不正: 0x{x:02X}")),
        };
        let recs: Vec<&[u8]> = b.chunks(self.rec_len).collect();
        let kind_of = |r: &[u8]| decode(code, &r[..1]).unwrap_or_default();
        let mut groups = Vec::new();
        let mut i = 0;
        while i < recs.len() && kind_of(recs[i]) == "1" {
            let header = Rec::decode(self.header, recs[i], code)?;
            if header.num("type") != self.type_code || header.num("code") != code.digit() as u64 {
                return proto("ヘッダの種別/コード区分が不正");
            }
            i += 1;
            let mut data = Vec::new();
            while i < recs.len() && kind_of(recs[i]) == "2" {
                data.push(self.decode_data(recs[i], code, &header, &data)?);
                i += 1;
            }
            if i >= recs.len() || kind_of(recs[i]) != "8" {
                return proto("トレーラ・レコードがない");
            }
            let trailer = Rec::decode(self.trailer, recs[i], code)?;
            i += 1;
            for c in self.checks {
                let want = aggregate(c, &data);
                if trailer.num(c.field) != want {
                    return proto(format!("トレーラの {} が不一致: {} (実際 {want})", c.field, trailer.num(c.field)));
                }
            }
            groups.push(Group { header, data, trailer });
        }
        if groups.is_empty() {
            return proto("ヘッダー・レコードがない");
        }
        if i != recs.len() - 1 || kind_of(recs[i]) != "9" {
            return proto("エンド・レコードがない、またはエンドのあとにレコードがある");
        }
        let end = Rec::decode(self.end, recs[i], code)?;
        Ok(Parsed {
            code,
            header: groups[0].header.clone(),
            data: groups.iter().flat_map(|g| g.data.clone()).collect(),
            trailer: groups[0].trailer.clone(),
            groups,
            end,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layouts_have_declared_lengths() {
        for f in &FMTS {
            for l in [f.header, f.trailer, f.end].into_iter().chain(f.datas.iter().map(|v| v.layout)) {
                assert_eq!(l.len(), f.rec_len, "{:?}", f.kind);
            }
        }
    }

    fn sogo(code: Code) -> Vec<u8> {
        let f = fmt(FileKind::Sogo);
        let h = f
            .new_header()
            .set("client_code", "1234567890")
            .set("client_name", "ｶ)ﾃｽﾄ")
            .set("date", "1121")
            .set_num("bank_no", 1);
        let d = |amt, no| {
            f.new_data()
                .set_num("bank_no", 2)
                .set_num("branch_no", 3)
                .set("acct_kind", "1")
                .set("acct_no", format!("{no:07}"))
                .set("name", "ﾔﾏﾀﾞ ﾀﾛｳ")
                .set_num("amount", amt)
        };
        f.build(code, h, &[d(1000, 1), d(250, 2)]).unwrap()
    }

    #[test]
    fn sogo_roundtrip_both_codes() {
        for code in [Code::Jis, Code::Ebcdic] {
            let b = sogo(code);
            assert_eq!(b.len(), 120 * 5);
            let p = fmt(FileKind::Sogo).parse(&b).unwrap();
            assert_eq!(p.code, code);
            assert_eq!(p.header.get("client_name"), "ｶ)ﾃｽﾄ");
            assert_eq!(p.data.len(), 2);
            assert_eq!(p.data[1].num("amount"), 250);
            assert_eq!(p.trailer.num("count"), 2);
            assert_eq!(p.trailer.num("amount"), 1250);
        }
    }

    #[test]
    fn known_bytes_jis() {
        let b = sogo(Code::Jis);
        // ヘッダ: データ区分 '1'、種別 '21'、コード区分 '0'
        assert_eq!(&b[..4], b"1210");
        // トレーラ (4番目のレコード): '8' + 件数6桁 + 金額12桁
        assert_eq!(&b[120 * 3..120 * 3 + 19], b"8000002000000001250");
        // エンド: '9' + スペース
        assert_eq!(b[120 * 4], b'9');
        assert!(b[120 * 4 + 1..].iter().all(|&x| x == 0x20));
    }

    #[test]
    fn rejects_trailer_mismatch_and_bad_order() {
        let mut b = sogo(Code::Jis);
        b[120 * 3 + 18] = b'9'; // 合計金額を変える
        assert!(fmt(FileKind::Sogo).parse(&b).is_err());
        let mut b = sogo(Code::Jis);
        b[120 * 3 + 6] = b'3'; // 合計件数を変える
        assert!(fmt(FileKind::Sogo).parse(&b).is_err());
        let b = sogo(Code::Jis);
        assert!(fmt(FileKind::Sogo).parse(&b[..120 * 4]).is_err()); // エンドなし
        assert!(fmt(FileKind::Kyuyo).parse(&sogo(Code::Jis)).is_err()); // 種別違い
    }

    #[test]
    fn rejects_bad_fields() {
        let f = fmt(FileKind::Sogo);
        let h = f.new_header().set("client_name", "a"); // 小文字は不可
        assert!(f.build(Code::Jis, h, &[]).is_err());
        let h = f.new_header().set("client_code", "12345678901"); // 桁あふれ
        assert!(f.build(Code::Jis, h, &[]).is_err());
        let h = f.new_header().set("date", "12x4");
        assert!(f.build(Code::Jis, h, &[]).is_err());
    }

    #[test]
    fn nyukin_and_exchange_roundtrip() {
        let f = fmt(FileKind::Nyukin);
        let h = f.new_header().set_num("bank_no", 2).set("acct_name", "ﾃｽﾄ");
        let d = f.new_data().set_num("amount", 777).set("client_name", "ﾌﾘｺﾐ");
        let b = f.build(Code::Ebcdic, h, &[d]).unwrap();
        assert_eq!(b.len(), 800);
        let p = f.parse(&b).unwrap();
        assert_eq!(p.data[0].num("amount"), 777);

        let f = fmt(FileKind::Ex94);
        let d = f.new_data().set_num("handling_no", 1121000001).set_num("reason", 1);
        let b = f.build(Code::Jis, f.new_header().set_num("from", 2), &[d]).unwrap();
        let p = f.parse(&b).unwrap();
        assert_eq!(p.data[0].num("handling_no"), 1121000001);
        assert_eq!(p.trailer.num("amount"), 0);
    }

    /// すべての業務で、すべての様式のデータ・レコードを作って、読み戻せること (様式の選び方とトレーラの集計を含む)。
    #[test]
    fn every_format_and_variant_roundtrips() {
        for f in all() {
            let mut groups = Vec::new();
            for v in f.datas {
                let mut header = f.new_header();
                if let Some((field, vals)) = v.header {
                    header = header.set(field, vals[0]);
                }
                let mut data = Vec::new();
                if let Some((field, vals)) = v.ctx {
                    // 先行レコード (例: 本体情報(1) のデータ構成区分) を先に置く
                    let lead = f.datas.iter().find(|w| Rec::new(w.layout).has(field)).unwrap();
                    data.push(Rec::new(lead.layout).set("record_kind", lead.kinds[0]).set(field, vals[0]));
                }
                let mut r = Rec::new(v.layout);
                if !v.kinds.is_empty() {
                    r = r.set("record_kind", v.kinds[0]);
                }
                if r.has("amount") {
                    r = r.set_num("amount", 1234);
                }
                data.push(r.clone());
                data.push(r);
                groups.push((header, data));
            }
            for code in [Code::Jis, Code::Ebcdic] {
                let b = f.build_groups(code, &groups).unwrap_or_else(|e| panic!("{:?}: {e}", f.kind));
                let p = f.parse(&b).unwrap_or_else(|e| panic!("{:?} {code:?}: {e}", f.kind));
                assert_eq!(p.groups.len(), groups.len(), "{:?}", f.kind);
                for (g, (_, want)) in p.groups.iter().zip(&groups) {
                    let got: Vec<&str> = g.data.iter().map(|d| d.layout_label(f)).collect();
                    let exp: Vec<&str> = want.iter().map(|d| d.layout_label(f)).collect();
                    assert_eq!(got, exp, "{:?}: 様式の選び方", f.kind);
                }
            }
        }
    }

    #[test]
    fn kind_lookup() {
        assert_eq!(kind_from_name("50200121").unwrap().kind, FileKind::Sogo);
        assert_eq!(kind_from_name("90209092").unwrap().kind, FileKind::Ex92);
        assert!(kind_from_name("12345678").is_none());
    }
}
