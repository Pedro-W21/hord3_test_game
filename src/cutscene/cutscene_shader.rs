use std::time::{Duration, Instant};

use hord3::{defaults::default_rendering::vectorinator::{textures::rgb_to_argb, triangles::collux_f32_a_u8}, horde::{geometry::{rotation::Orientation, vec3d::Vec3Df}, rendering::camera::Camera}};

use super::game_shader::GameShaderFrameData;

#[derive(Clone)]
pub enum ShaderChangeElement {
    ChangeSunPos {from:Vec3Df, to:Vec3Df},
    FogDistanceChange {from:f32, to:f32},
    FogColorChange {from:(f32,f32,f32), to:(f32,f32,f32)},
    Deactivate,
    Activate,
    StayPut,
}


#[derive(Clone)]
pub enum ShaderChangeDuration {
    Ticks{number:usize},
    RealTime{duration:Duration}
}

impl ShaderChangeDuration {
    pub fn from_start(&self, start:StartTime) -> Self {
        match start {
            StartTime::Tick { number } => match self {
                ShaderChangeDuration::Ticks { number:number_elapsed } => Self::Ticks { number: *number_elapsed + 1 },
                _ => panic!("NOT TICKS"),
            },
            StartTime::RealTime { instant } => match self {
                ShaderChangeDuration::RealTime { duration } => Self::RealTime { duration: instant.elapsed() },
                _ => panic!("NOT REAL TIME")
            }
        }
    }
}

#[derive(Clone)]
pub enum StartTime {
    Tick{number:usize},
    RealTime {instant:Instant}
}

pub struct ShaderSequence {
    camera_movements:Vec<ShaderChange>,
    current_sequence_number:usize,
    current_movement:Option<ShaderChange>,
    movement_start_time:StartTime,
    duration_since_start_time:ShaderChangeDuration
}

impl ShaderSequence {
    pub fn advance_sequence(&mut self, old_game_shader:GameShaderFrameData) -> Option<GameShaderFrameData> {
        self.duration_since_start_time = self.duration_since_start_time.from_start(self.movement_start_time.clone());
        match &self.current_movement {
            Some(movement) => {
                if movement.reached_total_time(self.duration_since_start_time.clone()) {
                    self.current_sequence_number += 1;
                    self.current_movement = self.camera_movements.get(self.current_sequence_number).cloned();
                    match &self.current_movement {
                        Some(movement) => {
                            match movement.duration {
                                ShaderChangeDuration::RealTime { duration } => {
                                    self.movement_start_time = StartTime::RealTime { instant: Instant::now() };
                                    self.duration_since_start_time = ShaderChangeDuration::RealTime { duration: Duration::from_secs_f32(0.0) };
                                },
                                ShaderChangeDuration::Ticks { number } => {
                                    self.movement_start_time = StartTime::Tick { number: 0 };
                                    self.duration_since_start_time = ShaderChangeDuration::Ticks { number: 0 }
                                }
                            }
                            Some(movement.get_position_and_rotation(self.duration_since_start_time.clone(), old_game_shader))
                        },
                        None => None
                    }
                }
                else {
                    Some(movement.get_position_and_rotation(self.duration_since_start_time.clone(), old_game_shader))
                }
            },
            None => {
                if self.current_sequence_number < self.camera_movements.len() {
                    self.current_movement = self.camera_movements.get(self.current_sequence_number).cloned();
                    match &self.current_movement {
                        Some(movement) => {
                            match movement.duration {
                                ShaderChangeDuration::RealTime { duration } => {
                                    self.movement_start_time = StartTime::RealTime { instant: Instant::now() };
                                    self.duration_since_start_time = ShaderChangeDuration::RealTime { duration: Duration::from_secs_f32(0.0) };
                                },
                                ShaderChangeDuration::Ticks { number } => {
                                    self.movement_start_time = StartTime::Tick { number: 0 };
                                    self.duration_since_start_time = ShaderChangeDuration::Ticks { number: 0 }
                                }
                            }
                            Some(movement.get_position_and_rotation(self.duration_since_start_time.clone(), old_game_shader))
                        },
                        None => None
                    }
                }
                else {
                    None
                }
            }
        }
    }
    pub fn new(movements:Vec<ShaderChange>) -> Self {
        Self { camera_movements:movements, current_sequence_number:0, current_movement:None, movement_start_time:StartTime::Tick { number: 0 }, duration_since_start_time: ShaderChangeDuration::Ticks { number: 0 }}
    }
}

#[derive(Clone)]
pub struct ShaderChange {
    elements:Vec<ShaderChangeElement>,
    duration:ShaderChangeDuration,
}

impl ShaderChange {
    pub fn new(elements:Vec<ShaderChangeElement>, duration:ShaderChangeDuration) -> Self {
        Self { elements, duration }
    }
    pub fn get_position_and_rotation(&self, time_since_started:ShaderChangeDuration, mut current_game_shader:GameShaderFrameData) -> GameShaderFrameData {
        let mut coefficient = match time_since_started {
            ShaderChangeDuration::Ticks { number } => match self.duration {
                ShaderChangeDuration::Ticks { number:number_total } => (number as f32)/(number_total as f32),
                _ => panic!("NOT POSSIBLE"), 
            },
            ShaderChangeDuration::RealTime { duration } => match self.duration {
                ShaderChangeDuration::RealTime { duration:duration_total } => duration.as_secs_f32()/duration_total.as_secs_f32(),
                _ => panic!("NOT POSSIBLE EITHER"),
            },
        };
        if coefficient < 1.0 {
            for element in &self.elements {
                match element {
                    ShaderChangeElement::ChangeSunPos { from, to } => {
                        current_game_shader.sun_dir = (from + (to - from) * coefficient).normalise();
                    },
                    ShaderChangeElement::FogDistanceChange { from, to } => {
                        current_game_shader.fog_distance = from + (to - from) * coefficient;
                        current_game_shader.inv_fog_distance = 1.0/current_game_shader.fog_distance;
                    },
                    ShaderChangeElement::FogColorChange { from, to } => {
                        current_game_shader.fog_color_f.0 = from.0 + (to.0 - from.0) * coefficient;
                        current_game_shader.fog_color_f.1 = from.1 + (to.1 - from.1) * coefficient;
                        current_game_shader.fog_color_f.2 = from.2 + (to.2 - from.2) * coefficient;
                        current_game_shader.fog_color = rgb_to_argb(collux_f32_a_u8(current_game_shader.fog_color_f));
                    },
                    ShaderChangeElement::Activate => {
                        current_game_shader.activated = true;
                    },
                    ShaderChangeElement::Deactivate => {
                        current_game_shader.activated = false;
                    }
                    _ => ()
                }
            }
            current_game_shader
        }
        else {
            for element in &self.elements {
                match element {
                    ShaderChangeElement::ChangeSunPos { from, to } => {
                        current_game_shader.sun_dir = *to;
                    },
                    _ => ()
                }
            }
            current_game_shader
        }
        
    }
    pub fn reached_total_time(&self, time_since_started:ShaderChangeDuration) -> bool {
        let mut coefficient = match time_since_started {
            ShaderChangeDuration::Ticks { number } => match self.duration {
                ShaderChangeDuration::Ticks { number:number_total } => (number as f32)/(number_total as f32),
                _ => panic!("NOT POSSIBLE"), 
            },
            ShaderChangeDuration::RealTime { duration } => match self.duration {
                ShaderChangeDuration::RealTime { duration:duration_total } => duration.as_secs_f32()/duration_total.as_secs_f32(),
                _ => panic!("NOT POSSIBLE EITHER"),
            },
        };
        coefficient >= 1.0
    }
}