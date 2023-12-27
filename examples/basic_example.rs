use simple_logger::SimpleLogger;
use las_kml_to_stl::height_map::HeightMap;


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
    let hm = HeightMap::load("example data.json").unwrap();

    // save the height map as an stl
    hm.save_as_stl("stl file out.stl", 2.0, 10.0).unwrap();

}