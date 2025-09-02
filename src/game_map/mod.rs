use std::{collections::{HashMap, VecDeque}, f32::consts::{PI, SQRT_2}, simd::{num::SimdFloat, Simd}, sync::{Arc, LazyLock}};

use hord3::{defaults::default_rendering::vectorinator_binned::{meshes::{Mesh, MeshID, MeshInstance, MeshLOD, MeshLODS, MeshLODType, MeshTriangles, TrianglePoint}, triangles::{collux_f32_a_u8, collux_one_simd_to_u8_level, collux_u8_a_f32, collux_u8_tuple_to_f32_simd}, Vectorinator, VectorinatorWrite}, horde::{game_engine::{entity::Renderable, multiplayer::Identify, world::{World, WorldEvent}}, geometry::{rotation::{Orientation, Rotation}, vec3d::{Vec3D, Vec3Df}}, rendering::RenderingBackend}, tests::engine_derive_test::TestRB};
use vec_sparse_grid::{SetGrid, SetGridUpdate};


pub mod light_spreader;
pub mod raycaster;
pub mod sparse_grid;
pub mod vec_sparse_grid;

pub const VEC_LENGTH:usize = 4;
pub const SET_CAPACITY:usize = 16;


#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct VoxelLight {
    level:u8,
    r:u8,
    g:u8,
    b:u8
}

impl VoxelLight {
    pub fn new(level:u8, r:u8, g:u8, b:u8) -> Self {
        Self { level, r, g, b }
    }
    pub fn random_light() -> Self {
        Self::new(fastrand::u8(0..255), fastrand::u8(0..255), fastrand::u8(0..255), fastrand::u8(0..255))
    }
    pub fn zero_light() -> Self {
        Self { level: 0, r: 0, g: 0, b: 0 }
    }
    pub fn max_light() -> Self {
        Self { level:255, r:255, g:255, b:255 }
    }
    pub fn slightly_less_random_light() -> Self {
        let mut rgb = [0 ; 3];
        for i in 0..fastrand::usize(1..3) {
            rgb[fastrand::usize(0..3)] = fastrand::u8(200..=255);
        }
        Self { level: fastrand::u8(230..=255), r: rgb[0], g: rgb[1], b: rgb[2] }
    }
    pub fn merge_with_other(&self, other:&Self) -> Self {
        let collux_self = collux_u8_tuple_to_f32_simd((self.r, self.g, self.b), self.level);
        let collux_other = collux_u8_tuple_to_f32_simd((other.r, other.g, other.b), other.level);
        let level_self = Simd::splat(collux_self.to_array()[3]);
        let level_other = Simd::splat(collux_other.to_array()[3]);
        let a01 = ((Simd::splat(1.0) - level_self) * level_other + level_self).simd_max(Simd::splat(0.001));
        let new_col = (((Simd::splat(1.0) - level_self) * level_other * collux_other + level_self * collux_self)/a01).simd_min(Simd::splat(1.0));
        let ((r,g,b), _) = collux_one_simd_to_u8_level(new_col);
        /* 
        let total_per_color = (collux_self + collux_other).simd_max(Simd::splat(0.01));
        let part_per_color_self = collux_self/total_per_color;
        let part_per_color_other = collux_other/total_per_color;
        let level = collux_one_simd_to_u8_level(collux_self).1.max(collux_one_simd_to_u8_level(collux_other).1);
        let ((r,g,b), nlevel) = collux_one_simd_to_u8_level(collux_self * part_per_color_self + collux_other * part_per_color_other);

        let ((nr,ng,nb), nlevel) = collux_one_simd_to_u8_level(total_per_color);
        let level_self = self.level as f32 / 255.0;
        let level_other = other.level as f32 / 255.0;
        let total_level = level_self + level_other;
        let part_of_self = level_self/total_level;
        let part_of_other = level_other/total_level;
        let final_color = collux_f32_a_u8((part_of_self * collux_self.0 + part_of_other * collux_other.0, part_of_self * collux_self.1 + part_of_other * collux_other.1, part_of_self * collux_self.2 + part_of_other * collux_other.2));
        let final_level = ((part_of_self * level_self + part_of_other * level_other) * 255.0) as u8;
        Self { level: final_level, r: final_color.0, g: final_color.1, b: final_color.2 }
        */
        Self { level:(a01.to_array()[0] * 255.0).min(255.0) as u8, r, g, b }
    }
}

pub trait Voxel:Clone + Send + Sync {
    type VT:VoxelType;
    fn voxel_id(&self) -> usize;
    fn orientation(&self) -> u8;
    fn light_level(&self) -> VoxelLight;
    /*fn orientation_converted(&self) -> Orientation {
        // first 3 bits = which side of the voxel is the underside against
        // 000 => base
        // 001 => backside
        // 010 => top side
        // 011 => front side
        // 100 => left side
        // 101 => right side
        // next 2 bits = which horizontal rotation around the underside
        // 00 => no rotation
        // 01 => pi/2
        // 10 => pi
        // 11 => -pi

        let side = match self.orientation() & 7 {
            0 => Orientation::zero(),
            1 => Orientation::new(0.0, PI/2.0, 0.0),
            2 => Orientation::new(0.0, PI, 0.0),
            3 => Orientation::new(0.0, -PI/2.0, 0.0),
            4 => Orientation::new(PI/2.0, PI/2.0, 0.0),
            5 => Orientation::new(-PI/2.0, PI/2.0, 0.0),
            _ => panic!("At the disco")
        };
        side
    }*/
}




pub enum VoxelModel {
    WrappedTexture(usize),
    SpecifiedTexture([usize ; 6]),
    Custom
}

pub trait VoxelType:Clone + Send + Sync {
    fn sides_empty(&self) -> u8;
    fn vertices_taken(&self) -> u8;
    fn kind_of_model(&self) -> VoxelModel;
    fn light_passthrough(&self) -> VoxelLight;
    fn empty_coming_from(&self, from:u8, orientation:u8) -> bool {
        let rotated_2_empty = self.empties_with_orientation(orientation);
        rotated_2_empty >> (from as u32) & 1 == 1
    }
    fn empties_with_orientation(&self, orientation:u8) -> u8 {
        let rotat_1 = (orientation >> 3) & 0b00000011;
        let self_empty = self.sides_empty();
        let mut rotated_1_empty:u8 = self_empty;
        for (i, rotates) in ROTATES_ON_1.into_iter().enumerate() {
            let target = LUT_ROTAT_1[rotat_1 as usize][i];
            if target >= rotates {

                rotated_1_empty |= self_empty & (1 << rotates) << (target - rotates)
            }
            else {
                rotated_1_empty |= self_empty & (1 << rotates) >> (rotates - target)
            }
        }
        let mut rotated_2_empty:u8 = 0;
        let rotat_2 = orientation & 0b00000111;
        for i in [0, 1, 2, 3, 4, 5] {
            let target = LUT_ROTAT_2[rotat_2 as usize][i as usize];
            if target >= i {

                rotated_2_empty |= rotated_1_empty & (1 << i) << (target - i)
            }
            else {
                rotated_2_empty |= rotated_1_empty & (1 << i) >> (i - target)
            }
        }
        if self_empty != rotated_2_empty {
            dbg!(self_empty, rotated_2_empty, rotated_1_empty);
        }   
        
        rotated_2_empty
    }
    fn full_coming_from(&self, from:u8, orientation:u8) -> bool {
        !self.empty_coming_from(from, orientation)
    }
    fn is_completely_empty(&self) -> bool {
        (self.sides_empty() & 0b00111111) == 0b00111111
    }
    fn sides_empty_bools(&self) -> [bool ; 6] {
        let empty = self.sides_empty();
        let mut bools = [false ; 6];
        for i in 0..6 {
            bools[i] = ((empty >> i) & 1) == 1
        }
        bools
    }
    fn easy_texture(&self) -> usize;
}

const EXPLORATION:[Vec3D<i32> ; 6] = [
    Vec3D::new(0, 0, 1),
    Vec3D::new(1, 0, 0),
    Vec3D::new(0, 0, -1),
    Vec3D::new(-1, 0, 0),
    Vec3D::new(0, -1, 0),
    Vec3D::new(0, 1, 0)
];

const DIR_MASK:[u8 ; 6] = [
    0b00000001,
    0b00000010,
    0b00000100,
    0b00001000,
    0b00010000,
    0b00100000
];

// first 3 bits = which side of the voxel is the underside against
// 000 => base
// 001 => backside
// 010 => top side
// 011 => front side
// 100 => left side
// 101 => right side
// next 2 bits = which horizontal rotation around the underside
// 00 => no rotation
// 01 => pi/2
// 10 => pi
// 11 => -pi
const ROTATES_ON_1:[u32 ; 4] = [
    1, 3, 4, 5
];

const LUT_ROTAT_1:[[u32 ; 4] ; 4] = [
    [1, 3, 4, 5],
    [3, 4, 5, 1],
    [4, 5, 1, 3],
    [5, 1, 3, 4]
];

const LUT_ROTAT_2:[[u32 ; 6]; 6] = [
    [0, 1, 2, 3, 4, 5],
    [1, 2, 3, 0, 4, 5],
    [2, 3, 0, 1, 4, 5],
    [3, 0, 1, 2, 4, 5],
    [4, 1, 5, 3, 2, 0],
    [5, 1, 4, 3, 0, 2],
];

const EMPTY_VOXEL:u8 = 0b00111111;

#[derive(Clone)]
pub struct MapChunk<V:Voxel> {
    voxels:Vec<V>,
    origin_worldpos:WorldVoxelPos,
    chunk_coord:WorldChunkPos,
    mesh_id:Option<usize>,
    mesh_updated:bool,
    mesh_instance:Option<usize>,
}

#[derive(Clone)]
pub enum GameMapEvent<V:Voxel> {
    UpdateVoxelAt(WorldVoxelPos, V),
    UpdateSetGrid(SetGridUpdate)
}

impl<ID:Identify, V:Voxel> WorldEvent<GameMap<V>, ID> for GameMapEvent<V> {
    fn get_source(&self) -> Option<ID> {
        None
    }
    fn should_sync(&self) -> bool {
        true
    }
    fn apply_event(self, world:&mut GameMap<V>) {
        match self {
            GameMapEvent::UpdateVoxelAt(pos, new_voxel) => {world.get_voxel_at_mut(pos).and_then(|vox| { *vox = new_voxel; None::<()>});},
            GameMapEvent::UpdateSetGrid(set_grid_update) => world.set_grid.apply_update::<VEC_LENGTH, SET_CAPACITY>(set_grid_update),
        }
    }
}

impl<V:Voxel, ID:Identify> World<ID> for GameMap<V> {
    type RB = TestRB;
    type WE = GameMapEvent<V>;
    fn update_rendering(&mut self, data:&mut <Self::RB as RenderingBackend>::PreTickData) {
        
    }
}

impl<'a, V:Voxel> Renderable<VectorinatorWrite<'a>> for GameMap<V> {
    fn do_render_changes(&mut self, render_data:&mut VectorinatorWrite<'a>) {
        if !self.rendering_up_to_date {
            let mut iterator = if self.remesh_fasttrack.len() > 0 {
                self.remesh_fasttrack.clone()
            }
            else {
                self.chunks.keys().map(|vector| {vector.clone()}).collect::<Vec<Vec3D<i32>>>()
            };
            self.remesh_fasttrack.clear();
            //let mut test_chunk_tris = 0;
            //let mut test_chunk_id = 0;
            let mesh_vec = self.mesh_vec;
            for pos in iterator {
                let around = self.get_chunks_around(pos);
                let chunk = self.chunks.get(&pos).unwrap();
                if chunk.mesh_updated == false || self.forced_rerender {
                    let mut x = Vec::with_capacity(600);
                    let mut y = Vec::with_capacity(600);
                    let mut z = Vec::with_capacity(600);
                    let mut triangles = MeshTriangles::with_capacity(600);
                    let mut lod = MeshLOD::new(x, y, z, triangles);
                    for x in 0..self.dims.chunk_length_i {
                        for y in 0..self.dims.chunk_width_i {
                            for z in 0..self.dims.chunk_height_i {

                                let voxel = chunk.get_at_local(Vec3D::new(x, y, z), &self.dims).unwrap();
                                if !&self.voxel_types[voxel.voxel_id()].is_completely_empty() {
                                    let empty_dirs = chunk.get_empty_directions_local(Vec3D::new(x, y, z), &self.dims, around, &self.voxel_types);
                                    let lights = chunk.get_lights_local(Vec3D::new(x, y, z), &self.dims, around);
                                    //dbg!(Vec3D::new(x, y, z));
                                    for (i, mask) in DIR_MASK.iter().enumerate() {
                                        /*if empty_dirs != 0 {
                                            dbg!(empty_dirs);
                                        }*/
                                        if empty_dirs & mask  == *mask { //if a direction is empty (hint:it's a me, Mario)
                                            let mut points = TRIS_INDICES_UVS.0[i];
                                            let indices = TRIS_INDICES_UVS.1;
                                            let uvs = TRIS_INDICES_UVS.2;
                                            let start_index = lod.x.len();
                                            let mut level = (lights[i].level.max(50) as f32) / 255.0;
                                            let mut converted_collux = collux_u8_a_f32((lights[i].r, lights[i].g, lights[i].b));
                                            let mut finished_collux = collux_f32_a_u8((converted_collux.0 * level, converted_collux.1 * level, converted_collux.2 * level));
                                            finished_collux = (finished_collux.0.max(self.min_light_levels.0), finished_collux.1.max(self.min_light_levels.1), finished_collux.2.max(self.min_light_levels.2));
                                            points = points + Vec3D::new(x as f32, y as f32, z as f32); 
                                            lod.add_points(&points);
                                            lod.triangles.add_triangle(
                                                TrianglePoint::new(
                                                    indices[0] + start_index,
                                                    uvs[indices[0]].0,
                                                    uvs[indices[0]].1,
                                                    finished_collux.0,
                                                    finished_collux.1,
                                                    finished_collux.2
                                                ),
                                                TrianglePoint::new(
                                                    indices[1] + start_index,
                                                    uvs[indices[1]].0,
                                                    uvs[indices[1]].1,
                                                    finished_collux.0,
                                                    finished_collux.1,
                                                    finished_collux.2
                                                ),
                                                TrianglePoint::new(
                                                    indices[2] + start_index,
                                                    uvs[indices[2]].0,
                                                    uvs[indices[2]].1,
                                                    finished_collux.0,
                                                    finished_collux.1,
                                                    finished_collux.2
                                                ),
                                                self.voxel_types[voxel.voxel_id()].easy_texture() as u32, 
                                                0
                                            );
                                            lod.triangles.add_triangle(
                                                TrianglePoint::new(
                                                    indices[3] + start_index,
                                                    uvs[indices[3]].0,
                                                    uvs[indices[3]].1,
                                                    finished_collux.0,
                                                    finished_collux.1,
                                                    finished_collux.2
                                                ),
                                                TrianglePoint::new(
                                                    indices[4] + start_index,
                                                    uvs[indices[4]].0,
                                                    uvs[indices[4]].1,
                                                    finished_collux.0,
                                                    finished_collux.1,
                                                    finished_collux.2
                                                ),
                                                TrianglePoint::new(
                                                    indices[5] + start_index,
                                                    uvs[indices[5]].0,
                                                    uvs[indices[5]].1,
                                                    finished_collux.0,
                                                    finished_collux.1,
                                                    finished_collux.2
                                                ),
                                                self.voxel_types[voxel.voxel_id()].easy_texture() as u32, 
                                                0
                                            );
                                        }
                                    }
                                }
                                
                            }
                        }
                    }
                    
                    let size = self.dims.chunk_length_f*SQRT_2;
                    let height = self.dims.chunk_height_f;
                    let mut chunk = self.get_chunk_at_mut(pos).unwrap();
                    match chunk.mesh_id {
                        Some(id) => {
                            render_data.meshes.set_mesh(&MeshID::Referenced(id), Mesh::new(MeshLODS::new(vec![MeshLODType::Mesh(Arc::new(lod))]), format!("Chunk {} {} {} {}", pos.x, pos.y, pos.z, mesh_vec), size));
                        },
                        None => {
                            /*let mut local_cool = false;
                            if lod.triangles.len() > test_chunk_tris {
                                test_chunk_tris = lod.triangles.len();
                                local_cool = true;
                            }*/
                            let id = render_data.meshes.add_mesh(Mesh::new(MeshLODS::new(vec![MeshLODType::Mesh(Arc::new(lod))]), format!("Chunk {} {} {} {}", pos.x, pos.y, pos.z, mesh_vec), size));
                            /*if local_cool {
                                test_chunk_id = id;
                            }*/
                            
                            chunk.mesh_id = Some(id);   
                        }
                    }
                    match chunk.mesh_instance {
                        Some(index) => render_data.meshes.instances[mesh_vec].get_instance_mut(index).change_visibility(true),
                        None => {
                            chunk.mesh_instance = Some(render_data.meshes.add_instance(MeshInstance::new(get_float_pos(pos) * height, Orientation::zero(), MeshID::Referenced(chunk.mesh_id.unwrap()), true, false, false), mesh_vec))
                        }
                    }
                    chunk.mesh_updated = true;
                }
            }
            // render_data.meshes.add_instance(MeshInstance::new(Vec3Df::new(5.0, 5.0, 5.0), Orientation::zero(), MeshID::Referenced(test_chunk_id), true, false, true), 3);
            render_data.meshes.change_buffer_size_for_instance_vec(self.mesh_vec, 1);
            self.rendering_up_to_date = true;
            self.forced_rerender = false;
            // YAHOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOo
        }
    }
}

static TRIS_INDICES_UVS:LazyLock<([[Vec3Df ; 4] ; 6], [usize ; 6], [(f32,f32) ; 4])> = std::sync::LazyLock::new(make_cube_tris);


fn make_cube_tris() -> ([[Vec3Df ; 4] ; 6], [usize ; 6], [(f32,f32) ; 4]) {
    let mut first_face = [
            Vec3D::new(0.5, -0.5, - 0.5),
            Vec3D::new(- 0.5, -0.5, 0.5),
            Vec3D::new(- 0.5, -0.5, - 0.5),
            Vec3D::new(0.5, -0.5, 0.5),
    ];
    let mut indices = [
        2,1,0, 0, 1, 3 
    ];
    let mut texture_uvs = [
        (0.0, 1.0),
        (1.0, 0.0),
        (1.0, 1.0),
        (0.0, 0.0),
    ];
    (
        [

            Rotation::from_orientation(Orientation::new(0.0, 0.0, -PI/2.0)).rotate_array(&first_face), // facing down
            Rotation::from_orientation(Orientation::new(PI/2.0, 0.0, 0.0)).rotate_array(&first_face), // facing front
            Rotation::from_orientation(Orientation::new(0.0, 0.0, PI/2.0)).rotate_array(&first_face), // facing up
            Rotation::from_orientation(Orientation::new(-PI/2.0, 0.0, 0.0)).rotate_array(&first_face), // facing back
            Rotation::from_orientation(Orientation::new(0.0, 0.0, 0.0)).rotate_array(&first_face), // facing right
            Rotation::from_orientation(Orientation::new(PI, 0.0, 0.0)).rotate_array(&first_face), // facing left
            
        ],
        indices,
        texture_uvs,
    )

}

#[derive(Clone)]
pub struct ChunkDims {
    chunk_width:usize,
    chunk_height:usize,
    chunk_slice_area:usize,
    chunk_length:usize,

    chunk_width_i:i32,
    chunk_height_i:i32,
    chunk_slice_area_i:i32,
    chunk_length_i:i32,

    chunk_width_f:f32,
    chunk_height_f:f32,
    chunk_slice_area_f:f32,
    chunk_length_f:f32,
}

impl ChunkDims {
    pub fn get_add_to_other_local(&self, index:usize) -> Vec3D<i32> {
        match index {
            0 => Vec3D::new(0,0, -self.chunk_height_i),
            1 => Vec3D::new(-self.chunk_length_i,0, 0),
            2 => Vec3D::new(0,0, self.chunk_height_i),
            3 => Vec3D::new(self.chunk_length_i,0, 0),
            4 => Vec3D::new(0,self.chunk_width_i, 0),
            5 => Vec3D::new(0,-self.chunk_width_i, 0),
            _ => panic!("Index outside of normal parameters {}", index)
        }
    }
    pub fn new(width:usize, length:usize, height:usize) -> Self {
        Self {
            chunk_width: width,
            chunk_height: height,
            chunk_slice_area: width * length,
            chunk_length: length,

            chunk_width_i: width as i32,
            chunk_height_i: height as i32,
            chunk_slice_area_i: (width * length) as i32,
            chunk_length_i: length as i32,
            
            chunk_width_f: width as f32, 
            chunk_height_f: height as f32,
            chunk_slice_area_f: (width * length) as f32,
            chunk_length_f: length as f32
        }
    }
}

impl<V:Voxel> MapChunk<V> {
    pub fn new(orig_worldpos:WorldVoxelPos, chunk_pos:WorldChunkPos, data:Vec<V>) -> Self {
        Self { voxels:data, origin_worldpos:orig_worldpos, chunk_coord:chunk_pos, mesh_id: None, mesh_updated: false, mesh_instance: None }
    }
    pub fn get_at_local(&self, pos:WorldVoxelPos, dims:&ChunkDims) -> Option<&V> {
        if pos.in_origin_prism(dims.chunk_length_i, dims.chunk_width_i, dims.chunk_height_i) {
            Some(self.get_voxel_data(pos, dims))
        }
        else {
            None
        }
    }
    fn get_voxel_data(&self, pos:WorldVoxelPos, dims:&ChunkDims) -> &V {
        &self.voxels[pos.x as usize + (pos.y as usize * dims.chunk_length) + (pos.z as usize * dims.chunk_slice_area)]
    }
    fn get_voxel_data_mut(&mut self, pos:WorldVoxelPos, dims:&ChunkDims) -> &mut V {
        &mut self.voxels[pos.x as usize + (pos.y as usize * dims.chunk_length) + (pos.z as usize * dims.chunk_slice_area)]
    }
    fn get_at_worldpos(&self, pos:WorldVoxelPos, dims:&ChunkDims) -> Option<&V> {
        self.get_at_local(pos - self.origin_worldpos, dims)
    }
    fn get_voxels_around<'a>(&'a self, local_pos:WorldVoxelPos, surrounding_chunks:[Option<&'a MapChunk<V>> ; 6], dims:&ChunkDims) -> [Option<&'a V> ; 6] { 
        let mut voxels = [None ; 6];
        for (i, dir) in EXPLORATION.into_iter().enumerate() {
            match self.get_at_local(local_pos + dir, dims) {
                Some(voxel) => voxels[i] = Some(voxel),
                None => match surrounding_chunks[i] {
                    Some(chunk) => voxels[i] = chunk.get_at_local(local_pos + dir + dims.get_add_to_other_local(i), dims),
                    None => ()
                }
            }
        }
        voxels
    }
    fn mark_for_remesh(&mut self) {
        self.mesh_updated = false;
    }
    fn get_at_worldpos_mut(&mut self, pos:WorldVoxelPos, dims:&ChunkDims) -> Option<&mut V> {
        self.get_at_local_mut(pos - self.origin_worldpos, dims)
    }
    fn get_at_local_mut(&mut self, pos:WorldVoxelPos, dims:&ChunkDims) -> Option<&mut V> {
        if pos.in_origin_prism(dims.chunk_length_i, dims.chunk_width_i, dims.chunk_height_i) {
            Some(self.get_voxel_data_mut(pos, dims))
        }
        else {
            None
        }
    }
    fn get_empty_directions_local<'a>(&self, pos:WorldVoxelPos, dims:&ChunkDims, chunks_around:[Option<&'a MapChunk<V>> ; 6], voxel_types:&Vec<V::VT>) -> u8 {
        let center = self.get_at_local(pos, dims);
        let voxels_around = self.get_voxels_around(pos, chunks_around, dims);
        match center {
            Some(voxel) => {
                let mut self_empty = voxel_types[voxel.voxel_id()].empties_with_orientation(voxel.orientation());
                for (i, dir) in EXPLORATION.iter().enumerate() {
                    match voxels_around[i] {
                        Some(vox_a) => self_empty |= ((voxel_types[vox_a.voxel_id()].empty_coming_from(i as u8, vox_a.orientation()) as u8) << i),
                        None => ()
                    } 
                }
                self_empty
                
            },
            None => EMPTY_VOXEL
        }
    }
    pub fn get_lights_local<'a>(&self, pos:WorldVoxelPos, dims:&ChunkDims, chunks_around:[Option<&'a MapChunk<V>> ; 6]) -> [VoxelLight ; 6] {
        let center = self.get_at_local(pos, dims);
        let voxels_around = self.get_voxels_around(pos, chunks_around, dims);
        let mut lights = [VoxelLight::new(0, 0, 0, 0) ; 6];
        match center {
            Some(voxel) => {
                for (i, dir) in EXPLORATION.iter().enumerate() {
                    match voxels_around[i] {
                        Some(vox_a) => lights[i] = vox_a.light_level(),
                        None => ()
                    } 
                }
            },
            None => ()
        }
        lights
    }
    
}


#[derive(Clone)]
pub struct GameMap<V:Voxel> {
    chunks:HashMap<WorldChunkPos, MapChunk<V>>,
    dims:ChunkDims,
    voxel_types:Vec<V::VT>,
    mesh_vec:usize,
    rendering_up_to_date:bool,
    forced_rerender:bool,
    min_light_levels:(u8,u8,u8),
    remesh_fasttrack:Vec<WorldChunkPos>,
    pub set_grid:SetGrid
}

pub type WorldChunkPos = Vec3D<i32>;
pub type WorldVoxelPos = Vec3D<i32>;

pub fn get_chunk_pos_i(dims:&ChunkDims, pos:Vec3D<i32>) -> WorldChunkPos {
    Vec3D::new(
        (pos.x.div_floor(dims.chunk_length_i)),
        (pos.y.div_floor(dims.chunk_width_i)),
        (pos.z.div_floor(dims.chunk_height_i)),
    )
}
impl<V:Voxel> GameMap<V> {
    pub fn new(expected_chunks:usize, dims:ChunkDims, voxel_types:Vec<V::VT>, min_light_levels:(u8,u8,u8), mesh_vec:usize) -> Self {
        Self { chunks: HashMap::with_capacity(expected_chunks), dims, voxel_types, forced_rerender:false, min_light_levels, mesh_vec, rendering_up_to_date: false, remesh_fasttrack:Vec::with_capacity(16), set_grid:SetGrid::new(5.0, Vec3D::all_ones() * -15, Vec3D::all_ones() * 15) }
    }
    pub fn does_chunk_exist(&self, chunk:WorldChunkPos) -> bool {
        self.chunks.contains_key(&chunk)
    }
    pub fn set_min_light_levels(&mut self, mins:(u8,u8,u8)) {
        self.min_light_levels = mins;
    }
    pub fn force_rerender(&mut self) {
        self.rendering_up_to_date = false;
        self.forced_rerender = true;
    }
    pub fn modified_this_pos_signal_remesh(&mut self, pos:WorldVoxelPos) {
        let mut must_re_render = false;
        let dims = self.dims.clone();
        let mut add_to_fasttrack = Vec::with_capacity(7);
        self.get_chunk_at_mut(self.get_chunk_pos_i(pos)).and_then(|chunk| {chunk.mark_for_remesh(); add_to_fasttrack.push(get_chunk_pos_i(&dims, pos)); must_re_render = true; Some(1_i32)});
        for dir in EXPLORATION {
            self.get_chunk_at_mut(self.get_chunk_pos_i(pos + dir)).and_then(|chunk| {chunk.mark_for_remesh(); add_to_fasttrack.push(get_chunk_pos_i(&dims, pos + dir)); must_re_render = true; Some(1_i32)});
        }
        for add in add_to_fasttrack {
            self.remesh_fasttrack.push(add);
        }
        self.rendering_up_to_date = !must_re_render;
    }
    pub fn generate_chunk<F:FnMut(Vec3D<i32>) -> V>(&mut self, chunk_pos:WorldChunkPos, func:&mut F) {
        let mut chunk_data = Vec::with_capacity(self.dims.chunk_slice_area * self.dims.chunk_height);
        let mut orig_worldpos = Vec3D::new(chunk_pos.x * self.dims.chunk_length_i, chunk_pos.y * self.dims.chunk_width_i, chunk_pos.z * self.dims.chunk_height_i);
        for z in orig_worldpos.z..orig_worldpos.z + self.dims.chunk_height_i {
            for y in orig_worldpos.y..orig_worldpos.y + self.dims.chunk_width_i {
                for x in orig_worldpos.x..orig_worldpos.x + self.dims.chunk_length_i {
                    chunk_data.push(func(Vec3D::new(x, y, z)));
                }
            }
        }
        self.chunks.insert(chunk_pos, MapChunk::new(orig_worldpos, chunk_pos, chunk_data));
    }
    pub fn get_all_chunk_pos(&self) -> Vec<Vec3D<i32>> {
        self.chunks.clone().keys().map(|vector| {vector.clone()}).collect::<Vec<Vec3D<i32>>>()
    }
    pub fn generate_chunks<F:FnMut(Vec3D<i32>) -> V>(&mut self, start:WorldChunkPos, end:WorldChunkPos, func:&mut F) {
        self.set_grid = SetGrid::new(4.0, start * self.dims.chunk_length_i, end * self.dims.chunk_length_i);
        for xc in start.x..end.x {
            for yc in start.y..end.y {
                for zc in start.z..end.z {
                    self.generate_chunk(Vec3D::new(xc, yc, zc), func);
                }
            }
        }
    }
    pub fn make_meshes_invisible<'a>(&mut self, write: &mut VectorinatorWrite<'a>) {
        write.meshes.change_visibility_of_all_instances_of_vec(self.mesh_vec, false);
    }
    pub fn change_mesh_vec(&mut self, new_vec:usize) {
        self.mesh_vec = new_vec;
    }

    pub fn make_meshes_visible<'a>(&mut self, write: &mut VectorinatorWrite<'a>) {
        write.meshes.change_visibility_of_all_instances_of_vec(self.mesh_vec, true);
    }
    pub fn get_chunk_at(&self, chunk_pos:WorldChunkPos) -> Option<&MapChunk<V>> {
        self.chunks.get(&chunk_pos)
    }
    pub fn get_chunk_at_mut(&mut self, chunk_pos:WorldChunkPos) -> Option<&mut MapChunk<V>> {
        self.chunks.get_mut(&chunk_pos)
    }
    pub fn get_chunks_around(&self, chunk_pos:WorldChunkPos) -> [Option<&MapChunk<V>> ; 6] {
        let mut chunks = [None ; 6];
        for (i, offset) in EXPLORATION.into_iter().enumerate() {
            chunks[i] = self.get_chunk_at(chunk_pos + offset)
        }
        chunks
    }
    pub fn get_chunk_and_surrounding(&self, chunk_pos:WorldChunkPos) -> (Option<&MapChunk<V>>, [Option<&MapChunk<V>> ; 6]) {
        (
            self.get_chunk_at(chunk_pos),
            self.get_chunks_around(chunk_pos)
        )
    }
    pub fn get_chunk_pos(&self, pos:Vec3Df) -> WorldChunkPos {
        Vec3D::new(
            (pos.x/self.dims.chunk_length_f).trunc() as i32,
            (pos.y/self.dims.chunk_width_f).trunc() as i32,
            (pos.z/self.dims.chunk_height_f - 1.0).trunc() as i32,
        )
    }
    pub fn get_chunk_pos_i(&self, pos:Vec3D<i32>) -> WorldChunkPos {
        Vec3D::new(
            (pos.x.div_floor(self.dims.chunk_length_i)),
            (pos.y.div_floor(self.dims.chunk_width_i)),
            (pos.z.div_floor(self.dims.chunk_height_i)),
        )
    }
    pub fn get_chunk_dims_vector(&self) -> Vec3D<i32> {
        Vec3D::new(
            self.dims.chunk_length_i,
            self.dims.chunk_width_i,
            self.dims.chunk_height_i,
        )
    }
    pub fn is_voxel_solid(&self, voxel:WorldVoxelPos) -> bool {
        self.get_voxel_at(voxel).is_some_and(|voxel| {!self.get_voxel_types()[voxel.voxel_id()].is_completely_empty()})
    }
    pub fn get_voxel_at_mut(&mut self, voxel:WorldVoxelPos) -> Option<&mut V> {
        let dims = self.dims.clone();
        self.get_chunk_at_mut(self.get_chunk_pos_i(voxel)).and_then(|chunk| {chunk.get_at_worldpos_mut(voxel, &dims)})
    }
    pub fn get_voxel_at(&self, voxel:WorldVoxelPos) -> Option<&V> {
        let dims = self.dims.clone();
        self.get_chunk_at(self.get_chunk_pos_i(voxel)).and_then(|chunk| {chunk.get_at_worldpos(voxel, &dims)})
    }
    pub fn get_type_of_voxel_at(&self, voxel:WorldVoxelPos) -> Option<&V::VT> {
        let dims = self.dims.clone();
        self.get_chunk_at(self.get_chunk_pos_i(voxel)).and_then(|chunk| {chunk.get_at_worldpos(voxel, &dims)}).and_then(|voxel| {Some(&self.voxel_types[voxel.voxel_id()])})
    }
    pub fn get_ceiling_at(&self, pos:WorldVoxelPos, margin:i32) -> WorldVoxelPos {
        for z in ((pos.z - margin)..(pos.z + margin)).rev() {
            match self.get_voxel_at(Vec3D::new(pos.x, pos.y, z)) {
                Some(voxel) => if !self.get_voxel_types()[voxel.voxel_id()].is_completely_empty() {
                    return Vec3D::new(pos.x, pos.y, z)
                },
                None => ()
            }
        }
        pos
    }
    pub fn get_voxel_types(&self) -> &Vec<V::VT> {
        &self.voxel_types
    }
}

pub fn get_voxel_pos(pos:Vec3Df) -> WorldVoxelPos {
    Vec3D::new(
        pos.x as i32,
        pos.y as i32,
        pos.z as i32
    )
}

pub fn get_float_pos(pos:WorldVoxelPos) -> Vec3Df {
    Vec3D::new(
        pos.x as f32,
        pos.y as f32,
        pos.z as f32
    )
}

pub fn get_f64_pos(pos:WorldVoxelPos) -> Vec3D<f64> {
    Vec3D::new(
        pos.x as f64,
        pos.y as f64,
        pos.z as f64
    )
}

