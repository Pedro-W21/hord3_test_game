use std::ops::{Add, AddAssign};

use hord3::horde::geometry::vec3d::{Vec3D, Vec3Df};
use to_from_bytes_derive::{FromBytes, ToBytes};

use crate::{game_engine::CoolVoxel, game_map::{get_voxel_pos, GameMap, VoxelType}};

#[derive(Clone, Copy, Debug, PartialEq, ToBytes, FromBytes)]
pub struct AABB {
    max: Vec3Df,
    min: Vec3Df,
}

impl AABB {
    pub fn get_vertices(&self) -> [Vec3Df; 8] {
        [
            self.max,
            self.min,
            Vec3Df::new(self.max.x, self.min.y, self.min.z),
            Vec3Df::new(self.min.x, self.max.y, self.min.z),
            Vec3Df::new(self.min.x, self.min.y, self.max.z),
            Vec3Df::new(self.max.x, self.max.y, self.min.z),
            Vec3Df::new(self.max.x, self.min.y, self.max.z),
            Vec3Df::new(self.min.x, self.max.y, self.max.z),
        ]
    }
    pub fn get_ground_vertices(&self) -> [Vec3Df ; 4] {
        [
            self.min,
            Vec3Df::new(self.max.x, self.min.y, self.min.z),
            Vec3Df::new(self.min.x, self.max.y, self.min.z),
            Vec3Df::new(self.max.x, self.max.y, self.min.z),
        ]
    }
    pub fn get_top_vertices(&self) -> [Vec3Df ; 4] {
        [
            self.max,
            Vec3Df::new(self.min.x, self.min.y, self.max.z),
            Vec3Df::new(self.max.x, self.min.y, self.max.z),
            Vec3Df::new(self.min.x, self.max.y, self.max.z),
        ]
    }
    pub fn get_first_point(&self) -> Vec3Df {
        self.min
    }
    pub fn get_second_point(&self) -> Vec3Df {
        self.max
    }
    pub fn get_minimum_side_length(&self) -> f32 {
        let sides = self.max - self.min;
        (sides.x.min(sides.y)).min(sides.z).abs()
    }
    pub fn new_precomputed(min: Vec3Df, max: Vec3Df) -> AABB {
        AABB { max, min }
    }
    pub fn new(point1: Vec3Df, point2: Vec3Df) -> AABB {
        let mut max = Vec3Df::new(0.0, 0.0, 0.0);
        let mut min = Vec3Df::new(0.0, 0.0, 0.0);
        if point1.x > point2.x {
            max.x = point1.x;
            min.x = point2.x;
        } else {
            max.x = point2.x;
            min.x = point1.x;
        }
        if point1.y > point2.y {
            max.y = point1.y;
            min.y = point2.y;
        } else {
            max.y = point2.y;
            min.y = point1.y;
        }
        if point1.z > point2.z {
            max.z = point1.z;
            min.z = point2.z;
        } else {
            max.z = point2.z;
            min.z = point1.z;
        }
        AABB { max, min }
    }
    pub fn get_both_points(&self) -> (Vec3Df, Vec3Df) {
        (self.min, self.max)
    }
    pub fn collision_point(&self, cible: &Vec3Df) -> bool {
        return (cible.x >= self.min.x && cible.x <= self.max.x)
            && (cible.y >= self.min.y && cible.y <= self.max.y)
            && (cible.z >= self.min.z && cible.z <= self.max.z);
    }
    pub fn collision_aabb(&self, cible: &AABB) -> bool {
        return (cible.max.x >= self.min.x && cible.min.x <= self.max.x)
            && (cible.max.y >= self.min.y && cible.min.y <= self.max.y)
            && (cible.max.z >= self.min.z && cible.min.z <= self.max.z);
    }
    pub fn update_avec_spd(&mut self, speed: Vec3Df) {
        self.max.x += speed.x;
        self.max.y += speed.y;
        self.max.z += speed.z;
        self.min.x += speed.x;
        self.min.y += speed.y;
        self.min.z += speed.z;
    }
    pub fn collision_world(&self, world:&GameMap<CoolVoxel>) -> bool {
        for vertex in self.get_vertices() {
            match world.get_voxel_at(get_voxel_pos(vertex)) {
                Some(voxel) => {    
                    if !world.get_voxel_types()[voxel.voxel_type as usize].is_completely_empty() {
                        return true
                    }
                },
                None => {
                    return true
                }
            }
        }
        false
    }
}

impl Add<Vec3Df> for AABB {
    type Output = AABB;
    fn add(self, rhs: Vec3Df) -> Self::Output {
        AABB::new_precomputed(self.min + rhs, self.max + rhs)
    }
}

impl AddAssign<Vec3Df> for AABB {
    fn add_assign(&mut self, rhs: Vec3Df) {
        self.max += rhs;
        self.min += rhs;
    }
}
