#!/bin/sh
# 全銀協の公式資料を ref/ に取得する (私的かつ非商業目的の利用に限る。再配布しないこと)。
# 実装を読むときの一次資料。リポジトリには含めない (.gitignore 済み)。
set -eu
base=https://www.zenginkyo.or.jp/fileadmin/res/abstract/efforts/system
mkdir -p "$(dirname "$0")/../ref"
cd "$(dirname "$0")/../ref"
for f in jba_protocol.pdf jba_protocol_pc.pdf isdn_leaf.pdf; do
  curl -fsSL -o "$f" "$base/$f"
  echo "取得: ref/$f"
done
