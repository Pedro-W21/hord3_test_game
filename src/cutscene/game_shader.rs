use std::sync::{atomic::{AtomicBool, Ordering}, Arc, RwLock};

use hord3::{defaults::default_rendering::{vectorinator::{textures::{argb_to_rgb, rgb_to_argb, rgbu_to_rgbf}, triangles::collux_f32_a_u8}, vectorinator_binned::shaders::{ShaderData, ShaderFrameData}}, horde::{geometry::{rotation::Rotation, vec3d::{Vec3D, Vec3Df}}, rendering::camera::Camera}};

#[derive(Clone)]
pub struct GameShader {
    pub activated:Arc<AtomicBool>,
    pub fog_distance:Arc<RwLock<f32>>,
    pub fog_color:Arc<RwLock<u32>>,
    pub sun_dir:Arc<RwLock<Vec3Df>>,
    pub do_normals:Arc<AtomicBool>
    
}

impl GameShader {
    pub fn new_default() -> Self {
        Self {  sun_dir:Arc::new(RwLock::new(Vec3Df::all_ones().normalise())), activated: Arc::new(AtomicBool::new(true)), fog_distance: Arc::new(RwLock::new(1000.0)), fog_color: Arc::new(RwLock::new(rgb_to_argb((53, 81, 92)))), do_normals: Arc::new(AtomicBool::new(true)) }
    }
}

impl ShaderData for GameShader {
    type SFD = GameShaderFrameData;
    fn get_frame_data(&self, cam:&Camera, rotat_cam:&Rotation) -> Self::SFD {
        let fog = self.fog_distance.read().unwrap().clone();
        let sun_dir = rotat_cam.rotate(self.sun_dir.read().unwrap().clone());
        GameShaderFrameData {
            activated:self.activated.load(Ordering::Relaxed),
            inv_fog_distance:1.0/fog,
            fog_distance:fog,
            fog_color:self.fog_color.read().unwrap().clone(),
            fog_color_f:rgbu_to_rgbf(argb_to_rgb(self.fog_color.read().unwrap().clone())),
            sun_dir,
            sun_dir_norm:1.0/sun_dir.norme(),
            do_normals:self.do_normals.load(Ordering::Relaxed),

        }
    }
    fn get_raw_frame_data(&self) -> Self::SFD {
        let fog = self.fog_distance.read().unwrap().clone();
        let sun_dir = self.sun_dir.read().unwrap().clone();
        GameShaderFrameData {
            activated:self.activated.load(Ordering::Relaxed),
            inv_fog_distance:1.0/fog,
            fog_distance:fog,
            fog_color:self.fog_color.read().unwrap().clone(),
            fog_color_f:rgbu_to_rgbf(argb_to_rgb(self.fog_color.read().unwrap().clone())),
            sun_dir,
            sun_dir_norm:1.0/sun_dir.norme(),
            do_normals:self.do_normals.load(Ordering::Relaxed),
        }
    }

}
#[derive(Clone)]
pub struct GameShaderFrameData {
    pub activated:bool,
    pub inv_fog_distance:f32,
    pub fog_distance:f32,
    pub fog_color:u32,
    pub fog_color_f:(f32,f32,f32),
    pub sun_dir:Vec3Df,
    pub sun_dir_norm:f32,
    pub do_normals:bool
}

impl ShaderFrameData for GameShaderFrameData {
    fn get_new_pixel(&mut self, pixel_index:usize, old_color:u32, old_depth:f32, old_normal:u32, framebuf:&Vec<u32>, zbuf:&Vec<f32>, nbuf:&Vec<u32>, width:usize, height:usize) -> u32 {
        if self.activated {
            if old_depth > self.fog_distance {
                self.fog_color
            }
            else {
                unsafe {
                    let coefficient = old_depth * self.inv_fog_distance;
                    let one_m_coef = 1.0 - coefficient;
                    let (fr, fg, fb) = rgbu_to_rgbf(argb_to_rgb(old_color));
                    if self.do_normals {
                        let normal = old_normal.to_le_bytes().map(|byte| {std::mem::transmute::<u8, i8>(byte)});
                        let normal_vec = Vec3Df::new(normal[0] as f32, normal[1] as f32, normal[2] as f32);
                        let scalar = (normal_vec.dot(&self.sun_dir) * 1.0/128.0 * self.sun_dir_norm + 0.1).clamp(0.4, 1.0);
                        rgb_to_argb(collux_f32_a_u8((one_m_coef * (fr * scalar) + coefficient * self.fog_color_f.0, one_m_coef * (fg * scalar) + coefficient * self.fog_color_f.1,one_m_coef * (fb * scalar) + coefficient * self.fog_color_f.2)))
                        
                    }
                    else {
                        rgb_to_argb(collux_f32_a_u8((one_m_coef * (fr) + coefficient * self.fog_color_f.0, one_m_coef * (fg) + coefficient * self.fog_color_f.1,one_m_coef * (fb) + coefficient * self.fog_color_f.2)))
                        
                    }
                    //rgb_to_argb(collux_f32_a_u8(((normal_vec.x + 127.0) * 1.0/255.0, (normal_vec.y + 127.0) * 1.0/255.0, (normal_vec.z + 127.0) * 1.0/255.0)))
                }
            }
        }
        else {
            old_color
        }
    }
}