use clap::{Arg, ArgAction, Command};
use rayon::prelude::*;
use std::str::FromStr;

pub mod dataset;
mod rasterize;

pub use rasterize::CompressionKind;

#[derive(Debug)]
pub struct Config {
    input_files: Vec<String>,
    output_dir: String,
    merge: bool,
    compression: CompressionKind,
}

pub fn get_args() -> Result<Config, Box<dyn std::error::Error>> {
    let matches = Command::new("fgddem")
        .bin_name("fgddem")
        .version("0.1.0")
        .author("Kanahiro Iguchi")
        .args([
            Arg::new("input").value_name("INPUT").num_args(1..),
            Arg::new("output_dir").value_name("OUTPUT_DIR").short('o'),
            Arg::new("merge")
                .long("merge")
                .short('m')
                .action(ArgAction::SetTrue)
                .help("Merge all inputs into a single GeoTIFF (merged.tif)"),
            Arg::new("compression")
                .long("compression")
                .short('c')
                .value_name("KIND")
                .default_value("deflate")
                .value_parser(["none", "deflate", "lzw", "zstd"])
                .help("GeoTIFF compression: none, deflate (default), lzw, zstd"),
        ])
        .get_matches();

    let compression = match matches.get_one::<String>("compression").unwrap().as_str() {
        "none" => CompressionKind::None,
        "deflate" => CompressionKind::Deflate,
        "lzw" => CompressionKind::Lzw,
        "zstd" => CompressionKind::Zstd,
        other => return Err(format!("unknown compression: {}", other).into()),
    };

    Ok(Config {
        input_files: matches
            .get_many::<String>("input")
            .unwrap()
            .map(|x| x.to_owned())
            .collect(),
        output_dir: matches.get_one::<String>("output_dir").unwrap().to_owned(),
        merge: matches.get_flag("merge"),
        compression,
    })
}

fn extract_stem(path: &str) -> Result<String, Box<dyn std::error::Error>> {
    match std::path::Path::new(path).file_stem() {
        Some(stem) => match stem.to_str() {
            Some(stem_str) => Ok(stem_str.to_owned()),
            None => Err("Failed to convert stem to string".into()),
        },
        None => Err("Failed to extract stem".into()),
    }
}

pub fn run(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(&config.output_dir)?;

    if config.merge {
        let output_path = std::path::Path::new(&config.output_dir).join("merged.tif");
        rasterize::write_merged_streaming(
            &config.input_files,
            output_path.to_str().unwrap(),
            config.compression,
        )?;
    } else {
        let compression = config.compression;
        config.input_files.par_iter().for_each(|input_file| {
            let basename = extract_stem(&input_file).unwrap();

            let content = std::fs::read_to_string(&input_file).unwrap();
            let dataset = dataset::Dataset::from_str(&content).unwrap();
            rasterize::write(
                dataset.get_grid_shape(),
                dataset.get_extent(),
                dataset.get_grid_values(),
                std::path::Path::new(&config.output_dir)
                    .join(format!("{}.tif", basename))
                    .to_str()
                    .unwrap(),
                compression,
            )
            .unwrap();
        });
    }
    Ok(())
}

#[test]
fn test_extract_stem() {
    assert_eq!(
        extract_stem("tests/fixture/FG-GML-5238-74-00-DEM5A-20161001.xml").unwrap(),
        "FG-GML-5238-74-00-DEM5A-20161001"
    );
}

#[test]
fn test_extract_stem_no_filename() {
    // A path ending in `..` has no file_stem -> error branch.
    assert!(extract_stem("..").is_err());
}
