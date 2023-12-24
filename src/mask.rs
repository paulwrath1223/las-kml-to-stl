use std::ops::BitOrAssign;
use geo::{EuclideanLength, LineString, Polygon};
use log::warn;

use crate::coords::{RasterizedPolygon, UtmCoord, UtmPolygon, UtmTrail};
use crate::errors::LasToStlError;
use crate::kml_utils::KmlRegion;
use crate::utils::get_point_deltas_within_radius;
use crate::utm_bounds::UtmBoundingBox;
use crate::utm_point::UtmCoord;

/// A Boolean mask intended to span the same region as a heightmap to be able to apply certain
/// functions selectively
pub struct Mask{
    pub data: Vec<bool>,
    pub x_res: usize,
    pub y_res: usize,
    /// This should probably not be public, but I don't believe in private fields.
    /// so just think about what you're doing if you want to use this.
    ///
    /// just a precalculated value that is used often behind the scenes
    pub x_tick: f64,
    /// This should probably not be public, but I don't believe in private fields.
    /// so just think about what you're doing if you want to use this.
    ///
    /// just a precalculated value that is used often behind the scenes
    pub y_tick: f64,
    pub bounds: UtmBoundingBox
}

impl Mask{
    /// sets all points in `deltas` offset by `(x, y)` to `state`
    ///
    /// made for `utils::get_point_deltas_within_radius()` to be able to set a circle centered at
    /// `(x, y)` with a radius to a given state.
    ///
    /// Returns a vec of errors for all points that were out of bounds and could not be set, but will still fill in all
    /// points that are within bounds
    pub fn set_with_deltas(&mut self, x: usize, y: usize, state: bool, deltas: &Vec<(i16, i16)>) -> Result<(), LasToStlError>{
        let mut num_successes: usize = 0;
        for (delta_x, delta_y) in deltas{
            let new_x: usize = (x as i64 + *delta_x as i64) as usize;
            let new_y: usize = (y as i64 + *delta_y as i64) as usize;
            match self.set_x_y(new_x, new_y, state) {
                Ok(_) => {
                    num_successes += 1;
                }
                Err(e) => {
                    warn!("error writing to ({new_x}, {new_y}):\n\t{e}\nskipping point");
                }
            }
        }
        if num_successes == 0 {
            Ok(())
        } else {
            Err(LasToStlError::SetWithDeltaError {
                x_res: self.x_res,
                y_res: self.y_res,
                x,
                y,
            })
        }
    }

    /// sets the state of the point at `(x, y)` to `state`. Returns an error if out of bounds
    pub fn set_x_y(&mut self, x: usize, y: usize, new_state: bool) -> Result<(), LasToStlError>{

        if x < self.x_res && y < self.y_res {
            self.data[(y*self.x_res) + x] = new_state;
            Ok(())
        } else {
            Err(LasToStlError::BadIndexError {
                x_res: self.x_res,
                y_res: self.y_res,
                x,
                y,
            })
        }

    }

    /// Creates a new mask from some basic info. Recommended to get this info from the heightmap it is intended to be applied to
    pub fn new_with_dims(x_res: usize, y_res: usize, bounds: UtmBoundingBox) -> Mask{

        let x_tick: f64 = bounds.x_range() / (x_res - 1) as f64;
        let y_tick: f64 = bounds.y_range() / (y_res - 1) as f64;

        Mask {
            data: vec![false; x_res*y_res],
            x_res,
            y_res,
            x_tick,
            y_tick,
            bounds,
        }
    }

    pub fn add_polygon_outline(&mut self, polygon: Polygon, width: u16){
        todo!("may our god that funny little crab smite the creator of the kml library im using and am too stupid to understand");
        let mut region_as_trail: UtmTrail = UtmTrail(kml_region.ordered_corners);
        region_as_trail.interpolate_until_distance_threshold((width/2) as f64);
        self.add_utm_trail_dotted(region_as_trail, width);
    }

    /// will return an error if any of the points (with radius) have no pixels within bounds
    /// plots every point in the line as circle with radius `dot_radius`
    pub fn add_trail_raw(&mut self, trail: LineString, dot_radius: u16) -> Result<(), LasToStlError>{
        let deltas: Vec<(i16, i16)> = get_point_deltas_within_radius(dot_radius);
        for point in trail{
            let utm_point: UtmCoord = UtmCoord::from(&point);
            let (x, y) = utm_point.get_x_y_coords(self.bounds.min_x, self.bounds.min_y, self.x_tick, self.y_tick);
            self.set_with_deltas(x, y, true, &deltas)?;
        }

        Ok(())
    }

    pub fn add_trail(&mut self, trail: LineString, dot_radius: u16) -> Result<(), LasToStlError>{

        let trail_length = trail.euclidean_length();

        let avg_meters_per_pixel: f64 = (self.x_tick + self.y_tick) / 2f64;

        let target_num_points: f64 = dot_radius as f64 / trail_length;

        let deltas: Vec<(i16, i16)> = get_point_deltas_within_radius(dot_radius);
        for point in trail{
            let utm_point: UtmCoord = UtmCoord::from(&point);
            let (x, y) = utm_point.get_x_y_coords(self.bounds.min_x, self.bounds.min_y, self.x_tick, self.y_tick);
            self.set_with_deltas(x, y, true, &deltas)?;
        }

        Ok(())
    }

    /// adds a UTM coordinate with the specified radius.
    /// If adding multiple points please use `add_utm_points` instead to avoid recalculating deltas
    /// returns an error if none of the pixels in or on the radius are within bounds of the mask.
    pub fn add_utm_point(&mut self, utm_coord: UtmCoord, radius: u16) -> Result<(), LasToStlError>{
        let deltas: Vec<(i16, i16)> = get_point_deltas_within_radius(radius);
        let (x, y) = utm_coord.get_x_y_coords(self.bounds.min_x, self.bounds.min_y, self.x_tick, self.y_tick);
        self.set_with_deltas(x, y, true, &deltas)
    }

    /// returns an error if: for any of the points, none of the pixels in or on the radius are within bounds of the mask.
    pub fn add_utm_points(&mut self, utm_coords: Vec<UtmCoord>, dot_radius: u16) -> Result<(), LasToStlError>{
        let deltas: Vec<(i16, i16)> = get_point_deltas_within_radius(dot_radius);
        for utm_coord in utm_coords{
            let (x, y) = utm_coord.get_x_y_coords(self.bounds.min_x, self.bounds.min_y, self.x_tick, self.y_tick);
            self.set_with_deltas(x, y, true, &deltas)?
        }
        Ok(())
    }

    pub fn add_utm_trails_dotted(&mut self, utm_trails: Vec<UtmTrail>, dot_radius: u16){
        for utm_trail in utm_trails{
            self.add_utm_trail_dotted(utm_trail, dot_radius)
        }
    }

    /// Bounds and resolution must match
    pub fn bitor_assign(&mut self, other_mask: Mask) -> Result<(), LasToStlError> {
        if self.x_res == other_mask.x_res && self.y_res == other_mask.y_res && self.bounds == other_mask.bounds{
            for (own_state, other_state) in self.data.iter_mut().zip(other_mask.data.iter()){
                *own_state |= *other_state;
            }
            Ok(())
        } else {
            Err(LasToStlError::MaskBoundMismatchError {
                other_x_res: other_mask.x_res,
                other_y_res:other_mask.y_res,
                mask_x_res: self.x_res,
                mask_y_res: self.y_res,
                other_bounds: other_mask.bounds,
                mask_bounds: self.bounds,
            })
        }
    }

    /// Bounds and resolution must match
    pub fn bitand_assign(&mut self, other_mask: Mask) -> Result<(), LasToStlError> {
        if self.x_res == other_mask.x_res && self.y_res == other_mask.y_res && self.bounds == other_mask.bounds{
            for (own_state, other_state) in self.data.iter_mut().zip(other_mask.data.iter()){
                *own_state &= *other_state;
            }
            Ok(())
        } else {
            Err(LasToStlError::MaskBoundMismatchError {
                other_x_res: other_mask.x_res,
                other_y_res:other_mask.y_res,
                mask_x_res: self.x_res,
                mask_y_res: self.y_res,
                other_bounds: other_mask.bounds,
                mask_bounds: self.bounds,
            })
        }
    }

    /// Bounds and resolution must match
    pub fn bitxor_assign(&mut self, other_mask: Mask) -> Result<(), LasToStlError> {
        if self.x_res == other_mask.x_res && self.y_res == other_mask.y_res && self.bounds == other_mask.bounds{
            for (own_state, other_state) in self.data.iter_mut().zip(other_mask.data.iter()){
                *own_state ^= *other_state;
            }
            Ok(())
        } else {
            Err(LasToStlError::MaskBoundMismatchError {
                other_x_res: other_mask.x_res,
                other_y_res:other_mask.y_res,
                mask_x_res: self.x_res,
                mask_y_res: self.y_res,
                other_bounds: other_mask.bounds,
                mask_bounds: self.bounds,
            })
        }
    }
}