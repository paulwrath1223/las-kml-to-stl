use geo::{Coord, Point};
use utm::to_utm_wgs84;




#[derive(Debug)]
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

    /// see [UTM on wikipedia](https://en.wikipedia.org/wiki/Universal_Transverse_Mercator_coordinate_system) to find what a UTM zone is.
    /// This is required and must be correct (or at least constant)
    pub fn from_gps_coord_zoned(gps_point: &Coord<f64>, utm_zone: u8) -> Self {
        let (northing, easting, _) = to_utm_wgs84(gps_point.y, gps_point.x, utm_zone);
        UtmCoord {
            northing,
            easting,
        }
    }

    /// converts from a LAT LON point to a utm_coord
    pub fn from_lat_lon_point_zoned(gps_point: &Point<f64>, utm_zone: u8) -> Self {
        UtmCoord::from_gps_coord_zoned(&gps_point.0, utm_zone)
    }
}


impl From<&UtmCoord> for (f64, f64) {
    fn from(utm_coord: &UtmCoord) -> Self {
        (utm_coord.easting, utm_coord.northing)
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