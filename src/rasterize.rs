use gdal::raster::Buffer;
use gdal::spatial_ref::SpatialRef;
use gdal::DriverManager;

pub fn write(
    grid_shape: (usize, usize),
    extent: ((f64, f64), (f64, f64)),
    values: &Vec<f64>,
    output: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let driver = DriverManager::get_driver_by_name("GTiff")?;
    let mut dataset =
        driver.create_with_band_type::<f64, _>(output, grid_shape.0, grid_shape.1, 1)?;

    let x_res = (extent.1 .1 - extent.0 .1) / grid_shape.0 as f64;
    let y_res = (extent.1 .0 - extent.0 .0) / grid_shape.1 as f64;
    dataset.set_geo_transform(&[extent.0 .1, x_res, 0.0, extent.1 .0, 0.0, -y_res])?;

    let sr = SpatialRef::from_epsg(6668)?;
    dataset.set_spatial_ref(&sr)?;

    let mut band_buffer = Buffer::new(grid_shape, values.clone());
    dataset
        .rasterband(1)?
        .write((0, 0), grid_shape, &mut band_buffer)?;

    Ok(())
}
