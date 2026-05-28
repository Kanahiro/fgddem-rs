use assert_cmd::Command;

#[test]
fn run() -> Result<(), Box<dyn std::error::Error>> {
    Command::cargo_bin("fgddem")?
        .args([
            "tests/fixture/FG-GML-5238-74-00-DEM5A-20161001.xml",
            "-o",
            "output",
        ])
        .assert()
        .success();
    Ok(())
}

#[test]
fn run_merge_mode() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::env::temp_dir().join("fgddem-cli-merge");
    let _ = std::fs::remove_dir_all(&out_dir);
    Command::cargo_bin("fgddem")?
        .args([
            "tests/fixture/FG-GML-5238-74-00-DEM5A-20161001.xml",
            "-o",
            out_dir.to_str().unwrap(),
            "--merge",
        ])
        .assert()
        .success();
    assert!(out_dir.join("merged.tif").exists());
    Ok(())
}

#[test]
fn run_with_lzw_compression() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::env::temp_dir().join("fgddem-cli-lzw");
    let _ = std::fs::remove_dir_all(&out_dir);
    Command::cargo_bin("fgddem")?
        .args([
            "tests/fixture/FG-GML-5238-74-00-DEM5A-20161001.xml",
            "-o",
            out_dir.to_str().unwrap(),
            "-c",
            "lzw",
        ])
        .assert()
        .success();
    assert!(out_dir
        .join("FG-GML-5238-74-00-DEM5A-20161001.tif")
        .exists());
    Ok(())
}

#[test]
fn run_rejects_unknown_compression() -> Result<(), Box<dyn std::error::Error>> {
    Command::cargo_bin("fgddem")?
        .args([
            "tests/fixture/FG-GML-5238-74-00-DEM5A-20161001.xml",
            "-o",
            "output",
            "-c",
            "bogus",
        ])
        .assert()
        .failure();
    Ok(())
}
