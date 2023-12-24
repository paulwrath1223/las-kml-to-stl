use std::fmt::Display;
use std::path::PathBuf;
use las::{Bounds, Read, Reader};
use serde::{Deserialize, Serialize};
use geo::Coord;
use log::info;
use crate::errors::LasToStlError;
use crate::utils::{f64_max, f64_min, get_paths};

/// Bounds for 3d space in UTM form. This is used to convert between UTM objects and unit-less discrete grids
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct UtmBoundingBox {
    pub min_x: f64,
    pub max_x: f64,
    pub min_y: f64,
    pub max_y: f64,
    pub min_z: f64,
    pub max_z: f64
}

impl UtmBoundingBox {
    pub fn new(min_x: f64, max_x: f64, min_y: f64, max_y: f64, min_z: f64, max_z: f64) -> Self{
        UtmBoundingBox {
            min_x,
            max_x,
            min_y,
            max_y,
            min_z,
            max_z
        }
    }

    /// this function creates a new `UtmBoundingBox` from a LAS or LAZ file.
    pub fn get_bounds_from_las(path_buf: &PathBuf) -> Result<UtmBoundingBox, LasToStlError> {
        let reader = Reader::from_path(path_buf)?;
        let b: Bounds = reader.header().bounds();
        Ok(UtmBoundingBox {
            min_x: b.min.x,
            max_x: b.max.x,
            min_y: b.min.y,
            max_y: b.max.y,
            min_z: b.min.z,
            max_z: b.max.z,
        })
    }

    /// adds another bounding box to self, making self include all points in both regions
    pub fn add(&mut self, other: UtmBoundingBox){
        self.min_x = f64_min(self.min_x, other.min_x);
        self.max_x = f64_max(self.max_x, other.max_x);
        self.min_y = f64_min(self.min_y, other.min_y);
        self.max_y = f64_max(self.max_y, other.max_y);
        self.min_z = f64_min(self.min_z, other.min_z);
        self.max_z = f64_max(self.max_z, other.max_z);
    }

    /// adds a UTM coordinate, changes self to include the UTM coordinate
    pub fn add_utm(&mut self, utm_coord: Coord){
        self.min_x = f64_min(self.min_x, utm_coord.x);
        self.max_x = f64_max(self.max_x, utm_coord.x);
        self.min_y = f64_min(self.min_y, utm_coord.y);
        self.max_y = f64_max(self.max_y, utm_coord.y);
    }

    /// Creates a new `UtmBoundingBox` to include all LAS/LAZ data from the provided paths.
    /// Paths should be to individual LAS files, if you want to do a folder use `utils::get_paths`.
    /// Logs info about the process using because it can take around 10 seconds for large data sets.
    ///
    /// logging done with log::info (https://docs.rs/log/latest/log/enum.Level.html#variant.Info)
    pub fn get_bounds_from_las_paths(las_paths: &Vec<PathBuf>) -> Result<UtmBoundingBox, LasToStlError> {

        let mut global_bounds = UtmBoundingBox::default();

        let num_files = las_paths.len();

        let mut count: usize = 1;

        info!("finding bounds of {num_files} files");

        for path in las_paths{
            info!("bounding... {count} / {num_files}");
            global_bounds.add(UtmBoundingBox::get_bounds_from_las(path)?);
            count += 1;
        }
        Ok(global_bounds)
    }

    /// Gets the difference of the largest and smallest x values
    pub fn x_range(&self) -> f64 {
        self.max_x - self.min_x
    }

    /// Gets the difference of the largest and smallest y values
    pub fn y_range(&self) -> f64 {
        self.max_y - self.min_y
    }

    /// Gets the difference of the largest and smallest z values
    pub fn z_range(&self) -> f64 {
        self.max_z - self.min_z
    }
}

impl PartialEq for UtmBoundingBox {
    fn eq(&self, other_bounds: &UtmBoundingBox) -> bool {
        self.min_x == other_bounds.min_x &&
            self.max_x == other_bounds.max_x &&
            self.min_y == other_bounds.min_y &&
            self.max_y == other_bounds.max_y &&
            self.min_z == other_bounds.min_z &&
            self.max_z == other_bounds.max_z
    }
}

impl Default for UtmBoundingBox {

    /// Defaults to an impossible range that WILL cause errors if used by itself.
    /// (using default is ok, but at least one other bound must be added)
    ///
    /// This is done so that adding any range to a default will just turn it into the added bounds.
    fn default() -> Self {
        UtmBoundingBox {
            min_x: f64::MAX,
            max_x: f64::MIN,
            min_y: f64::MAX,
            max_y: f64::MIN,
            min_z: f64::MAX,
            max_z: f64::MIN
        }
    }
}

impl Display for UtmBoundingBox{
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(formatter,
               "x: ({}, {})\
                x: ({}, {})\
                x: ({}, {})",
               self.min_x, self.max_x, self.min_y, self.max_y, self.min_z, self.max_z)
    }
}