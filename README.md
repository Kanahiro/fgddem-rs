# fgddem-rs

基盤地図情報の数値標高モデル (DEM) GML ファイルを GeoTIFF に変換する CLI。Pure Rust 実装で GDAL 不要。

ビルド済みバイナリは [Releases](https://github.com/Kanahiro/fgddem-rs/releases) から取得できます。

## 使い方

```sh
# 1ファイルにつき1つのGeoTIFFを出力
fgddem input_dir/*.xml -o out

# 全入力を1ファイルにマージ（merged.tif）
fgddem input_dir/*.xml -o out --merge
# → out/merged.tif
```

### オプション

| オプション | 説明 |
|---|---|
| `-o <DIR>` | 出力ディレクトリ |
| `-m`, `--merge` | 全入力を1つのGeoTIFFにマージ |
| `-c`, `--compression <KIND>` | `none` / `deflate` (default) / `lzw` / `zstd` |

## 出力仕様

- GeoTIFF, single band, Float64
- CRS: EPSG:6668 (JGD2011 lat-lon)
- Nodata: `-9999`
- マージ時は 256×256 タイル形式、個別出力時はストリップ形式
