use std::fs::File;
use std::io::{Read, Write};
use std::ops::{AddAssign};
use std::path::{Path};
use csv::WriterBuilder;
use image::{ImageBuffer, Luma};
use las::Point;
use num::Zero;

use crate::utils::{scale_float_to_uint_range, x_y_to_index};
use serde::{Deserialize, Serialize};
use crate::errors::LasToStlError;
use crate::mask::Mask;
use crate::utm_bounds::UtmBoundingBox;




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

    /// number of bins on the x axis
    pub x_res: usize,

    /// number of bins on the y axis
    pub y_res: usize,

    /// meters (or units if you aren't using UTM) per pixel on the x axis
    pub x_tick: f64,

    /// meters (or units if you aren't using UTM) per pixel on the y axis
    pub y_tick: f64,

    /// x value of pixel at (0, 0) in meters (or units if you aren't using UTM)
    pub x_offset: f64,

    /// y value of pixel at (0, 0) in meters (or units if you aren't using UTM)
    pub y_offset: f64,

    /// bounds in meters (or units if you aren't using UTM)
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

        // let inverted_y_index = ((self.y_res - y - 1)*self.x_res) + x;
        let normal_y_index = (y*self.x_res) + x;

        self.data[normal_y_index].add_sample(new_height);
    }

    /// adds a point from a LAS/LAZ file, mildly (01.09%) slower that `add_point_unchecked`
    /// This should probably not be public, but I don't believe in private fields. so just think about what you're doing if you want to use this.
    pub fn add_point(&mut self, new_point: Point){
        let new_height: f64 = new_point.z;
        let x: usize = ((new_point.x - self.x_offset) / self.x_tick) as usize;
        let y: usize = ((new_point.y - self.y_offset) / self.y_tick) as usize;


        if x < self.x_res && y < self.y_res{

            // let inverted_y_index = ((self.y_res - y - 1)*self.x_res) + x;
            let normal_y_index = (y*self.x_res) + x;

            self.data[normal_y_index].add_sample(new_height);
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
            output.serialize(row)?;
            output.flush()?;
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
    /// This is useful for doing a sanity check on your data and comparing it to a map.
    ///
    /// Note that the image is vertically flipped.
    /// This is normal and means that the stl data will be correct when saved as STL.
    /// If that's a problem, rotate your monitor and then it will be horizontally flipped.
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
            let mask_iter = mask.data.iter();

            for (height, mask_state) in data_iter.zip(mask_iter){
                if *mask_state {
                    height.add_assign(offset);
                }
            }

            Ok(())
        } else {
            Err(LasToStlError::MaskBoundMismatchError{
                other_x_res: self.x_res,
                other_y_res: self.y_res,
                mask_x_res: mask.x_res,
                mask_y_res: mask.y_res,
                other_bounds: self.bounds,
                mask_bounds: mask.bounds,
            })
        }
    }

    /// assigns `value_to_set_where_mask_true` to all points with coordinates that are set to true in `mask`.
    /// Mask must have the same resolution and bounds as self.
    ///
    /// You can guarantee this by constructing the mask with parameters from the heightmap you intend on applying it to.
    ///
    /// (e.g. `Mask::new_with_dims(hm.x_res, hm.y_res, hm.bounds)`)
    pub fn set_by_mask(&mut self, mask: &Mask, value_to_set_where_mask_true: f64) -> Result<(), LasToStlError>{
        if self.x_res == mask.x_res && self.y_res == mask.y_res && self.bounds == mask.bounds{
            let data_iter = self.data.iter_mut();
            let mask_iter = mask.data.iter();

            for (height, mask_state) in data_iter.zip(mask_iter){
                if *mask_state {
                    *height = value_to_set_where_mask_true;
                }
            }

            Ok(())
        } else {
            Err(LasToStlError::MaskBoundMismatchError{
                other_x_res: self.x_res,
                other_y_res: self.y_res,
                mask_x_res: mask.x_res,
                mask_y_res: mask.y_res,
                other_bounds: self.bounds,
                mask_bounds: mask.bounds,
            })
        }
    }

    /// to change the units to proper UTM (which is assumed for all heightmaps),
    /// convert this object's bounds to UTM and pass them here. Z axis does not matter.
    /// [espg](https://epsg.io/) is a great tool for converting various coordinate systems
    ///
    /// despite the name, there is no 'checked' variation (for now)
    pub fn convert_projection_unchecked(&mut self, new_bounds: UtmBoundingBox){
        self.bounds = new_bounds;
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

