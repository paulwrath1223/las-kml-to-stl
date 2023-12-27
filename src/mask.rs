use std::ops::{BitAndAssign, BitOrAssign, BitXorAssign, SubAssign};
use geo::{BoundingRect, Contains, Coord, EuclideanLength, LineInterpolatePoint, LineString, Point, Polygon};
use log::{error, info, trace, warn};
use crate::errors::LasToStlError;
use crate::kml_utils::{linestring_to_utm_linestring, polygon_to_utm_polygon};
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
    /// (meters per pixel in x axis)
    pub x_tick: f64,
    /// This should probably not be public, but I don't believe in private fields.
    /// so just think about what you're doing if you want to use this.
    ///
    /// just a precalculated value that is used often behind the scenes
    /// (meters per pixel in y axis)
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
            Err(LasToStlError::SetWithDeltaError {
                x_res: self.x_res,
                y_res: self.y_res,
                x,
                y,
            })
        } else {
            Ok(())
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
    pub fn add_trail_raw(&mut self, trail: &LineString, dot_radius: u16) -> Result<(), LasToStlError>{
        let deltas: Vec<(i16, i16)> = get_point_deltas_within_radius(dot_radius);
        for point in trail{
            let utm_point: UtmCoord = UtmCoord::from(point);
            let (x, y) = utm_point.get_x_y_coords(self.bounds.min_x, self.bounds.min_y, self.x_tick, self.y_tick);
            self.set_with_deltas(x, y, true, &deltas)?;
        }

        Ok(())
    }

    /// resamples and plots a LineString
    pub fn add_utm_trail_auto_sample(&mut self, utm_trail: &LineString, dot_radius: u16) -> Result<(), LasToStlError>{

        let trail_length_meters = utm_trail.euclidean_length();
        let avg_meters_per_pixel: f64 = (self.x_tick + self.y_tick) / 2f64;
        let trail_length_pixels = trail_length_meters / avg_meters_per_pixel;

        let target_num_points: usize = (dot_radius as f64 / trail_length_pixels) as usize + 1;

        self.add_utm_trail(&utm_trail, dot_radius, target_num_points)
    }

    /// resamples and plots a LineString
    pub fn add_lat_lon_trail_auto_sample(&mut self, lat_lon_trail: &LineString, dot_radius: u16) -> Result<(), LasToStlError>{

        let utm_trail = linestring_to_utm_linestring(lat_lon_trail);

        self.add_utm_trail_auto_sample(&utm_trail, dot_radius)
    }

    pub fn add_lat_lon_trail(&mut self, lat_lon_trail: &LineString, dot_radius: u16, target_num_points: usize) -> Result<(), LasToStlError>{
        self.add_utm_trail(&linestring_to_utm_linestring(&lat_lon_trail), dot_radius, target_num_points)
    }

    pub fn add_utm_trail(&mut self, utm_trail: &LineString, dot_radius: u16, target_num_points: usize) -> Result<(), LasToStlError>{

        let deltas: Vec<(i16, i16)> = get_point_deltas_within_radius(dot_radius);
        for i in 0..=target_num_points{

            let fraction_of_length = i as f64 / target_num_points as f64;

            match utm_trail.line_interpolate_point(i as f64 / target_num_points as f64){
                Some(utm_interpolated_point) => {
                    let utm_coord = UtmCoord::new(utm_interpolated_point.x_y());
                    let (x, y) = utm_coord.get_x_y_coords(self.bounds.min_x, self.bounds.min_y, self.x_tick, self.y_tick);
                    self.set_with_deltas(x, y, true, &deltas)?;
                }
                None => {
                    error!("Could not interpolate point at {:.2} of trail, Skipping point", fraction_of_length)
                }
            }
        }

        Ok(())
    }

    /// sets all points inside the polygon to true
    pub fn add_filled_lat_lon_polygon(&mut self, lat_lon_region: &Polygon) -> Result<(), LasToStlError>{

        let utm_region = polygon_to_utm_polygon(lat_lon_region);

        self.add_filled_utm_polygon(&utm_region)
    }

    pub fn add_filled_utm_polygon(&mut self, utm_region: &Polygon) -> Result<(), LasToStlError>{
        // get bounding rectangle to avoid checking points that arent even close

        let utm_bounding_rectangle = utm_region.bounding_rect().ok_or(LasToStlError::NoBoundingRectError)?;
        let min_utm = UtmCoord::new(utm_bounding_rectangle.min().x_y());
        let max_utm = UtmCoord::new(utm_bounding_rectangle.max().x_y());

        trace!("min_utm: {:?}, max_utm: {:?}", min_utm, max_utm);
        trace!("self.bounds: {}", self.bounds);

        let first_coord = utm_region.exterior().coords().next().ok_or(LasToStlError::EmptyPolygonError)?;
        trace!("first 'utm' coord in exterior: {:?}", first_coord);

        let (min_x, min_y) = min_utm.get_x_y_coords(self.bounds.min_x, self.bounds.min_y, self.x_tick, self.y_tick);
        let (max_x, max_y) = max_utm.get_x_y_coords(self.bounds.min_x, self.bounds.min_y, self.x_tick, self.y_tick);

        trace!("min_x: {:?}, min_y: {:?}", min_x, min_y);
        trace!("max_x: {:?}, max_y: {:?}", max_x, max_y);

        if max_x > self.x_res || max_y > self.y_res{
            return Err(LasToStlError::PolygonOutOfBoundsError {
                x_res: self.x_res,
                y_res: self.y_res,
                x: max_x,
                y: max_y,
            })
        }

        for x in min_x..=max_x{
            for y in min_y..=max_y{
                self.data[(y*self.x_res) + x] |=
                    utm_region.contains(&Coord::from(&self.get_x_y_utm_unchecked(x, y)))
            }
            if x % 512 == 0{
                info!("region_rasterizing: {:.2}%", 100f64 * x as f64 / self.x_res as f64)
            }
        }

        Ok(())
    }

    /// expects line_string to be in lat lon, not UTM
    pub fn add_lat_lon_line_string_as_region(&mut self, line_string: &LineString) -> Result<(), LasToStlError>{
        if !line_string.is_closed(){
            return Err(LasToStlError::OpenLineStringError)
        }
        let utm_line_string: LineString = linestring_to_utm_linestring(&line_string);
        let utm_polygon = Polygon::new(utm_line_string, vec!());

        self.add_filled_utm_polygon(&utm_polygon)
    }

    /// adds a GEO point with the specified radius.
    /// If adding multiple points please use `add_waypoints` instead to avoid recalculating deltas
    /// returns an error if none of the pixels in or on the radius are within bounds of the mask.
    pub fn add_lat_lon_waypoint(&mut self, waypoint: Point, radius: u16) -> Result<(), LasToStlError>{
        let deltas: Vec<(i16, i16)> = get_point_deltas_within_radius(radius);

        let utm_coord = UtmCoord::from(&waypoint);

        let (x, y) = utm_coord.get_x_y_coords(self.bounds.min_x, self.bounds.min_y, self.x_tick, self.y_tick);
        self.set_with_deltas(x, y, true, &deltas)
    }

    /// adds a list of geo points with a specified radius
    /// returns an error if: for any of the points, none of the pixels in or on the radius are within bounds of the mask.
    pub fn add_lat_lon_waypoints(&mut self, waypoints: Vec<Point>, dot_radius: u16) -> Result<(), LasToStlError>{
        let deltas: Vec<(i16, i16)> = get_point_deltas_within_radius(dot_radius);
        for waypoint in waypoints{

            let utm_coord = UtmCoord::from(&waypoint);

            let (x, y) = utm_coord.get_x_y_coords(self.bounds.min_x, self.bounds.min_y, self.x_tick, self.y_tick);
            self.set_with_deltas(x, y, true, &deltas)?
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

    /// adds a vec of UTM points with specified radius
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
    pub fn checked_bitor_assign(&mut self, other_mask: &Mask) -> Result<(), LasToStlError> {
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
    pub fn checked_bitand_assign(&mut self, other_mask: &Mask) -> Result<(), LasToStlError> {
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
    pub fn checked_bitxor_assign(&mut self, other_mask: &Mask) -> Result<(), LasToStlError> {
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

    pub fn checked_sub_assign(&mut self, other_mask: &Mask) -> Result<(), LasToStlError> {
        if self.x_res == other_mask.x_res && self.y_res == other_mask.y_res && self.bounds == other_mask.bounds{
            for (own_state, other_state) in self.data.iter_mut().zip(other_mask.data.iter()){
                *own_state = *own_state && !*other_state;
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

    /// inverts the mask... Duh
    pub fn invert(&mut self){
        self.data.iter_mut().for_each(|p| { *p = !*p })
    }

    /// neighbors are in the order of the following relative coordinates:
    /// `[(-1isize, 1isize), (0isize, 1isize), (1isize, 1isize),
    ///   (-1isize, 0isize), (0isize, 0isize), (1isize, 0isize),
    ///   (-1isize, -1isize), (0isize, -1isize), (1isize, -1isize)]`
    pub fn get_neighbors(&self, x: usize, y: usize) -> [bool; 9]{
        const MAP: [(isize, isize); 9] = [
            (-1isize, 1isize), (0isize, 1isize), (1isize, 1isize),
            (-1isize, 0isize), (0isize, 0isize), (1isize, 0isize),
            (-1isize, -1isize), (0isize, -1isize), (1isize, -1isize)
        ];

        MAP.iter().map(|coord| {
            match self.get_by_xy_checked(x as isize + coord.0, y as isize + coord.1){
                Ok(s) => {
                    s
                }
                Err(_) => {
                    false
                }
            }
        }).collect::<Vec<bool>>().try_into().unwrap() // I don't think this can produce an error because the length must be 9.
    }

    /// gets the UTM coordinates of the specified point in pixel space
    /// without checking if it is inside the bounds of the mask.
    /// The points outside will still be marginally valid,
    /// but for most applications this indicates an error in the parameters.
    pub fn get_x_y_utm_unchecked(&self, x: usize, y: usize) -> UtmCoord{
        UtmCoord::from(((x as f64 * self.x_tick) + self.bounds.min_x, (y as f64 * self.y_tick) + self.bounds.min_y))
    }

    /// gets the UTM coordinates of the specified point in pixel space
    pub fn get_x_y_utm(&self, x: usize, y: usize) -> Result<UtmCoord, LasToStlError>{
        if x < self.x_res && y < self.y_res{
            Ok(self.get_x_y_utm_unchecked(x, y))
        } else {
            Err(LasToStlError::BadIndexError {
                x_res: self.x_res,
                y_res: self.y_res,
                x,
                y,
            })
        }
    }

    pub fn get_by_xy_unchecked(&self, x: usize, y: usize) -> bool{
        self.data[(y*self.x_res) + x]
    }

    pub fn get_mut_ref_by_xy_unchecked(&mut self, x: usize, y: usize) -> &mut bool{
        &mut self.data[(y*self.x_res) + x]
    }

    pub fn get_by_xy_checked(&self, x: isize, y: isize) -> Result<bool, LasToStlError>{
        if x < self.x_res as isize && y < self.y_res as isize && x >= 0 && y >= 0{
            Ok(self.get_by_xy_unchecked(x as usize, y as usize))
        } else {
            Err(LasToStlError::GetByXyCheckedError {
                x_res: self.x_res,
                y_res: self.y_res,
                x,
                y,
            })
        }
    }

    pub fn get_percent_coverage(&self) -> f64{
        let mut num_true: u64 = 0;
        for state in &self.data{
            if *state{
                num_true += 1;
            }
        }

        100f64 * num_true as f64 / (self.x_res * self.y_res) as f64

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

impl SubAssign for Mask{
    fn sub_assign(&mut self, rhs: Self) {
        for (own_state, other_state) in self.data.iter_mut().zip(rhs.data.iter()){
            *own_state = *own_state && !*other_state;
        }
    }
}