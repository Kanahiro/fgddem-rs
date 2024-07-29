# fgddem-rs

基盤地図情報の数値標高モデルXMLをパースしてGeoTIFFに変換するツールです。

## 使い方

### ビルド

```sh
# git, cargoがインストール済みであること
# GDALがインストールされている必要もあるかもしれない
git clone git@github.com:Kanahiro/fgddem-rs.git
cd fgddem-rs
cargo build --release
```

### 実行

```sh
./target/release/fgddem FG-GML-5238-74-00-DEM5A-20161001.xml -o output_dir
# ./output_dir/FG-GML-5238-74-00-DEM5A-20161001.tif が生成される

# glob形式の入力が可能
./target/release/fgddem input_dir/*.xml -o output_dir
# ./output_dir/*.tif として出力される
```
