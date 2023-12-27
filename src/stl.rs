use std::fs::OpenOptions;
use std::time::SystemTime;
use log::{debug, error, info};
use stl_io::{Normal, Triangle, Vector, Vertex};
use crate::errors::LasToStlError;
use crate::height_map::HeightMap;
use crate::mask::Mask;

use crate::utils::{normal_or_default, normal_pos_or_default, x_y_to_index};

impl HeightMap {
    pub fn save_as_stl(&self, path: &str, z_scaling: f64, base_thickness: f32) -> Result<(), LasToStlError>{

        println!("saving as stl");

        let now = SystemTime::now();

        let z_scale_factor = z_scaling * self.x_res as f64 / self.bounds.x_range();

        let data_length = self.x_res * self.y_res;

        info!("assembling vertex lists");

        let top_vertex_list: Vec<Vertex> = self.data.iter().enumerate().map(|(index, height)| {
            let x = index % self.x_res;
            let y = index / self.x_res;
            Vertex::new([x as f32, y as f32, (normal_pos_or_default(height - self.bounds.min_z, 0f64) * z_scale_factor) as f32 + base_thickness])
        }).collect();

        let bottom_vertex_list: Vec<Vertex> = self.data.iter().enumerate().map(|(index, _height)| {
            let x = index % self.x_res;
            let y = index / self.x_res;
            Vertex::new([x as f32, y as f32, 0f32])
        }).collect();

        info!("assembled vertex lists");

        let total_triangles = (4 * data_length) + (4 * self.x_res) + (4 * self.y_res);

        let mut triangle_list: Vec<Triangle> = Vec::with_capacity(total_triangles);

        for x in 0..self.x_res-1{
            for y in 0..self.y_res-1{
                triangle_list.extend(vertex_rec_to_triangles_diagonal(
                    top_vertex_list[x_y_to_index(self.x_res, self.y_res, x, y)?],
                    top_vertex_list[x_y_to_index(self.x_res, self.y_res, x, y+1)?],
                    top_vertex_list[x_y_to_index(self.x_res, self.y_res, x+1, y+1)?],
                    top_vertex_list[x_y_to_index(self.x_res, self.y_res, x+1, y)?],
                    Normal::from(Vector::new([0f32, 0f32, 1f32]))
                ));

                triangle_list.extend(vertex_rec_to_triangles_diagonal(
                    bottom_vertex_list[x_y_to_index(self.x_res, self.y_res, x+1, y)?],
                    bottom_vertex_list[x_y_to_index(self.x_res, self.y_res, x+1, y+1)?],
                    bottom_vertex_list[x_y_to_index(self.x_res, self.y_res, x, y+1)?],
                    bottom_vertex_list[x_y_to_index(self.x_res, self.y_res, x, y)?],
                    Normal::from(Vector::new([0f32, 0f32, -1f32]))
                ));
            }
        }

        info!("assembled top and bottom faces");

        // north?
        for x in 0..self.x_res-1{
            triangle_list.extend(vertex_rec_to_triangles_diagonal(
                top_vertex_list[x_y_to_index(self.x_res, self.y_res, x+1, self.y_res-1)?],
                top_vertex_list[x_y_to_index(self.x_res, self.y_res, x, self.y_res-1)?],
                bottom_vertex_list[x_y_to_index(self.x_res, self.y_res, x, self.y_res-1)?],
                bottom_vertex_list[x_y_to_index(self.x_res, self.y_res, x+1, self.y_res-1)?],
                Normal::from(Vector::new([0f32, 1f32, 0f32]))
            ))
        }

        info!("assembled north faces");

        // south?
        for x in 0..self.x_res-1{
            triangle_list.extend(vertex_rec_to_triangles_diagonal(
                top_vertex_list[x_y_to_index(self.x_res, self.y_res, x, 0)?],
                top_vertex_list[x_y_to_index(self.x_res, self.y_res, x+1, 0)?],
                bottom_vertex_list[x_y_to_index(self.x_res, self.y_res, x+1, 0)?],
                bottom_vertex_list[x_y_to_index(self.x_res, self.y_res, x, 0)?],
                Normal::from(Vector::new([0f32, -1f32, 0f32]))
            ))
        }

        info!("assembled south faces");

        // east?
        for y in 0..self.y_res-1{
            triangle_list.extend(vertex_rec_to_triangles_diagonal(
                top_vertex_list[x_y_to_index(self.x_res, self.y_res, self.x_res-1, y+1)?],
                top_vertex_list[x_y_to_index(self.x_res, self.y_res, self.x_res-1, y)?],
                bottom_vertex_list[x_y_to_index(self.x_res, self.y_res, self.x_res-1, y)?],
                bottom_vertex_list[x_y_to_index(self.x_res, self.y_res, self.x_res-1, y+1)?],
                Normal::from(Vector::new([1f32, 0f32, 0f32]))
            ))
        }

        info!("assembled east faces");

        // west?
        for y in 0..self.y_res-1{
            triangle_list.extend(vertex_rec_to_triangles_diagonal(
                top_vertex_list[x_y_to_index(self.x_res, self.y_res, 0, y)?],
                top_vertex_list[x_y_to_index(self.x_res, self.y_res, 0, y+1)?],
                bottom_vertex_list[x_y_to_index(self.x_res, self.y_res, 0, y+1)?],
                bottom_vertex_list[x_y_to_index(self.x_res, self.y_res, 0, y)?],
                Normal::from(Vector::new([-1f32, 0f32, 0f32]))
            ))
        }

        info!("assembled west faces");

        let mut file = OpenOptions::new().write(true).create_new(true).open(path)?; // .create_new(true)
        stl_io::write_stl(&mut file, triangle_list.iter())?;

        debug!("saved as stl. took {:?}", now.elapsed());

        Ok(())
    }

    pub fn save_as_stl_masked(&self, path: &str, mask: &Mask, z_scaling: f64, base_thickness: f32) -> Result<(), LasToStlError>{

        debug!("save as stl masked");

        let now = SystemTime::now();

        let z_scale_factor = z_scaling * self.x_res as f64 / self.bounds.x_range();

        let top_vertex_list: Vec<Option<Vertex>> = self.data.iter().enumerate().map(|(index, height)| {
            match mask.data[index]{
                false => {
                    None
                }
                true => {
                    let x = index % self.x_res;
                    let y = index / self.x_res;
                    Some(Vertex::new([x as f32, y as f32, (normal_pos_or_default(height - self.bounds.min_z, 0f64) * z_scale_factor) as f32 + base_thickness]))
                }
            }

        }).collect::<Vec<Option<Vertex>>>();

        let bottom_vertex_list: Vec<Option<Vertex>> = self.data.iter().enumerate().map(|(index, _height)| {
            match mask.data[index]{
                false => {
                    None
                }
                true => {
                    let x = index % self.x_res;
                    let y = index / self.x_res;
                    Some(Vertex::new([x as f32, y as f32, 0f32]))
                }
            }

        }).collect::<Vec<Option<Vertex>>>();

        info!("assembled vertex lists");

        let mut triangle_list: Vec<Triangle> = Vec::new();

        for x in 0..self.x_res-1{
            for y in 0..self.y_res-1{

                let top_vertices = option_vertex_rec_to_triangles_diagonal(
                    top_vertex_list[x_y_to_index(self.x_res, self.y_res, x, y)?],
                    top_vertex_list[x_y_to_index(self.x_res, self.y_res, x, y+1)?],
                    top_vertex_list[x_y_to_index(self.x_res, self.y_res, x+1, y+1)?],
                    top_vertex_list[x_y_to_index(self.x_res, self.y_res, x+1, y)?],
                    Normal::from(Vector::new([0f32, 0f32, 1f32]))
                );

                if top_vertices.is_some(){
                    triangle_list.extend(top_vertices.unwrap() /*safe unwrap*/);
                }

                let bottom_vertices = option_vertex_rec_to_triangles_diagonal(
                    bottom_vertex_list[x_y_to_index(self.x_res, self.y_res, x+1, y)?],
                    bottom_vertex_list[x_y_to_index(self.x_res, self.y_res, x+1, y+1)?],
                    bottom_vertex_list[x_y_to_index(self.x_res, self.y_res, x, y+1)?],
                    bottom_vertex_list[x_y_to_index(self.x_res, self.y_res, x, y)?],
                    Normal::from(Vector::new([0f32, 0f32, -1f32]))
                );

                if bottom_vertices.is_some(){
                    triangle_list.extend(bottom_vertices.unwrap() /*safe unwrap*/);
                }
            }
        }

        info!("assembled top and bottom faces");

        let stl_helper_mask = StlHelperMask::from(mask);

        let x_pos_edges = stl_helper_mask.get_cardinal_edge(true, true);
        info!("calculated east edge faces");
        let x_neg_edges = stl_helper_mask.get_cardinal_edge(true, false);
        info!("calculated west edge faces");
        let y_pos_edges = stl_helper_mask.get_cardinal_edge(false, true);
        info!("calculated north edge faces");
        let y_neg_edges = stl_helper_mask.get_cardinal_edge(false, false);
        info!("calculated south edge faces");

        for edge_coord in x_pos_edges{

            match option_vertex_rec_to_triangles_diagonal(
                top_vertex_list[x_y_to_index(self.x_res, self.y_res, edge_coord.0 + 1, (edge_coord.1)+1)?],
                top_vertex_list[x_y_to_index(self.x_res, self.y_res, edge_coord.0 + 1, edge_coord.1)?],
                bottom_vertex_list[x_y_to_index(self.x_res, self.y_res, edge_coord.0 + 1, edge_coord.1)?],
                bottom_vertex_list[x_y_to_index(self.x_res, self.y_res, edge_coord.0 + 1, (edge_coord.1)+1)?],
                Normal::from(Vector::new([1f32, 0f32, 0f32]))
            ){
                Some(faces) => {
                    triangle_list.extend(faces);
                }
                None => {
                    error!("Attempted to build a face from a vertex that doesn't exist. Skipping");
                    debug!("edge coord: {:?}", edge_coord);
                }
            }
        }

        info!("assembled east edge faces");

        for edge_coord in x_neg_edges{
            match option_vertex_rec_to_triangles_diagonal(
                top_vertex_list[x_y_to_index(self.x_res, self.y_res, edge_coord.0, edge_coord.1)?],
                top_vertex_list[x_y_to_index(self.x_res, self.y_res, edge_coord.0, edge_coord.1 + 1)?],
                bottom_vertex_list[x_y_to_index(self.x_res, self.y_res, edge_coord.0, edge_coord.1 + 1)?],
                bottom_vertex_list[x_y_to_index(self.x_res, self.y_res, edge_coord.0, edge_coord.1)?],
                Normal::from(Vector::new([-1f32, 0f32, 0f32]))
            ){
                Some(faces) => {
                    triangle_list.extend(faces);
                }
                None => {
                    error!("Attempted to build a face from a vertex that doesn't exist. Skipping");
                    debug!("edge coord: {:?}", edge_coord);
                }
            }
        }

        info!("assembled west edge faces");

        for edge_coord in y_pos_edges{
            match option_vertex_rec_to_triangles_diagonal(
                top_vertex_list[x_y_to_index(self.x_res, self.y_res, edge_coord.0 + 1, edge_coord.1 + 1)?],
                top_vertex_list[x_y_to_index(self.x_res, self.y_res, edge_coord.0, edge_coord.1 + 1)?],
                bottom_vertex_list[x_y_to_index(self.x_res, self.y_res, edge_coord.0, edge_coord.1 + 1)?],
                bottom_vertex_list[x_y_to_index(self.x_res, self.y_res, edge_coord.0 + 1, edge_coord.1 + 1)?],
                Normal::from(Vector::new([0f32, 1f32, 0f32]))
            ){
                Some(faces) => {
                    triangle_list.extend(faces);
                }
                None => {
                    error!("Attempted to build a face from a vertex that doesn't exist. Skipping");
                    debug!("edge coord: {:?}", edge_coord);
                }
            }
        }

        info!("assembled north edge faces");

        for edge_coord in y_neg_edges{

            match option_vertex_rec_to_triangles_diagonal(
                top_vertex_list[x_y_to_index(self.x_res, self.y_res, edge_coord.0, edge_coord.1)?],
                top_vertex_list[x_y_to_index(self.x_res, self.y_res, edge_coord.0 + 1, edge_coord.1)?],
                bottom_vertex_list[x_y_to_index(self.x_res, self.y_res, edge_coord.0 + 1, edge_coord.1)?],
                bottom_vertex_list[x_y_to_index(self.x_res, self.y_res, edge_coord.0, edge_coord.1)?],
                Normal::from(Vector::new([0f32, -1f32, 0f32]))
            ){
                Some(faces) => {
                    triangle_list.extend(faces);
                }
                None => {
                    error!("Attempted to build a face from a vertex that doesn't exist. Skipping");
                    debug!("edge coord: {:?}", edge_coord);
                }
            }
        }

        info!("assembled south edge faces");

        let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
        stl_io::write_stl(&mut file, triangle_list.iter())?;

        debug!("save as stl masked top and bottom done in {:?}", now.elapsed());

        Ok(())
    }
}

/// will preserve order, so if you want them to be clockwise, pass them clockwise and vice versa
pub fn vertex_rec_to_triangles_diagonal(vertex_1: Vertex, vertex_2: Vertex, vertex_3: Vertex, vertex_4: Vertex, normal: Normal) -> [Triangle; 2]{
    [Triangle{
        normal,
        vertices: [
            vertex_1,
            vertex_2,
            vertex_4
        ]
    }, Triangle{
        normal,
        vertices: [
            vertex_2,
            vertex_3,
            vertex_4
        ]
    }]
}

pub fn option_vertex_rec_to_triangles_diagonal(
    vertex_1: Option<Vertex>,
    vertex_2: Option<Vertex>,
    vertex_3: Option<Vertex>,
    vertex_4: Option<Vertex>,
    normal: Normal) -> Option<[Triangle; 2]>
{
    Some([Triangle{
        normal,
        vertices: [
            vertex_1?,
            vertex_2?,
            vertex_4?
        ]
    }, Triangle{
        normal,
        vertices: [
            vertex_2?,
            vertex_3?,
            vertex_4?
        ]
    }])
}

pub struct StlHelperMask {
    data: Vec<bool>,
    x_res: usize,
    y_res: usize,
}

impl From<&Mask> for StlHelperMask{
    fn from(mask: &Mask) -> StlHelperMask {

        let new_mask_x_res: usize = mask.x_res-1;
        let new_mask_y_res: usize = mask.y_res-1;

        let mut new_data_vec: Vec<bool> = vec!(false; (new_mask_x_res) * (new_mask_y_res));
        for x in 0..new_mask_x_res{
            for y in 0..new_mask_y_res{
                let neighbors = mask.get_neighbors(x, y);
                new_data_vec[y*new_mask_x_res + x] = neighbors[1] && neighbors[2] && neighbors[4] && neighbors[5]
                // neighbors are in the order of the following relative coordinates:
                // `[(-1isize, 1isize), (0isize, 1isize), (1isize, 1isize),
                //   (-1isize, 0isize), (0isize, 0isize), (1isize, 0isize),
                //   (-1isize, -1isize), (0isize, -1isize), (1isize, -1isize)]`
            }
        }
        StlHelperMask{
            data: new_data_vec,
            x_res: new_mask_x_res,
            y_res: new_mask_y_res,
        }
    }
}
impl StlHelperMask{

    /// gets a list of coordinates that are true but have a false neighbor in the specified direction.
    /// Out of bounds points are considered to be false
    pub fn get_cardinal_edge(&self, use_x_axis: bool, check_positive_edge: bool) -> Vec<(usize, usize)>{

        let mut out_vec: Vec<(usize, usize)> = Vec::new();

        let (x_offset, y_offset) = if use_x_axis
        {
            (if check_positive_edge {
                1isize
            } else {
                -1isize
            }, 0)
        } else {
            (0, if check_positive_edge {
                1isize
            } else {
                -1isize
            })
        };

        for x in 0..self.x_res{
            for y in 0..self.y_res{
                if self.get_by_xy_unchecked(x, y) && !match self.get_by_xy_checked(x as isize + x_offset, y as isize + y_offset){
                    Ok(s) => {
                        s
                    }
                    Err(_) => {
                        false
                    }
                }{
                    out_vec.push((x, y))
                }
            }
        }
        out_vec
    }

    pub fn get_by_xy_unchecked(&self, x: usize, y: usize) -> bool{
        self.data[(y*self.x_res) + x]
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
}
