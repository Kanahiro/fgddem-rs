use anyhow::Result;
use serde::Deserialize;
use std::fmt::Debug;
use std::str::FromStr;

#[derive(Deserialize, Debug)]
pub struct Dataset {
    //id: String,
    //description: String,
    //name: String,
    DEM: Dem,
}

#[derive(Deserialize, Debug)]
struct Dem {
    // fid: String,
    // mesh: String,
    coverage: Coverage,
}

#[derive(Deserialize, Debug)]
struct Coverage {
    boundedBy: BoundedBy,
    gridDomain: GridDomain,
    rangeSet: RangeSet,
}

#[derive(Deserialize, Debug)]
struct BoundedBy {
    Envelope: Envelope,
}

#[derive(Deserialize, Debug)]
struct Envelope {
    // srsName: String,
    #[serde(deserialize_with = "parse_tuple_as_numeric")]
    lowerCorner: (f64, f64),
    #[serde(deserialize_with = "parse_tuple_as_numeric")]
    upperCorner: (f64, f64),
}

#[derive(Deserialize, Debug)]
struct GridDomain {
    Grid: Grid,
}

#[derive(Deserialize, Debug)]
struct Grid {
    limits: Limits,
}

#[derive(Deserialize, Debug)]
struct Limits {
    GridEnvelope: GridEnvelope,
}

#[derive(Deserialize, Debug)]
struct GridEnvelope {
    #[serde(deserialize_with = "parse_tuple_as_numeric")]
    low: (usize, usize),
    #[serde(deserialize_with = "parse_tuple_as_numeric")]
    high: (usize, usize),
}

#[derive(Deserialize, Debug)]
struct RangeSet {
    DataBlock: DataBlock,
}

#[derive(Deserialize, Debug)]
struct DataBlock {
    #[serde(deserialize_with = "parse_tupleList")]
    tupleList: Vec<f64>,
}

fn parse_tuple_as_numeric<'de, D, T>(deserializer: D) -> Result<(T, T), D::Error>
where
    D: serde::Deserializer<'de>,
    T: FromStr + Debug,
    <T as FromStr>::Err: Debug,
{
    let s = String::deserialize(deserializer)?;
    let v: Vec<&str> = s.split_whitespace().collect();
    let x: T = v[0].parse().unwrap();
    let y: T = v[1].parse().unwrap();
    Ok((x, y))
}

fn parse_tupleList<'de, D>(deserializer: D) -> Result<Vec<f64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let v = s
        .lines()
        .map(|x| {
            let seg: Vec<String> = x.split(',').map(|x| x.to_string()).collect();
            seg[1].parse().unwrap()
        })
        .collect();
    Ok(v)
}

pub fn parse(content: &str) -> Result<Dataset> {
    let dataset = serde_xml_rs::from_str(content)?;
    Ok(dataset)
}

impl FromStr for Dataset {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        parse(s)
    }
}

impl Dataset {
    pub fn get_extent(&self) -> ((f64, f64), (f64, f64)) {
        let lower = self.DEM.coverage.boundedBy.Envelope.lowerCorner;
        let upper = self.DEM.coverage.boundedBy.Envelope.upperCorner;
        (lower, upper)
    }

    pub fn get_grid_shape(&self) -> (usize, usize) {
        let low = self.DEM.coverage.gridDomain.Grid.limits.GridEnvelope.low;
        let high = self.DEM.coverage.gridDomain.Grid.limits.GridEnvelope.high;
        let x = high.0 - low.0 + 1;
        let y = high.1 - low.1 + 1;
        (x, y)
    }

    pub fn get_grid_values(&self) -> &Vec<f64> {
        &self.DEM.coverage.rangeSet.DataBlock.tupleList
    }
}
