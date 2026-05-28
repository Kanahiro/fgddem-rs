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
    tupleList: Vec<f32>,
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

fn parse_tupleList<'de, D>(deserializer: D) -> Result<Vec<f32>, D::Error>
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

impl FromStr for Dataset {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let dataset = quick_xml::de::from_str(s)?;
        Ok(dataset)
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

    pub fn get_grid_values(&self) -> &Vec<f32> {
        &self.DEM.coverage.rangeSet.DataBlock.tupleList
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Metadata {
    pub shape: (usize, usize),
    pub extent: ((f64, f64), (f64, f64)),
}

pub fn parse_metadata(path: &std::path::Path) -> Result<Metadata> {
    use quick_xml::events::Event;
    use quick_xml::reader::Reader;

    let mut reader = Reader::from_file(path)?;
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut lower: Option<(f64, f64)> = None;
    let mut upper: Option<(f64, f64)> = None;
    let mut low: Option<(usize, usize)> = None;
    let mut high: Option<(usize, usize)> = None;

    enum Capture {
        None,
        LowerCorner,
        UpperCorner,
        Low,
        High,
    }
    let mut capture = Capture::None;

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) => {
                capture = match e.local_name().as_ref() {
                    b"lowerCorner" => Capture::LowerCorner,
                    b"upperCorner" => Capture::UpperCorner,
                    b"low" => Capture::Low,
                    b"high" => Capture::High,
                    _ => Capture::None,
                };
            }
            Event::Text(t) => {
                let text = t.unescape()?;
                let parts: Vec<&str> = text.split_whitespace().collect();
                if parts.len() >= 2 {
                    match capture {
                        Capture::LowerCorner => {
                            lower = Some((parts[0].parse()?, parts[1].parse()?));
                        }
                        Capture::UpperCorner => {
                            upper = Some((parts[0].parse()?, parts[1].parse()?));
                        }
                        Capture::Low => {
                            low = Some((parts[0].parse()?, parts[1].parse()?));
                        }
                        Capture::High => {
                            high = Some((parts[0].parse()?, parts[1].parse()?));
                        }
                        Capture::None => {}
                    }
                }
                capture = Capture::None;
            }
            Event::Eof => break,
            _ => {}
        }
        if lower.is_some() && upper.is_some() && low.is_some() && high.is_some() {
            break;
        }
        buf.clear();
    }

    let lower = lower.ok_or_else(|| anyhow::anyhow!("missing lowerCorner"))?;
    let upper = upper.ok_or_else(|| anyhow::anyhow!("missing upperCorner"))?;
    let low = low.ok_or_else(|| anyhow::anyhow!("missing GridEnvelope/low"))?;
    let high = high.ok_or_else(|| anyhow::anyhow!("missing GridEnvelope/high"))?;

    Ok(Metadata {
        shape: (high.0 - low.0 + 1, high.1 - low.1 + 1),
        extent: (lower, upper),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let content = r#"<?xml version="1.0" encoding="UTF-8"?>
<Dataset xsi:schemaLocation="http://fgd.gsi.go.jp/spec/2008/FGD_GMLSchema FGD_GMLSchema.xsd" 
	xmlns:gml="http://www.opengis.net/gml/3.2" 
	xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" 
	xmlns:xlink="http://www.w3.org/1999/xlink" 
	xmlns="http://fgd.gsi.go.jp/spec/2008/FGD_GMLSchema" 
	gml:id="Dataset1">
	<gml:description>基盤地図情報メタデータ ID=fmdid:15-3101</gml:description>
	<gml:name>基盤地図情報ダウンロードデータ（GML版）</gml:name>
	<DEM gml:id="DEM001">
		<fid>fgoid:10-00100-15-60101-52387400</fid>
		<lfSpanFr gml:id="DEM001-1">
			<gml:timePosition>2016-10-01</gml:timePosition>
		</lfSpanFr>
		<devDate gml:id="DEM001-2">
			<gml:timePosition>2016-10-01</gml:timePosition>
		</devDate>
		<orgGILvl>0</orgGILvl>
		<orgMDId>H21C0333 H27HEIGOU</orgMDId>
		<type>5mメッシュ（標高）</type>
		<mesh>52387400</mesh>
		<coverage gml:id="DEM001-3">
			<gml:boundedBy>
				<gml:Envelope srsName="fguuid:jgd2011.bl">
					<gml:lowerCorner>0 0</gml:lowerCorner>
					<gml:upperCorner>2 2</gml:upperCorner>
				</gml:Envelope>
			</gml:boundedBy>
			<gml:gridDomain>
				<gml:Grid dimension="2" gml:id="DEM001-4">
					<gml:limits>
						<gml:GridEnvelope>
							<gml:low>0 0</gml:low>
							<gml:high>2 2</gml:high>
						</gml:GridEnvelope>
					</gml:limits>
					<gml:axisLabels>x y</gml:axisLabels>
				</gml:Grid>
			</gml:gridDomain>
			<gml:rangeSet>
				<gml:DataBlock>
					<gml:rangeParameters>
						<gml:QuantityList uom="DEM構成点"></gml:QuantityList>
					</gml:rangeParameters>
                    <gml:tupleList>
                        地表面,145.30
                        地表面,145.10
                        地表面,144.90
                        地表面,144.90
                        地表面,144.84
                        地表面,144.81
                        地表面,121.90
                        内水面,-9999.
                        内水面,-9999.
                    </gml:tupleList>
				</gml:DataBlock>
			</gml:rangeSet>
			<gml:coverageFunction>
				<gml:GridFunction>
					<gml:sequenceRule order="+x-y">Linear</gml:sequenceRule>
					<gml:startPoint>0 0</gml:startPoint>
				</gml:GridFunction>
			</gml:coverageFunction>
		</coverage>
	</DEM>
</Dataset>"#;
        let dataset = Dataset::from_str(&content).unwrap();
        assert_eq!(dataset.get_extent(), ((0.0, 0.0), (2.0, 2.0)));
        assert_eq!(dataset.get_grid_shape(), (3, 3));
        assert_eq!(
            dataset.get_grid_values(),
            &vec![145.30, 145.10, 144.90, 144.90, 144.84, 144.81, 121.90, -9999., -9999.]
        );
    }

    #[test]
    fn test_parse_metadata_matches_full_parse() {
        let path = std::path::Path::new("tests/fixture/FG-GML-5238-74-00-DEM5A-20161001.xml");
        let meta = parse_metadata(path).unwrap();

        let content = std::fs::read_to_string(path).unwrap();
        let dataset = Dataset::from_str(&content).unwrap();
        assert_eq!(meta.shape, dataset.get_grid_shape());
        assert_eq!(meta.extent, dataset.get_extent());
    }

    #[test]
    fn test_parse_metadata_missing_file() {
        let path = std::path::Path::new("tests/fixture/does-not-exist.xml");
        assert!(parse_metadata(path).is_err());
    }

    #[test]
    fn test_parse_metadata_missing_fields() {
        let dir = std::env::temp_dir().join("fgddem-test-missing-fields");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("partial.xml");
        // Only lowerCorner is present — upperCorner / low / high are missing.
        std::fs::write(
            &path,
            r#"<?xml version="1.0" encoding="UTF-8"?>
<root xmlns:gml="http://www.opengis.net/gml/3.2">
  <gml:lowerCorner>0 0</gml:lowerCorner>
</root>"#,
        )
        .unwrap();
        let err = parse_metadata(&path).unwrap_err();
        assert!(err.to_string().contains("missing"));
    }

    #[test]
    fn test_dataset_from_str_invalid_xml() {
        let res = Dataset::from_str("not xml at all");
        assert!(res.is_err());
    }
}
