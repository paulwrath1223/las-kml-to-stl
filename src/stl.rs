use std::fs::OpenOptions;
use std::time::SystemTime;
use stl_io::{Normal, Triangle, Vector, Vertex};
use crate::errors::LasToStlError;
use crate::height_map::HeightMap;
use crate::mask::Mask;

use crate::utils::{normal_or_default, x_y_to_index};

impl HeightMap {
    pub fn save_as_stl(&self, path: &str, z_scaling: f64, base_thickness: f32) -> Result<(), LasToStlError>{

        println!("saving as stl");

        let now = SystemTime::now();

        let z_scale_factor = z_scaling * self.x_res as f64 / self.bounds.x_range();

        let data_length = self.x_res * self.y_res;

        let top_vertex_list: Vec<Vertex> = self.data.iter().enumerate().map(|(index, height)| {
            let x = index % self.x_res;
            let y = index / self.x_res;
            Vertex::new([x as f32, y as f32, (normal_or_default(height - self.bounds.min_z, 0f64) * z_scale_factor) as f32 + base_thickness])
        }).collect();

        let bottom_vertex_list: Vec<Vertex> = self.data.iter().enumerate().map(|(index, _height)| {
            let x = index % self.x_res;
            let y = index / self.x_res;
            Vertex::new([x as f32, y as f32, 0f32])
        }).collect();

        let total_triangles = (2 * data_length) + (4 * self.x_res) + (4 * self.y_res);

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

        let mut file = OpenOptions::new().write(true).create(true).open(path)?; // .create_new(true)
        stl_io::write_stl(&mut file, triangle_list.iter())?;

        println!("saved as stl. took {:?}", now.elapsed());

        Ok(())
    }

    pub fn save_as_stl_masked(&self, path: &str, invert: bool, mask: Mask, z_scaling: f64, base_thickness: f32) -> Result<(), LasToStlError>{

        println!("save as stl masked");

        let now = SystemTime::now();

        let z_scale_factor = z_scaling * self.x_res as f64 / self.bounds.x_range();

        let top_vertex_list: Vec<Option<Vertex>> = self.data.iter().enumerate().map(|(index, height)| {
            if mask.data[index] ^ invert {
                None
            } else {
                let x = index % self.x_res;
                let y = index / self.x_res;
                Some(Vertex::new([x as f32, y as f32, (normal_or_default(height - self.bounds.min_z, 0f64) * z_scale_factor) as f32 + base_thickness]))
            }
        }).collect::<Vec<Option<Vertex>>>();

        let bottom_vertex_list: Vec<Option<Vertex>> = self.data.iter().enumerate().map(|(index, _height)| {
            match mask.data[index] ^ invert{
                true => {
                    None
                }
                false => {
                    let x = index % self.x_res;
                    let y = index / self.x_res;
                    Some(Vertex::new([x as f32, y as f32, 0f32]))
                }
            }

        }).collect::<Vec<Option<Vertex>>>();

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

        let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
        stl_io::write_stl(&mut file, triangle_list.iter())?;

        println!("save as stl masked top and bottom done in {:?}", now.elapsed());

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

/// will preserve order, so if you want them to be clockwise, pass them clockwise and vice versa
pub fn vertex_rec_to_triangles(vertex_1: Vertex, vertex_2: Vertex, vertex_3: Vertex, vertex_4: Vertex, normal: Normal) -> (Triangle, Triangle){
    todo!()
}
