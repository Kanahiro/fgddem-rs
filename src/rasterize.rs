use std::collections::HashMap;
use std::str::FromStr;

use anyhow::{anyhow, Result};
use geotiff_writer::{Compression, GeoTiffBuilder};
use ndarray::Array2;
use rayon::prelude::*;

use crate::dataset::{parse_metadata, Dataset};

const NODATA: f32 = -9999.0;
const EPSG: u16 = 6668;
const TIFF_TILE_W: usize = 256;
const TIFF_TILE_H: usize = 256;
const PARSE_CHUNK: usize = 16;

#[derive(Debug, Clone, Copy)]
pub enum CompressionKind {
    None,
    Deflate,
    Lzw,
    Zstd,
}

impl CompressionKind {
    fn to_geotiff(self) -> Compression {
        match self {
            CompressionKind::None => Compression::None,
            CompressionKind::Deflate => Compression::Deflate,
            CompressionKind::Lzw => Compression::Lzw,
            CompressionKind::Zstd => Compression::Zstd,
        }
    }
}

pub fn write(
    grid_shape: (usize, usize),
    extent: ((f64, f64), (f64, f64)),
    values: &Vec<f32>,
    output: &str,
    compression: CompressionKind,
) -> Result<()> {
    let (width, height) = grid_shape;

    let x_min = extent.0 .1;
    let y_max = extent.1 .0;
    let x_res = (extent.1 .1 - extent.0 .1) / width as f64;
    let y_res = (extent.1 .0 - extent.0 .0) / height as f64;

    let data = Array2::from_shape_vec((height, width), values.clone())?;

    GeoTiffBuilder::new(width as u32, height as u32)
        .epsg(EPSG)
        .pixel_scale(x_res, y_res)
        .origin(x_min, y_max)
        .nodata(&NODATA.to_string())
        .compression(compression.to_geotiff())
        .write_2d(output, data.view())?;

    Ok(())
}

struct TileMeta {
    path: String,
    shape: (usize, usize),
    extent: ((f64, f64), (f64, f64)),
    col_off: usize,
    row_off: usize,
}

pub fn write_merged_streaming(
    input_files: &[String],
    output: &str,
    compression: CompressionKind,
) -> Result<()> {
    if input_files.is_empty() {
        return Err(anyhow!("no inputs to merge"));
    }

    let mut metas: Vec<TileMeta> = input_files
        .par_iter()
        .map(|path| -> Result<TileMeta> {
            let meta = parse_metadata(std::path::Path::new(path))?;
            Ok(TileMeta {
                path: path.clone(),
                shape: meta.shape,
                extent: meta.extent,
                col_off: 0,
                row_off: 0,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let (fgd_w, fgd_h) = metas[0].shape;
    let ((lat0_min, lon0_min), (lat0_max, lon0_max)) = metas[0].extent;
    let x_res = (lon0_max - lon0_min) / fgd_w as f64;
    let y_res = (lat0_max - lat0_min) / fgd_h as f64;

    for m in &metas {
        if m.shape != (fgd_w, fgd_h) {
            return Err(anyhow!(
                "tile shape mismatch in {}: expected {:?}, got {:?}",
                m.path,
                (fgd_w, fgd_h),
                m.shape
            ));
        }
    }

    let mut lon_min = f64::INFINITY;
    let mut lat_min = f64::INFINITY;
    let mut lon_max = f64::NEG_INFINITY;
    let mut lat_max = f64::NEG_INFINITY;
    for m in &metas {
        let ((t_lat_min, t_lon_min), (t_lat_max, t_lon_max)) = m.extent;
        lon_min = lon_min.min(t_lon_min);
        lon_max = lon_max.max(t_lon_max);
        lat_min = lat_min.min(t_lat_min);
        lat_max = lat_max.max(t_lat_max);
    }

    let width = ((lon_max - lon_min) / x_res).round() as usize;
    let height = ((lat_max - lat_min) / y_res).round() as usize;

    for m in &mut metas {
        let ((_, t_lon_min), (t_lat_max, _)) = m.extent;
        m.col_off = ((t_lon_min - lon_min) / x_res).round() as usize;
        m.row_off = ((lat_max - t_lat_max) / y_res).round() as usize;
    }

    let mut remaining: HashMap<(usize, usize), usize> = HashMap::new();
    for m in &metas {
        let (tc_lo, tc_hi, tr_lo, tr_hi) = tiff_tile_range(m.col_off, m.row_off, fgd_w, fgd_h);
        for tr in tr_lo..=tr_hi {
            for tc in tc_lo..=tc_hi {
                *remaining.entry((tc, tr)).or_insert(0) += 1;
            }
        }
    }

    metas.sort_by_key(|m| (m.row_off, m.col_off));

    let mut writer = GeoTiffBuilder::new(width as u32, height as u32)
        .epsg(EPSG)
        .pixel_scale(x_res, y_res)
        .origin(lon_min, lat_max)
        .nodata(&NODATA.to_string())
        .compression(compression.to_geotiff())
        .tile_size(TIFF_TILE_W as u32, TIFF_TILE_H as u32)
        .tile_writer_file::<f32, _>(output)?;

    let mut active: HashMap<(usize, usize), Vec<f32>> = HashMap::new();

    for chunk in metas.chunks(PARSE_CHUNK) {
        let parsed: Vec<(usize, usize, Vec<f32>)> = chunk
            .par_iter()
            .map(|m| -> Result<(usize, usize, Vec<f32>)> {
                let content = std::fs::read_to_string(&m.path)?;
                let dataset = Dataset::from_str(&content)?;
                Ok((m.col_off, m.row_off, dataset.get_grid_values().clone()))
            })
            .collect::<Result<Vec<_>>>()?;

        for (col_off, row_off, values) in parsed {
            let (tc_lo, tc_hi, tr_lo, tr_hi) = tiff_tile_range(col_off, row_off, fgd_w, fgd_h);

            for tr in tr_lo..=tr_hi {
                for tc in tc_lo..=tc_hi {
                    let buf = active
                        .entry((tc, tr))
                        .or_insert_with(|| vec![NODATA; TIFF_TILE_W * TIFF_TILE_H]);

                    let tiff_x = tc * TIFF_TILE_W;
                    let tiff_y = tr * TIFF_TILE_H;
                    let inter_x_lo = col_off.max(tiff_x);
                    let inter_x_hi = (col_off + fgd_w).min(tiff_x + TIFF_TILE_W);
                    let inter_y_lo = row_off.max(tiff_y);
                    let inter_y_hi = (row_off + fgd_h).min(tiff_y + TIFF_TILE_H);

                    for y in inter_y_lo..inter_y_hi {
                        let fgd_row = y - row_off;
                        let tiff_row = y - tiff_y;
                        let fgd_x = inter_x_lo - col_off;
                        let tiff_xx = inter_x_lo - tiff_x;
                        let n = inter_x_hi - inter_x_lo;
                        let src = &values[fgd_row * fgd_w + fgd_x..fgd_row * fgd_w + fgd_x + n];
                        let dst = &mut buf
                            [tiff_row * TIFF_TILE_W + tiff_xx..tiff_row * TIFF_TILE_W + tiff_xx + n];
                        dst.copy_from_slice(src);
                    }

                    let rem = remaining.get_mut(&(tc, tr)).unwrap();
                    *rem -= 1;
                    if *rem == 0 {
                        let buf = active.remove(&(tc, tr)).unwrap();
                        flush_tile(&mut writer, tc, tr, buf, width, height)?;
                        remaining.remove(&(tc, tr));
                    }
                }
            }
        }
    }

    debug_assert!(active.is_empty());
    debug_assert!(remaining.is_empty());

    writer.finish()?;
    Ok(())
}

fn tiff_tile_range(
    col_off: usize,
    row_off: usize,
    fgd_w: usize,
    fgd_h: usize,
) -> (usize, usize, usize, usize) {
    let tc_lo = col_off / TIFF_TILE_W;
    let tc_hi = (col_off + fgd_w - 1) / TIFF_TILE_W;
    let tr_lo = row_off / TIFF_TILE_H;
    let tr_hi = (row_off + fgd_h - 1) / TIFF_TILE_H;
    (tc_lo, tc_hi, tr_lo, tr_hi)
}

fn flush_tile<W: std::io::Write + std::io::Seek>(
    writer: &mut geotiff_writer::StreamingTileWriter<f32, W>,
    tc: usize,
    tr: usize,
    buf: Vec<f32>,
    width: usize,
    height: usize,
) -> Result<()> {
    let tiff_x = tc * TIFF_TILE_W;
    let tiff_y = tr * TIFF_TILE_H;
    let expected_w = (width - tiff_x).min(TIFF_TILE_W);
    let expected_h = (height - tiff_y).min(TIFF_TILE_H);

    let arr = Array2::from_shape_vec((TIFF_TILE_H, TIFF_TILE_W), buf)?;
    let view = arr.slice(ndarray::s![0..expected_h, 0..expected_w]);
    writer.write_tile(tiff_x, tiff_y, &view)?;
    Ok(())
}
