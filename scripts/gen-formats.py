#!/usr/bin/env python3
"""formats/*.json (別冊のレコード・フォーマットを書き写したもの) から src/formats.rs を作る。

    python scripts/gen-formats.py

JSON は、別冊 (全銀協パーソナル・コンピュータ用標準通信プロトコル 適用業務およびレコード・フォーマット 13 版) の
表を、項目名・属性・桁数・要点の形で書き写したもの。各レコードの桁数の合計がレコード長になることを、ここで確かめる。
振込入金通知 (1)・給与振込 (4)・総合振込 (8) は src/record.rs に手で書いてあるので、ここでは扱わない。

データ・レコードの様式の選び方 (SEL) とトレーラの集計 (CHECKS) は、JSON の説明 (notes、variant_rule) を読んで決めたもので、
下の表に書く。表にないトレーラは、項目名が count なら件数、amount (データにも amount がある) なら金額の合計とする。
"""

import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
SRC = ROOT / "formats"
OUT = ROOT / "src" / "formats.rs"

# 別冊の番号 → FileKind の名前
KINDS = {
    2: "Meisai", 3: "Zandaka", 5: "Shoyo", 6: "ChihoKyuyo", 7: "ChihoShoyo", 9: "Haito",
    10: "NenkinShintaku", 11: "KotekiNenkin", 12: "Iryo", 13: "KozaShokai", 14: "KozaShokaiKekka",
    15: "Furikae", 16: "FurikaeKekka", 17: "FurikaeHenko", 18: "LoanZaikei", 19: "LoanSonota",
    20: "Zaikei", 21: "ZaikeiTsumitate", 22: "ZaikeiRishi", 23: "ZaikeiSonota", 24: "KariireKeisan",
    25: "KariireZandaka", 26: "KariireGokei", 27: "FxKaikei", 28: "FxHikaikei", 29: "FxKanren",
    30: "Soukin", 31: "ShinyoJo", 32: "GaikaMeisai",
}

# データ・レコードの様式の選び方。(別冊の番号, 様式のラベル) → (データ・レコード区分の値, ヘッダーの条件, 先行レコードの条件)
# 条件は (項目名, [値...])。値は数として比べる (前ゼロを無視)。
ACCT_FLOW = ["1", "2", "4"]
ACCT_STOCK = ["5", "6", "7"]
SEL = {
    (2, "普通・当座・貯蓄"): ([], ("acct_kind", ACCT_FLOW), None),
    (2, "通知・定期・積立定期"): ([], ("acct_kind", ACCT_STOCK), None),
    (24, "loan"): (["10", "20"], None, None),
    (24, "discount_detail"): (["30"], None, None),
    (24, "discount_total"): (["31"], None, None),
    (25, "loan"): (["10", "20"], None, None),
    (25, "discount"): (["30"], None, None),
    (29, "外国為替公示相場"): (["01"], None, None),
    (30, "本体情報"): (["01"], None, None),
    (30, "会計処理内訳"): (["02"], None, None),
    (30, "送金指図内訳"): (["03"], None, None),
    (31, "本体情報(1)"): (["11"], None, None),
    (31, "本体情報(2)"): (["12"], None, None),
    (31, "本体情報(3)"): (["13"], None, None),
    (31, "取引情報(1) 発行依頼"): (["21"], None, ("composition_kind", ["01"])),
    (31, "取引情報(2) 発行依頼"): (["22"], None, ("composition_kind", ["01"])),
    (31, "取引情報(3) 発行依頼"): (["23"], None, None),
    (31, "取引情報(4) 発行依頼"): (["24"], None, None),
    (31, "取引情報(5) 発行依頼"): (["25"], None, None),
    (31, "取引情報(6) 発行依頼"): (["26"], None, None),
    (31, "取引情報(7) 発行依頼"): (["27"], None, None),
    (31, "取引情報(1) 条件変更依頼"): (["21"], None, ("composition_kind", ["02"])),
    (31, "取引情報(2) 条件変更依頼"): (["22"], None, ("composition_kind", ["02"])),
    (31, "内訳情報"): (["30"], None, None),
    (32, "本体情報(1) 普通・当座・貯蓄"): (["01"], ("deposit_type", ACCT_FLOW), None),
    (32, "本体情報(2) 通知・定期・積立定期"): (["01"], ("deposit_type", ACCT_STOCK), None),
    (32, "内訳情報"): (["20"], None, None),
}


def count(field, *when):
    return (field, "Count", list(when))


def total(field, of, *when):
    return (field, f'Sum("{of}")', list(when))


def eq(f, v):
    return (f, True, v)


def ne(f, v):
    return (f, False, v)


TSUMITATE = [total(f"{k}_total", f"{k}_amount") for k in ("salary", "bonus", "other1", "other2")]
ZAIKEI_RESULT = [count("count"), total("balance_total", "balance"), *TSUMITATE,
                 total("subsidy_base_balance_total", "subsidy_base_balance"),
                 total("subsidy_amount_total", "subsidy_amount"),
                 total("subsidy_cumulative_total", "subsidy_cumulative")]
CHECKS = {
    2: [count("count"), count("in_count", eq("io_kind", "1")), total("in_amount", "amount", eq("io_kind", "1")),
        count("out_count", eq("io_kind", "2")), total("out_amount", "amount", eq("io_kind", "2"))],
    15: [count("count"), total("amount", "amount")],
    16: [count("count"), total("amount", "amount"),
         count("ok_count", eq("result_code", "0")), total("ok_amount", "amount", eq("result_code", "0")),
         count("ng_count", ne("result_code", "0")), total("ng_amount", "amount", ne("result_code", "0"))],
    18: [count("count")],
    19: [count("count")],
    20: [count("count"), *TSUMITATE],
    21: ZAIKEI_RESULT,
    22: ZAIKEI_RESULT,
    23: ZAIKEI_RESULT,
    27: [count("record_count")],
    28: [count("record_count")],
    30: [count("data_record_count"), count("body_count", eq("record_kind", "01"))],
    31: [count("data_record_count"), count("body_count", eq("record_kind", "11")),
         count("issue_count", eq("record_kind", "11"), eq("composition_kind", "01")),
         count("amend_count", eq("record_kind", "11"), eq("composition_kind", "02"))],
    32: [count("data_record_count"),
         count("deposit_count", eq("record_kind", "01"), eq("in_out", "1")),
         total("deposit_amount", "amount", eq("record_kind", "01"), eq("in_out", "1")),
         count("withdrawal_count", eq("record_kind", "01"), eq("in_out", "2")),
         total("withdrawal_amount", "amount", eq("record_kind", "01"), eq("in_out", "2"))],
}

# 項目の属性を書き換える (別冊の表記と、実際の使われ方が違うもの)
KIND_FIX = {
    # 識別表示が Y のとき、顧客コード1・2 は EDI 情報 (文字 20 桁) になる。総合振込 (record.rs) と同じ扱い
    (13, "cust1"): "C", (13, "cust2"): "C", (14, "cust1"): "C", (14, "cust2"): "C",
}


def rs_str(s):
    return '"' + s.replace("\\", "\\\\").replace('"', '\\"') + '"'


def main():
    files = sorted(SRC.glob("*.json"))
    if not files:
        sys.exit(f"{SRC} に JSON がない")
    out = [
        "//! 別冊の業務のレコード・フォーマット (振込入金通知・給与振込・総合振込を除く)。",
        "//! scripts/gen-formats.py が formats/*.json から生成する。手で直さないこと。",
        "",
        "use crate::record::{Agg, Check, FileKind, Fmt, Layout, Var, c, n};",
        "",
    ]
    fmts = []
    for path in files:
        d = json.loads(path.read_text(encoding="utf-8"))
        no = d["no"]
        if no not in KINDS:
            sys.exit(f"{path.name}: 番号 {no} の FileKind がない")
        rec_len = d["rec_len"]
        names = {}
        datas = []
        for r in d["records"]:
            fields = r["fields"]
            total_len = sum(f["len"] for f in fields)
            if total_len != rec_len:
                sys.exit(f"{path.name}: {r['kind']} {r.get('variant')} の桁数の合計 {total_len} != {rec_len}")
            seen = set()
            for f in fields:
                if f["name"] in seen:
                    sys.exit(f"{path.name}: {r['kind']} {r.get('variant')} に項目名 {f['name']} が重複")
                seen.add(f["name"])
            kind = r["kind"]
            label = r.get("variant") or ""
            ident = f"L{no:02}_{kind.upper()}" + (f"_{len(datas)}" if kind == "data" else "")
            out.append(f"/// {d['title']} {kind} {label}".rstrip())
            out.append(f"static {ident}: Layout = Layout {{")
            out.append("    fields: &[")
            for f in fields:
                k = KIND_FIX.get((no, f["name"]), f["k"])
                out.append(f'        {k.lower()}({rs_str(f["name"])}, {f["len"]}), // {f["ja"]}')
            out.append("    ],")
            out.append("};")
            if kind == "data":
                datas.append((label, ident))
            else:
                names[kind] = ident
        vars_ = []
        for label, ident in datas:
            if len(datas) == 1 and (no, label) not in SEL:
                vars_.append(f"Var {{ label: {rs_str(label)}, layout: &{ident}, kinds: &[], header: None, ctx: None }}")
                continue
            if (no, label) not in SEL and no in (27, 28):
                # 外国為替取引明細: 様式名の先頭がデータ・レコード区分 (例: "11 取引情報［輸出手形買取］")
                SEL[(no, label)] = ([label.split()[0]], None, None)
            if (no, label) not in SEL:
                sys.exit(f"{path.name}: 様式 {label!r} の選び方 (SEL) がない")
            kinds, hdr, ctx = SEL[(no, label)]
            cond = lambda c: "None" if c is None else f"Some(({rs_str(c[0])}, &[{', '.join(rs_str(v) for v in c[1])}]))"
            vars_.append(
                f"Var {{ label: {rs_str(label)}, layout: &{ident}, kinds: &[{', '.join(rs_str(v) for v in kinds)}], "
                f"header: {cond(hdr)}, ctx: {cond(ctx)} }}"
            )
        data_fields = {f["name"] for r in d["records"] if r["kind"] == "data" for f in r["fields"]}
        trailer_fields = {f["name"] for r in d["records"] if r["kind"] == "trailer" for f in r["fields"]}
        checks = CHECKS.get(no)
        if checks is None:
            checks = []
            if "count" in trailer_fields:
                checks.append(count("count"))
            if "amount" in trailer_fields and "amount" in data_fields:
                checks.append(total("amount", "amount"))
        for field, _, _ in checks:
            if field not in trailer_fields:
                sys.exit(f"{path.name}: トレーラに {field} がない")
        check_rs = []
        for field, agg, when in checks:
            w = ", ".join(f"({rs_str(f)}, {'true' if e else 'false'}, {rs_str(v)})" for f, e, v in when)
            check_rs.append(f"Check {{ field: {rs_str(field)}, agg: Agg::{agg}, when: &[{w}] }}")
        fmts.append(
            "\n".join(
                [
                    "    Fmt {",
                    f"        kind: FileKind::{KINDS[no]},",
                    f"        title: {rs_str(d['title'])},",
                    f"        rec_len: {rec_len},",
                    f"        type_code: {d['type_code']},",
                    f"        name_kind: \"5020{d['data_code']}\",",
                    f"        inquiry: {'true' if d['mode'] == '照会' else 'false'},",
                    f"        header: &{names['header']},",
                    "        datas: &[",
                    *[f"            {v}," for v in vars_],
                    "        ],",
                    f"        trailer: &{names['trailer']},",
                    f"        end: &{names['end']},",
                    "        checks: &[",
                    *[f"            {c}," for c in check_rs],
                    "        ],",
                    "    },",
                ]
            )
        )
    out.append("")
    out.append(f"pub static GENERATED: [Fmt; {len(fmts)}] = [")
    out.extend(fmts)
    out.append("];")
    OUT.write_text("\n".join(out) + "\n", encoding="utf-8", newline="\n")
    print(f"{OUT.relative_to(ROOT)}: {len(fmts)} 種類")


if __name__ == "__main__":
    main()
