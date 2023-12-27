use geo::{GeometryCollection, LineString, Polygon};
use simple_logger::SimpleLogger;
use las_kml_to_stl::height_map::HeightMap;
use las_kml_to_stl::kml_utils::{get_regions, get_trails, load_kml_file};
use las_kml_to_stl::mask::Mask;


fn main() {
    // start a logger to print info from las_kml_to_stl
    SimpleLogger::new().env().init().unwrap();

    save_data();

    load_and_manipulate();
}

pub fn save_data(){

    // load LAZ/LAS data
    let hm = HeightMap::glob_get_height_map
        (
            "test_laz/*.laz", // look for files ending in '.laz' in folder 'test_laz_file_folder'
            Some(1000), // use 1000 sample points across the x axis
            None // calculate the number of sample points on the y axis to best keep the original aspect ratio
        ).unwrap();

    // save this height map.
    /*
        While you do not have to save the heightmap for this library to work, loading data takes a
        considerable amount of time and saving this progress is useful in case an error occurs in the future
        or you decide to change something.

        The saved data is specific to this library and is not a standard.
    */
    hm.save("example data.json").unwrap();
}

pub fn load_and_manipulate(){
    // load the height map that was previously saved
    let mut hm = HeightMap::load("example data.json").unwrap();

    // load a file that contains a boundary
    let kml_file_containing_property_line: GeometryCollection = load_kml_file("test_perimeters/property_line.kml").unwrap();

    // get a list (vec) of all polygons in the file.
    let all_polygons_in_file: Vec<Polygon> = get_regions(kml_file_containing_property_line);

    // in this example I assume the first polygon is the property line.
    // get a reference to the first polygon in the file
    let property_line_polygon = &all_polygons_in_file[0];

    // create a mask with the same resolution and bounds as our heightmap
    let mut property_mask: Mask = Mask::new_with_dims(hm.x_res, hm.y_res, hm.bounds);

    // add the filled in polygon to the mask.
    // The KML file must be in decimal GPS coordinates. I have never seen a KML in a different format,
    // but if you want to make sure, open the KML with a text editor and check the coordinates.
    // If they look like what you would expect, they are probably ok
    property_mask.add_filled_lat_lon_polygon(property_line_polygon).unwrap();

    // load a file that contains some trails (LineStrings in KML speak)
    let kml_file_with_trails: GeometryCollection = load_kml_file("test_perimeters/trails.kml").unwrap();

    // get a list (vec) of all trails in the file.
    let all_trails_in_file: Vec<LineString> = get_trails(kml_file_with_trails);

    // create another mask for the trails
    let mut trail_mask: Mask = Mask::new_with_dims(hm.x_res, hm.y_res, hm.bounds);

    let trail_width_in_pixels: u16 = 16;

    for trail in all_trails_in_file{
        // for each trail, add it to the mask with sample points every ~ `trail_width_in_pixels / 4` pixels
        trail_mask.add_lat_lon_trail_auto_sample(&trail, trail_width_in_pixels / 2 /* divide by two because this function is asking for a radius*/)
    }

    // subtract 10 units from the height where trail_mask is true (lower the elevation of the trails by 10 units)
    hm.offset_by_mask(&trail_mask, -10.0).unwrap();

    // save the height map as an stl file named my property.stl, only saving data on the property,
    // with height values exaggerated (multiplied by 2) and a base thickness of 10 units.
    // The base thickness just adds that many units to every height value so the printed part is a little stronger.
    hm.save_as_stl_masked("my property.stl", &property_mask, 2.0, 10.0).unwrap();
}