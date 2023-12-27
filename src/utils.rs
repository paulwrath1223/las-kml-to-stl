use std::num::FpCategory;
use std::path::PathBuf;
use glob::glob;
use crate::errors::LasToStlError;
use log::warn;

/// maps `value_float` from a range between `min_float` and `max_float` to a u64 in range `0` to `max_val`
pub fn scale_float_to_uint_range(value_float: &f64, min_float: f64, max_float: f64, max_val: u64) -> u64{
    let range = max_float - min_float;
    (((value_float-min_float) / range) * max_val as f64) as u64
}

/// returns the smaller f64, defaulting to `a` when a == b
pub fn f64_min(a: f64, b: f64) -> f64{
    if a > b {
        b
    } else {
        a
    }
}

/// returns the bigger f64, defaulting to `a` when a == b
pub fn f64_max(a: f64, b: f64) -> f64{
    if a < b {
        b
    } else {
        a
    }
}

/// basically a wrapper for `glob(glob_pattern)` with error conversions
pub fn get_paths(glob_pattern: &str) -> Result<Vec<PathBuf>, LasToStlError>{

    let mut path_vec: Vec<PathBuf> = Vec::new();

    for entry in glob(glob_pattern)? {
        match entry {
            Ok(path) => {
                path_vec.push(path)
            },
            Err(e) => warn!("file was not able to be read, skipping it.: {:?} ", e),
        }
    }

    if path_vec.is_empty(){
        Err(LasToStlError::NoValidGlobReturnsError(glob_pattern.to_string()))
    } else {
        Ok(path_vec)
    }
}

/// 'normalizes' a float to be a real number. if `float` is a normal float it returns `float`, otherwise it returns `default`
///
/// See rust docs for float categories https://doc.rust-lang.org/nightly/core/num/enum.FpCategory.html
pub fn normal_or_default<F>(float: F, default: F) -> F
    where F: num::Float
{
    match float.classify(){
        FpCategory::Normal => {
            float
        }
        _ => {
            default
        }
    }
}

/// 'normalizes' a float to be a real positive number. if `float` is a normal float it returns `float`, otherwise it returns `default`
///
/// See rust docs for float categories https://doc.rust-lang.org/nightly/core/num/enum.FpCategory.html
pub fn normal_pos_or_default<F>(float: F, default: F) -> F
    where F: num::Float
{
    match float.classify(){
        FpCategory::Normal => {
            match float.is_sign_positive(){
                true => {
                    float
                }
                false => {
                    default
                }
            }
        }
        _ => {
            default
        }
    }
}

/// Reconsider what you're doing if you are going this deep into this library. 
/// This function takes an x and y coordinate and returns the index that point would appear in a 
/// list according to the scheme ive been using. 
/// I don't know what said scheme is called and I don't expect anyone to need to know.
/// 
/// Returns an error if the requested points are out of bounds.
pub fn x_y_to_index(x_res: usize, y_res: usize, x: usize, y: usize) -> Result<usize, LasToStlError>{
    if x < x_res && y < y_res{
        Ok(y * x_res + x)
    } else {
        Err(LasToStlError::BadIndexError {
            x_res,
            y_res,
            x,
            y,
        })
    }

}


/// taking a radius for a circle you can imagine to be at 0,0 it will return a list of integer points inside (including on) that circle.
/// if the circle you want to create is not at (0,0), add the circle's center to the coordinates this function returns
/// 
/// This function can be greatly optimized, but shouldn't be prohibitively slow.
pub fn get_point_deltas_within_radius(radius: u16) -> Vec<(i16, i16)>{

    //TODO: optimize to only check one quadrant

    let diameter: u16 = (2*radius) + 1;
    let signed_radius: i16 = radius as i16;

    let radius_squared_plus_one: i16 = signed_radius.pow(2) + 1;

    let mut point_deltas: Vec<(i16, i16)> = Vec::with_capacity(diameter.pow(2) as usize);
    for x in -signed_radius..=signed_radius{
        for y in -signed_radius..=signed_radius{
            if x.pow(2) + y.pow(2) < radius_squared_plus_one{
                point_deltas.push((x, y))
            }
        }
    }
    point_deltas
}

pub fn utm_point_to_pixel_space(x: f64, y: f64, x_offset: f64, y_offset: f64, x_tick: f64, y_tick: f64) -> (usize, usize){
    (((x - x_offset) / x_tick) as usize, ((y - y_offset) / y_tick) as usize)
}