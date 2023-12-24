use geo::{Coord, Point};
use utm::{to_utm_wgs84_no_zone};




pub struct UtmCoord {
    pub northing: f64,
    pub easting: f64,
}


impl UtmCoord {
    pub fn get_x_y_coords(&self, x_offset: f64, y_offset: f64, x_tick: f64, y_tick: f64) -> (usize, usize){
        (((self.easting - x_offset) / x_tick) as usize, ((self.northing - y_offset) / y_tick) as usize)
    }
}

impl From<&UtmCoord> for (f64, f64) {
    fn from(utm_coord: &UtmCoord) -> Self {
        (utm_coord.easting, utm_coord.northing)
    }
}

impl From<&Point<f64>> for UtmCoord{
    fn from(gps_point: &Point<f64>) -> Self {
        UtmCoord::from(&gps_point.0)
    }
}

impl From<&Coord<f64>> for UtmCoord{
    fn from(gps_point: &Coord<f64>) -> Self {
        let (northing, easting, _) = to_utm_wgs84_no_zone(gps_point.y, gps_point.x);
        UtmCoord {
            northing,
            easting,
        }
    }
}