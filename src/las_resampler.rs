use std::time::SystemTime;
use las::{Read, Reader};
use log::{info, warn};
use crate::errors::LasToStlError;
use crate::height_map::{HeightMap, HeightMapIntermediate};
use crate::utils;
use crate::utm_bounds::UtmBoundingBox;



/// Creates a heightmap of specified resolution. The resolution is defines how many samples to take of the terrain,
/// so even 1000 will be way more than necessary.
/// Using a resolution too high will result in pixels with no height data
/// (defaulting to the lowest point seen in the dataset)
///
/// (use None in x or y resolution to auto calculate the other based on the aspect ratio)
///
/// The resolution also happens to represent the (default? if I don't add a way to scale the stl)
/// size of the stl model in units
/// (STL doesn't have units so it doesn't matter, but most software will assume mm)
///
/// This takes a long time and logs info with log::info
/// (https://docs.rs/log/latest/log/enum.Level.html#variant.Info)

impl HeightMap{
    pub fn glob_get_height_map(glob_pattern: &str, resolution_x_in: Option<usize>, resolution_y_in: Option<usize>)
                               -> Result<HeightMap, LasToStlError>
    {

        let paths = utils::get_paths(glob_pattern)?;
        // get a bound on all data
        let bounds = UtmBoundingBox::get_bounds_from_las_paths(&paths)?;

        let x_range = bounds.x_range();
        let y_range = bounds.y_range();

        let (mut resolution_x, mut resolution_y): (usize, usize);

        match (resolution_x_in, resolution_y_in){
            (Some(x), Some(y)) => {
                resolution_x = x;
                resolution_y = y;
            },
            (Some(x), None) => {
                resolution_x = x;
                resolution_y = ((x as f64) * (y_range/x_range)) as usize;
            },
            (None, Some(y)) => {
                resolution_x = ((y as f64) * (x_range/y_range)) as usize;
                resolution_y = y;
            },
            (None, None) => {
                return Err(LasToStlError::NoResolutionError)
            }
        }


        // create a height map intermediate to hold the data while reading LAS files.
        // This struct should not be used in any other context
        let mut height_map_intermediate = HeightMapIntermediate::new(resolution_x, resolution_y, bounds);

        // the 'index' of the file being processed (starting at 1)
        let mut current_file_number: usize = 1;

        let global_now = SystemTime::now();

        let num_files = paths.len();

        for path in paths{

            let now = SystemTime::now();

            match Reader::from_path(&path){
                Ok(mut reader) => {
                    let num_points = reader.header().number_of_points();

                    let display_path = path.display().to_string();

                    info!("Number of points: {num_points} in {display_path}");

                    let mut counter: usize = 0;
                    for wrapped_point_result in reader.points() {
                        match wrapped_point_result{
                            Ok(wrapped_point) => {
                                height_map_intermediate.add_point_unchecked(wrapped_point); // TODO: spawn this in a new thread
                                counter += 1;

                                // total time with this \/ check: Ok(633.5581957s). Without: Ok(632.358371s)

                                if counter % 4194304 == 0 {
                                    info!("{:.2}% done with {display_path}. (file {current_file_number} / {num_files})", 100f64 * counter as f64 / num_points as f64);
                                }
                            }
                            Err(e) => {
                                warn!("reader failed to data point in file {:?} with error:\n\t{:?}\nSkipping point.", path.display(), e)
                            }
                        }
                    }

                    println!("file {current_file_number} / {num_files} took {:?} seconds", now.elapsed());
                    current_file_number += 1;
                }
                Err(e) => {
                    warn!("reader failed to read file {:?} with error:\n\t{:?}\nSkipping file.", path.display(), e)
                }
            };
        }
        info!("loading all {num_files} files took {:?}", global_now.elapsed());

        Ok(HeightMap::from(height_map_intermediate))
    }
}

