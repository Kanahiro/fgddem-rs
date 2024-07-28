use clap::{Arg, Command};
use rayon::prelude::*;
use std::str::FromStr;

pub mod dataset;
mod rasterize;

#[derive(Debug)]
pub struct Config {
    input_files: Vec<String>,
    output_dir: String,
}

pub fn get_args() -> Result<Config, Box<dyn std::error::Error>> {
    let matches = Command::new("fgddem")
        .bin_name("fgddem")
        .version("0.1.0")
        .author("Kanahiro Iguchi")
        .args([
            Arg::new("input").value_name("INPUT").num_args(1..),
            Arg::new("output_dir").value_name("OUTPUT_DIR").short('o'),
        ])
        .get_matches();

    Ok(Config {
        input_files: matches
            .get_many::<String>("input")
            .unwrap()
            .map(|x| x.to_owned())
            .collect(),
        output_dir: matches.get_one::<String>("output_dir").unwrap().to_owned(),
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
        );
    });
    Ok(())
}
