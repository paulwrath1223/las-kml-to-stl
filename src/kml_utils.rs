use std::fmt::Debug;
use std::path::Path;
use geo::{Coord, Geometry, GeometryCollection, LineString, Point, Polygon};
use kml::{Kml, KmlReader, quick_collection};
use log::error;
use crate::errors::LasToStlError;
use crate::utm_point::UtmCoord;

/// basically a wrapper for some functions from the kml library
/// given a path to a kml file, it returns a collection of geometry stuff
pub fn load_kml_file<P: AsRef<Path>>(path: P) -> Result<GeometryCollection<f64>, LasToStlError>{
    let mut kml_reader = KmlReader::<_, f64>::from_path(path)?;

    let kml_data: Kml<f64> = kml_reader.read()?;

    Ok(quick_collection(kml_data)?)
}

/// loads a file for each path.
///
/// # Error handling:
/// Any files that are unable to be read will log the error with [log::error](https://docs.rs/log/0.4.20/log/enum.Level.html#variant.Error),
/// but as long as at least one geometry is successfully loaded, this function will not return an error.
pub fn load_kml_files<P: AsRef<Path> + Debug>(paths: Vec<P>)
    -> Result<GeometryCollection<f64>, LasToStlError>
{
    let mut out_vec: Vec<Geometry<f64>> = Vec::new();

    for path in paths{
        match load_kml_file(&path){
            Ok(mut gc) => {
                out_vec.append(&mut gc.0)
            }
            Err(e) => {
                error!("error loading file {:?}:\n\t{:?}\nSkipping file.", path, e)
            }
        }
    }

    if out_vec.is_empty(){
        Err(LasToStlError::NoValidGeometriesError)
    } else {
        Ok(GeometryCollection::<f64>::new_from(out_vec))
    }
}

/// recursively gets all polygons in the collection. Vec may be empty
pub fn get_regions(geometry_collection: GeometryCollection<f64>) -> Vec<Polygon>{

    let mut out_vec: Vec<Polygon> = Vec::new();

    for geometry in geometry_collection{
        match geometry{
            Geometry::Polygon(poly) => {
                out_vec.push(poly);
            }
            Geometry::GeometryCollection(gc) => {
                out_vec.extend(get_regions(gc));
            }
            _ => {}
        }
    }

    out_vec
}

/// recursively gets all line strings in the collection. Vec may be empty
pub fn get_trails(geometry_collection: GeometryCollection<f64>) -> Vec<LineString>{

    let mut out_vec: Vec<LineString> = Vec::new();

    for geometry in geometry_collection{
        match geometry{
            Geometry::LineString(ls) => {
                out_vec.push(ls);
            }
            Geometry::GeometryCollection(gc) => {
                out_vec.extend(get_trails(gc));
            }
            _ => {}
        }
    }

    out_vec
}

/// recursively gets all points in the collection. Vec may be empty
pub fn get_waypoints(geometry_collection: GeometryCollection<f64>) -> Vec<Point>{

    let mut out_vec: Vec<Point> = Vec::new();

    for geometry in geometry_collection{
        match geometry{
            Geometry::Point(pt) => {
                out_vec.push(pt);
            }
            Geometry::GeometryCollection(gc) => {
                out_vec.extend(get_waypoints(gc));
            }
            _ => {}
        }
    }

    out_vec
}

pub fn linestring_to_utm_linestring(line_string: &LineString) -> LineString{
    line_string.into_iter().map(|coord|{
        Coord::from(&UtmCoord::from(coord))
    }).collect::<LineString>()
}

pub fn polygon_to_utm_polygon(polygon: &Polygon) -> Polygon{
    Polygon::new(
        linestring_to_utm_linestring(polygon.exterior()),
        polygon.interiors().iter().map(|line_string|{
            linestring_to_utm_linestring(line_string)
        }).collect()
    )
}