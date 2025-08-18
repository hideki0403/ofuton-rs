# ofuton-rs
TODO  

## ofuton v1からのマイグレーション方法
TODO

## ファイル情報のimportに使用するtsvファイルの出力方法
```sh
psql misskey -t -c 'SELECT name, type, url FROM drive_file WHERE "userHost" IS NULL' -A -F $'\t' > drive_file.tsv
```
