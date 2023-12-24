use std::fs::File;
use std::io::{Read, Write};
use std::ops::{AddAssign, BitOrAssign};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use csv::WriterBuilder;
use image::{ImageBuffer, Luma};
use las::{Point, Read as LasRead, Reader};
use log::{info, warn};
use num::Zero;


use crate::utils;
use crate::utils::{get_point_deltas_within_radius, scale_float_to_uint_range, x_y_to_index};
use serde::{Deserialize, Serialize};
use crate::errors::LasToStlError;
use crate::mask::Mask;
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






/// A sum and counter to be able to do calculations after reading from file.
/// This should only be used in the context of loading LAS files(s) into a heightmap
/// This should probably not be public, but I don't believe in private fields. so just think about what you're doing if you want to use this.
#[derive(Copy, Clone)]
pub struct PointAggregate{
    point_sum: f64,
    num_points: u16
}

impl Default for PointAggregate {
    fn default() -> Self {
        PointAggregate {
            point_sum: 0f64,
            num_points: 0u16
        }
    }
}

impl PointAggregate{
    pub fn add_sample(&mut self, new_height: f64){
        self.point_sum += new_height;
        self.num_points += 1;
    }

    /// returns average height from all added values, or `default` if there were no samples.
    pub fn get_average_or_default(&self, default: f64) -> f64{
        if self.num_points.is_zero(){
            default
        } else {
            self.point_sum / self.num_points as f64
        }

    }
}

/// The precursor to a heightmap. this should only be used in the context of loading data from LAS/LAZ file(s)
/// Contains relevant precalculated values and a vec of `PointAggregate`s. This should probably not be public,
/// but I don't believe in private fields. so just think about what you're doing if you want to use this.
pub struct HeightMapIntermediate{
    pub data: Vec<PointAggregate>,
    pub x_res: usize,
    pub y_res: usize,
    pub x_tick: f64,
    pub y_tick: f64,
    pub x_offset: f64,
    pub y_offset: f64,
    pub bounds: UtmBoundingBox,
}

impl HeightMapIntermediate{

    /// Creates a new `HeightMapIntermediate` ready to receive data from LAS files
    /// This should probably not be public, but I don't believe in private fields. so just think about what you're doing if you want to use this.
    pub fn new(x_res: usize, y_res: usize, utm_bounds: UtmBoundingBox) -> HeightMapIntermediate{

        let x_range = utm_bounds.x_range();
        let y_range = utm_bounds.y_range();

        let x_tick: f64 = x_range / (x_res - 1) as f64;
        let y_tick: f64 = y_range / (y_res - 1) as f64;

        HeightMapIntermediate{
            data: vec![PointAggregate::default(); x_res*y_res],
            x_res,
            y_res,
            x_tick,
            y_tick,
            x_offset: utm_bounds.min_x,
            y_offset: utm_bounds.min_y,
            bounds: utm_bounds,
        }
    }

    /// returns the index of where the point should go in data. This could be used in conjunction
    /// with `add_point_by_index` to allow some multithreading on these operations, as opposed to
    /// `add_point_unchecked` which is single thread.
    /// However I believe the speed bottleneck is disk read speed as well as the LAS library, not these calculations
    /// This should probably not be public, but I don't believe in private fields. so just think about what you're doing if you want to use this.
    pub fn get_index(&self, new_point: Point) -> usize{
        let x: usize = ((new_point.x - self.x_offset) / self.x_tick) as usize;
        let y: usize = ((new_point.y - self.y_offset) / self.y_tick) as usize;

        (y*self.x_res) + x
    }

    /// adds a height value by its index.
    /// This should probably not be public, but I don't believe in private fields. so just think about what you're doing if you want to use this.
    pub fn add_point_by_index(&mut self, height: f64, index: usize){
        if index > self.x_res * self.y_res{
            println!("out of bounds point, moving on");
            return;
        }
        self.data[index].add_sample(height)
    }

    /// adds a point from a LAS/LAZ file, mildly (01.09%) faster that `add_point`
    /// This should probably not be public, but I don't believe in private fields. so just think about what you're doing if you want to use this.
    pub fn add_point_unchecked(&mut self, new_point: Point){
        let new_height: f64 = new_point.z;
        let x: usize = ((new_point.x - self.x_offset) / self.x_tick) as usize;
        let y: usize = ((new_point.y - self.y_offset) / self.y_tick) as usize;

        //total time with previous bounds check: Ok(645.8486943s). Without bounds check: Ok(633.5581957s)

        let inverted_y_index = ((self.y_res - y - 1)*self.x_res) + x;
        // let normal_y_index = (y*self.x_res) + x;

        self.data[inverted_y_index].add_sample(new_height);
    }

    /// adds a point from a LAS/LAZ file, mildly (01.09%) slower that `add_point_unchecked`
    /// This should probably not be public, but I don't believe in private fields. so just think about what you're doing if you want to use this.
    pub fn add_point(&mut self, new_point: Point){
        let new_height: f64 = new_point.z;
        let x: usize = ((new_point.x - self.x_offset) / self.x_tick) as usize;
        let y: usize = ((new_point.y - self.y_offset) / self.y_tick) as usize;


        if x < self.x_res && y < self.y_res{


            let inverted_y_index = ((self.y_res - y - 1)*self.x_res) + x;
            // let normal_y_index = (y*self.x_res) + x;

            self.data[inverted_y_index].add_sample(new_height);
        }

    }
}

/// A grid of height values (in meters) spanning `bounds` (in utm)
/// The primary struct used by this library
#[derive(Serialize, Deserialize, Debug)]
pub struct HeightMap{
    pub data: Vec<f64>,
    pub x_res: usize,
    pub y_res: usize,
    pub bounds: UtmBoundingBox
}

impl HeightMap{

    /// Tet the height at x, y. The coordinates are unit-less but evenly spaced.
    pub fn get_height(&self, x: usize, y: usize) -> Result<f64, LasToStlError>{
        Ok(self.data[x_y_to_index(self.x_res, self.y_res, x, y)?])
    }

    /// This was used at some point as a sanity check to validate the data, but now that image and stl work, this is pointless.
    /// Nonetheless I will keep it for that on MF who wants his height data represented by a unit-less csv file.
    pub fn save_to_csv<P: AsRef<Path>>(&self, path: P) -> Result<(), LasToStlError>{
        let mut output = WriterBuilder::new().has_headers(false).from_path(path)?;
        for row in self.data.chunks(self.x_res){
            output.serialize(row).unwrap();
            output.flush().unwrap();
        };
        Ok(())
    }

    /// Loads from a JSON file. Extremely useful because parsing LAS/LAZ data can take a while
    /// (depending on the area ofc) but adding kml regions and waypoints is almost instant.
    /// So instead of rerunning the entire process to add a waypoint you can just load the JSON of
    /// the same region and avoid parsing the same data over and over.
    /// This does NOT use a standard format and unless this project goes viral, will never be a standard.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<HeightMap, LasToStlError> {
        let mut file = File::open(path)?;
        let mut buf = vec![];
        file.read_to_end(&mut buf)?;
        serde_json::from_slice::<HeightMap>(&buf[..]).map_err(|e|{LasToStlError::SerdeError(e)})
    }

    /// Saves to a JSON file. Extremely useful because parsing LAS/LAZ data can take a while
    /// (depending on the area ofc) but adding kml regions and waypoints is almost instant.
    /// So instead of rerunning the entire process to add a waypoint you can just load the JSON of
    /// the same region and avoid parsing the same data over and over.
    /// This does NOT use a standard format and unless this project goes viral, will never be a standard.
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), LasToStlError>{
        let mut f = File::create(path)?;
        let buf = serde_json::to_vec(self)?;
        f.write_all(&buf[..])?;

        Ok(())
    }

    /// saves as a black and white png with brightness representing relative height.
    /// This is useful for doing a sanity check on your data and comparing it to a map
    pub fn save_to_image<P: AsRef<Path>>(&self, path: P) -> Result<(), LasToStlError>{
        let image: ImageBuffer<Luma<u8>, Vec<u8>> = ImageBuffer::from_vec(
            self.x_res as u32,
            self.y_res as u32,
            self.data.iter().map(|height| {
                scale_float_to_uint_range(height, self.bounds.min_z, self.bounds.max_z, 255) as u8
            }).collect()
        ).ok_or(LasToStlError::ImageNoneError)?;

        // write it out to a file
        image.save(path)?;
        Ok(())
    }

    /// adds `offset` to all height values with coordinates that are set to true in mask.
    /// Mask must have the same resolution and bounds as self.
    ///
    /// You can guarantee this by constructing the mask with parameters from the heightmap you intend on applying it to.
    ///
    /// (e.g. `Mask::new_with_dims(hm.x_res, hm.y_res, hm.bounds)`)
    pub fn offset_by_mask(&mut self, mask: &Mask, offset: f64) -> Result<(), LasToStlError>{
        if self.x_res == mask.x_res && self.y_res == mask.y_res && self.bounds == mask.bounds{
            let data_iter = self.data.iter_mut();
            let mut mask_iter = mask.data.iter();

            for height in data_iter{
                if *mask_iter.next().unwrap() {
                    height.add_assign(offset);
                }
            }

            Ok(())
        } else {
            Err(LasToStlError::MaskBoundMismatchError{
                heightmap_x_res: self.x_res,
                heightmap_y_res: self.y_res,
                mask_x_res: mask.x_res,
                mask_y_res: mask.y_res,
                heightmap_bounds: self.bounds,
                mask_bounds: mask.bounds,
            })
        }
    }
}

impl From<HeightMapIntermediate> for HeightMap{

    /// converts a `HeightMapIntermediate` into a `HeightMap`.
    /// This is lossy, but `HeightMapIntermediate` only serves to be converted into a `HeightMap`.
    ///
    /// This should probably not be public, but I don't believe in private fields. so just think about what you're doing if you want to use this.
    fn from(height_map_intermediate: HeightMapIntermediate) -> Self{
        HeightMap{
            data: height_map_intermediate.data.iter().map(|p| {p.get_average_or_default(height_map_intermediate.bounds.min_z)}).collect(),
            x_res: height_map_intermediate.x_res,
            y_res: height_map_intermediate.y_res,
            bounds: height_map_intermediate.bounds,
        }

    }
}

