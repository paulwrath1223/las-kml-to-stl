use geo::{Coord, Point};
use utm::{to_utm_wgs84_no_zone, wsg84_utm_to_lat_lon};




pub struct UtmCoord {
    pub northing: f64,
    pub easting: f64,
}


impl UtmCoord {
    pub fn get_x_y_coords(&self, x_offset: f64, y_offset: f64, x_tick: f64, y_tick: f64) -> (usize, usize){
        (((self.easting - x_offset) / x_tick) as usize, ((self.northing - y_offset) / y_tick) as usize)
    }
    
    pub fn new(coords: (f64, f64)) -> Self{
        UtmCoord{
            northing: coords.1,
            easting: coords.0,
        }
    }
}

impl From<&UtmCoord> for (f64, f64) {
    fn from(utm_coord: &UtmCoord) -> Self {
        (utm_coord.easting, utm_coord.northing)
    }
}

impl From<&Point<f64>> for UtmCoord{
    /// converts from a LAT LON point to a utm_coord
    fn from(gps_point: &Point<f64>) -> Self {
        UtmCoord::from(&gps_point.0)
    }
}

impl From<&Coord<f64>> for UtmCoord{
    
    /// converts from a LAT LON coord to a utm_coord
    fn from(gps_point: &Coord<f64>) -> Self {
        let (northing, easting, _) = to_utm_wgs84_no_zone(gps_point.y, gps_point.x);
        UtmCoord {
            northing,
            easting,
        }
    }
}

impl From<&UtmCoord> for Coord<f64>{
    /// creates a Coord with UTM units !!! Most Coords in this library use lat lon,
    /// so please do not mix them
    fn from(utm_point: &UtmCoord) -> Coord<f64> {
        Coord::from((utm_point.easting, utm_point.northing))
    }
}

impl From<(f64, f64)> for UtmCoord{

    /// (x, y)
    fn from(value: (f64, f64)) -> Self {
        UtmCoord{
            northing: value.1,
            easting: value.0,
        }
    }
}