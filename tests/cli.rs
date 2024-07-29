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
