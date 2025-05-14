use std::f32::consts::PI;

use hord3::{defaults::default_rendering::vectorinator_binned::rendering_spaces::ViewportData, horde::{geometry::{rotation::{Orientation, Rotation}, vec3d::{Vec3D, Vec3Df}}, rendering::camera::Camera}};

pub fn reverse_to_camera_coords(raster_space:Vec3Df, viewport_data:&ViewportData) -> Vec3Df {
    let real_z = 1.0/raster_space.z;
    Vec3D::new(
        (((raster_space.x / viewport_data.half_image_width) - 1.0) * real_z) / viewport_data.near_clipping_plane,
        ((-(raster_space.y / (viewport_data.half_image_height * viewport_data.aspect_ratio)) + 1.0) * real_z) / viewport_data.near_clipping_plane, 
        real_z
    )
}

pub fn reverse_from_camera_to_worldpos(camera_space:Vec3Df, camera:Camera) -> Vec3Df {
    let actual_orient = Orientation::new(-camera.orient.yaw, -camera.orient.pitch, -camera.orient.roll);
    let cool_rotat = Rotation::new_from_inverted_orient(actual_orient);
    cool_rotat.rotate(camera_space) + camera.pos
}

/// Only works for zero orientation on the camera side
pub fn reverse_from_raster_to_worldpos(raster_space:Vec3Df, viewport_data:&ViewportData, camera:Camera) -> Vec3D<f32> {
    reverse_from_camera_to_worldpos(reverse_to_camera_coords(raster_space, viewport_data), camera)
}