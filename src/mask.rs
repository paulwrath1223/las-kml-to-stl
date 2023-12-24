use std::ops::{BitAndAssign, BitOrAssign, BitXorAssign};
use geo::{EuclideanLength, LineInterpolatePoint, LineString, Polygon};
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

    /// resamples and plots a LineString
    pub fn add_trail(&mut self, trail: LineString, dot_radius: u16) -> Result<(), LasToStlError>{

        let trail_length_meters = trail.euclidean_length();
        let avg_meters_per_pixel: f64 = (self.x_tick + self.y_tick) / 2f64;
        let trail_length_pixels = trail_length_meters / avg_meters_per_pixel;

        let target_num_points: usize = (dot_radius as f64 / trail_length_pixels) as usize + 1;

        let deltas: Vec<(i16, i16)> = get_point_deltas_within_radius(dot_radius);
        for i in 0..=target_num_points{

            let interpolated_point = trail.line_interpolate_point(
                i as f64 / target_num_points as f64
            ).ok_or(LasToStlError::InterpolatePointError)?;

            let utm_point: UtmCoord = UtmCoord::from(&interpolated_point);
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

    /// adds a UTM point with specified radius
    /// returns an error if: for any of the points, none of the pixels in or on the radius are within bounds of the mask.
    pub fn add_utm_points(&mut self, utm_coords: Vec<UtmCoord>, dot_radius: u16) -> Result<(), LasToStlError>{
        let deltas: Vec<(i16, i16)> = get_point_deltas_within_radius(dot_radius);
        for utm_coord in utm_coords{
            let (x, y) = utm_coord.get_x_y_coords(self.bounds.min_x, self.bounds.min_y, self.x_tick, self.y_tick);
            self.set_with_deltas(x, y, true, &deltas)?
        }
        Ok(())
    }


    /// Bounds and resolution must match
    pub fn checked_bitor_assign(&mut self, other_mask: Mask) -> Result<(), LasToStlError> {
        if self.x_res == other_mask.x_res && self.y_res == other_mask.y_res && self.bounds == other_mask.bounds{
            self.bitor_assign(other_mask);
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
    pub fn checked_bitand_assign(&mut self, other_mask: Mask) -> Result<(), LasToStlError> {
        if self.x_res == other_mask.x_res && self.y_res == other_mask.y_res && self.bounds == other_mask.bounds{
            self.bitand_assign(other_mask);
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
    pub fn checked_bitxor_assign(&mut self, other_mask: Mask) -> Result<(), LasToStlError> {
        if self.x_res == other_mask.x_res && self.y_res == other_mask.y_res && self.bounds == other_mask.bounds{
            self.bitxor_assign(other_mask);
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

    /// inverts the mask... Duh
    pub fn invert(&mut self){
        self.data.iter_mut().for_each(|mut p| { *p = !*p })
    }
}

impl BitOrAssign for Mask{

    /// this is an unchecked version of `checked_bitor_assign`.
    ///
    /// Resolutions MUST match and while mismatched bounds technically aren't a problem,
    /// please think deeply about what it means to takes a mask from one region and apply a binary operation on a mask somewhere else.
    fn bitor_assign(&mut self, other_mask: Mask) {
        for (own_state, other_state) in self.data.iter_mut().zip(other_mask.data.iter()){
            *own_state |= *other_state;
        }
    }
}

impl BitAndAssign for Mask{

    /// this is an unchecked version of `checked_bitand_assign`.
    ///
    /// Resolutions MUST match and while mismatched bounds technically aren't a problem,
    /// please think deeply about what it means to takes a mask from one region and apply a binary operation on a mask somewhere else.
    fn bitand_assign(&mut self, other_mask: Self) {
        for (own_state, other_state) in self.data.iter_mut().zip(other_mask.data.iter()){
            *own_state &= *other_state;
        }
    }
}

impl BitXorAssign for Mask{

    /// this is an unchecked version of `checked_bitxor_assign`.
    ///
    /// Resolutions MUST match and while mismatched bounds technically aren't a problem,
    /// please think deeply about what it means to takes a mask from one region and apply a binary operation on a mask somewhere else.
    fn bitxor_assign(&mut self, other_mask: Self) {
        for (own_state, other_state) in self.data.iter_mut().zip(other_mask.data.iter()){
            *own_state ^= *other_state;
        }
    }
}