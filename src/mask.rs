use std::ops::BitOrAssign;

use crate::coords::{RasterizedPolygon, UtmCoord, UtmPolygon, UtmTrail};
use crate::kml::KmlRegion;
use crate::utils::get_point_deltas_within_radius;
use crate::utm_bounds::UtmBoundingBox;

pub struct Mask{
    pub data: Vec<bool>,
    pub x_res: usize,
    pub y_res: usize,
    x_tick: f64,
    y_tick: f64,
    pub bounds: UtmBoundingBox
}

impl Mask{

    pub fn set_with_deltas(&mut self, x: usize, y: usize, state: bool, deltas: &Vec<(i16, i16)>){

        for (delta_x, delta_y) in deltas{
            let new_x: usize = (x as i64 + *delta_x as i64) as usize;
            let new_y: usize = (y as i64 + *delta_y as i64) as usize;

            self.set_x_y(new_x, self.y_res - new_y - 1, state)
        }
    }

    pub fn set_x_y(&mut self, x: usize, y: usize, new_state: bool){
        self.data[(y*self.x_res) + x] = new_state
    }

    pub fn new_with_dims(x_res: usize, y_res: usize, bounds: Bounds3D) -> Mask{

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

    pub fn add_kml_region(&mut self, kml_region: KmlRegion, width: u16){
        todo!("may our god that funny little crab smite the creator of the kml library im using and am too stupid to understand");
        let mut region_as_trail: UtmTrail = UtmTrail(kml_region.ordered_corners);
        region_as_trail.interpolate_until_distance_threshold((width/2) as f64);
        self.add_utm_trail_dotted(region_as_trail, width);
    }

    pub fn add_utm_trail_dotted(&mut self, utm_trail: UtmTrail, dot_radius: u16){
        let deltas: Vec<(i16, i16)> = get_point_deltas_within_radius(dot_radius);
        for point in utm_trail{
            let (x, y) = point.get_x_y_coords(self.bounds.min_x, self.bounds.min_y, self.x_tick, self.y_tick);
            self.set_with_deltas(x, y, true, &deltas);
        }
    }

    pub fn add_utm_region(&mut self, utm_polygon: &UtmPolygon, invert: bool){
        let rasterized_region = RasterizedPolygon::from_poly_and_bounds(utm_polygon, self.bounds, self.x_tick, self.y_tick);

        println!("rasterized_region: {:?}", rasterized_region.0);

        for x in 0..self.x_res{
            for y in 0..self.y_res{
                self.set_x_y(x, y, rasterized_region.includes((x as f64, y as f64), invert))
            }
        }
    }

    pub fn add_utm_wps(&mut self, utm_coords: Vec<UtmCoord>, dot_radius: u16){
        let deltas: Vec<(i16, i16)> = get_point_deltas_within_radius(dot_radius);
        for utm_coord in utm_coords{
            let (x, y) = utm_coord.get_x_y_coords(self.bounds.min_x, self.bounds.min_y, self.x_tick, self.y_tick);
            self.set_with_deltas(x, y, true, &deltas)
        }
    }

    pub fn add_utm_trails_dotted(&mut self, utm_trails: Vec<UtmTrail>, dot_radius: u16){
        for utm_trail in utm_trails{
            self.add_utm_trail_dotted(utm_trail, dot_radius)
        }
    }

}

impl BitOrAssign for Mask{
    fn bitor_assign(&mut self, other_mask: Mask) {

        assert_eq!(self.x_res, other_mask.x_res, "Masks have different resolution :((");
        assert_eq!(self.y_res, other_mask.y_res, "Masks have different resolution :((");
        assert_eq!(self.bounds, other_mask.bounds, "Masks have different bounds :((");

        for (own_state, other_state) in self.data.iter_mut().zip(other_mask.data.iter()){
            *own_state |= *other_state;
        }
    }
}