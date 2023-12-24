
use thiserror::Error;
use crate::utm_bounds::UtmBoundingBox;

#[derive(Error, Debug)]
pub enum LasToStlError {
    #[error("IO error:\n\t{0}")]
    IoError(#[from] std::io::Error),
    #[error("Error in glob library:\n\t{0}")]
    GlobError(#[from] glob::GlobError),
    #[error("No files matching pattern {0} could be found or none of the found files could be read.\
        Files that could not be read are logged as errors.")]
    NoValidGlobReturnsError(String),
    #[error("Error deciphering glob pattern:\n\t{0}")]
    PatternError(#[from] glob::PatternError),
    #[error("Error in serde-json:\n\t{0}")]
    SerdeError(#[from] serde_json::Error),
    #[error("Error in LAS parsing Library:\n\t{0}")]
    LasError(#[from] las::Error),
    #[error("Error saving data as csv:\n\t{0}")]
    CsvError(#[from] csv::Error),
    #[error("Error saving image to file:\n\t{0}")]
    ImageError(#[from] image::ImageError),
    #[error("attempted to access the first element of a UTM trail, but it is not present.
        This could either be because an empty GPX file was provided,
        or a different error that I have to deal with")]
    EmptyTrailError,
    #[error("Bounding coordinates must be in the correct format.
        Ensure that the NW corner is further north and west than the SE corner")]
    BoundingError,
    #[error("attempted to get the internal 1d index for a coordinate pair that does not exist. \
        Note that because index starts at 0, \
        x and y must be LESS than their corresponding resolutions (not equal) \
        call variables: x_res: {x_res}, y_res: {y_res}, x: {x}, y: {y}")]
    BadIndexError{ x_res: usize, y_res: usize, x: usize, y: usize },

    #[error("`glob_get_height_map` called with resolution_x = None and resolution_y = None. \
        While one resolution can be left as none to preserve aspect ratio, one must be set. \
        See documentation for `glob_get_height_map`.")]
    NoResolutionError,

    #[error("`ImageBuffer::from_vec` returned None. Idk what this means or why or how. \
        Talk to Image: (https://docs.rs/image/0.24.7/).")]
    ImageNoneError,

    #[error("Attempted to apply a mask to a heightmap of different resolution / bounds.\
        heightmap_x_res: {heightmap_x_res}, 
        heightmap_y_res: {heightmap_y_res}, 
        mask_x_res: {mask_x_res}, 
        mask_y_res: {mask_y_res}, 
        heightmap_bounds: {heightmap_bounds}, 
        mask_bounds: {mask_bounds}")]
    MaskBoundMismatchError{ 
        heightmap_x_res: usize, 
        heightmap_y_res: usize, 
        mask_x_res: usize, 
        mask_y_res: usize, 
        heightmap_bounds: UtmBoundingBox, 
        mask_bounds: UtmBoundingBox
    },

}