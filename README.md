# ofuton-rs
TODO  

### ofuton v1からのマイグレーション方法
TODO

### ファイル情報のimportに使用するtsvファイルの出力方法
```sh
psql misskey -t -c 'SELECT name, type, url FROM drive_file WHERE "userHost" IS NULL' -A -F $'\t' > drive_file.tsv
```

### テストが落ちる場合
大抵の場合はコードスタイルの問題で落ちているため、以下のコマンドを実行して修正すると通る可能性が高い  
ツールチェイン等が最新版でない場合は `rustup update` しておく  

```sh
cargo clippy --no-deps --all-features --fix # 場合によっては --allow-dirty
cargo +nightly fmt --all
```