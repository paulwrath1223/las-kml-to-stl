pub mod height_map;
pub mod las_resampler;
pub mod errors;
pub mod utils;
pub mod utm_bounds;
pub mod mask;
pub mod kml_utils;
pub mod utm_point;
pub mod stl;

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use simple_logger::SimpleLogger;
    use crate::height_map::HeightMap;
    use crate::mask::Mask;
    use super::*;

    #[test]
    fn test_save_data() {
        SimpleLogger::new().env().init().unwrap();

        let hm = HeightMap::glob_get_height_map("test_laz/paynes creek/*.laz", Some(1080), None).unwrap();

        hm.save("raw_height_map.json").unwrap();

        hm.save_to_image("test_image.png").unwrap();
    }

    /// im using this to test with my data, if you see this and aren't me, get your own data
    #[test]
    fn test_playground() {

    }
}
