use std::time::{Duration, Instant};

use hord3::horde::{geometry::{rotation::Orientation, vec3d::Vec3Df}, rendering::camera::Camera};

#[derive(Clone)]
pub enum CameraMovementElement {
    MoveFromToLinear {from:Vec3Df, to:Vec3Df},
    RotateFromToLinear {from:Orientation, to:Orientation},
    RotationShake {ranges:Orientation},
    MovementShake {ranges:Vec3Df},
    ChangeFOV {from:f32, to:f32},
    PointAt {position:Vec3Df},
    ConstantOrientChange {change:Orientation},
    StayPut,
}


#[derive(Clone)]
pub enum CameraMovementDuration {
    Ticks{number:usize},
    RealTime{duration:Duration}
}

impl CameraMovementDuration {
    pub fn from_start(&self, start:StartTime) -> Self {
        match start {
            StartTime::Tick { number } => match self {
                CameraMovementDuration::Ticks { number:number_elapsed } => Self::Ticks { number: *number_elapsed + 1 },
                _ => panic!("NOT TICKS"),
            },
            StartTime::RealTime { instant } => match self {
                CameraMovementDuration::RealTime { duration } => Self::RealTime { duration: instant.elapsed() },
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

pub struct CameraSequence {
    camera_movements:Vec<CameraMovement>,
    current_sequence_number:usize,
    current_movement:Option<CameraMovement>,
    movement_start_time:StartTime,
    duration_since_start_time:CameraMovementDuration
}

impl CameraSequence {
    pub fn advance_sequence(&mut self) -> Option<Camera> {
        self.duration_since_start_time = self.duration_since_start_time.from_start(self.movement_start_time.clone());
        match &self.current_movement {
            Some(movement) => {
                if movement.reached_total_time(self.duration_since_start_time.clone()) {
                    self.current_sequence_number += 1;
                    self.current_movement = self.camera_movements.get(self.current_sequence_number).cloned();
                    match &self.current_movement {
                        Some(movement) => {
                            match movement.duration {
                                CameraMovementDuration::RealTime { duration } => {
                                    self.movement_start_time = StartTime::RealTime { instant: Instant::now() };
                                    self.duration_since_start_time = CameraMovementDuration::RealTime { duration: Duration::from_secs_f32(0.0) };
                                },
                                CameraMovementDuration::Ticks { number } => {
                                    self.movement_start_time = StartTime::Tick { number: 0 };
                                    self.duration_since_start_time = CameraMovementDuration::Ticks { number: 0 }
                                }
                            }
                            Some(movement.get_position_and_rotation(self.duration_since_start_time.clone()))
                        },
                        None => None
                    }
                }
                else {
                    Some(movement.get_position_and_rotation(self.duration_since_start_time.clone()))
                }
            },
            None => {
                if self.current_sequence_number < self.camera_movements.len() {
                    self.current_movement = self.camera_movements.get(self.current_sequence_number).cloned();
                    match &self.current_movement {
                        Some(movement) => {
                            match movement.duration {
                                CameraMovementDuration::RealTime { duration } => {
                                    self.movement_start_time = StartTime::RealTime { instant: Instant::now() };
                                    self.duration_since_start_time = CameraMovementDuration::RealTime { duration: Duration::from_secs_f32(0.0) };
                                },
                                CameraMovementDuration::Ticks { number } => {
                                    self.movement_start_time = StartTime::Tick { number: 0 };
                                    self.duration_since_start_time = CameraMovementDuration::Ticks { number: 0 }
                                }
                            }
                            Some(movement.get_position_and_rotation(self.duration_since_start_time.clone()))
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
    pub fn new(movements:Vec<CameraMovement>) -> Self {
        Self { camera_movements:movements, current_sequence_number:0, current_movement:None, movement_start_time:StartTime::Tick { number: 0 }, duration_since_start_time: CameraMovementDuration::Ticks { number: 0 }}
    }
}

#[derive(Clone)]
pub struct CameraMovement {
    elements:Vec<CameraMovementElement>,
    duration:CameraMovementDuration,
}

impl CameraMovement {
    pub fn new(elements:Vec<CameraMovementElement>, duration:CameraMovementDuration) -> Self {
        Self { elements, duration }
    }
    pub fn get_position_and_rotation(&self, time_since_started:CameraMovementDuration) -> Camera {
        let mut coefficient = match time_since_started {
            CameraMovementDuration::Ticks { number } => match self.duration {
                CameraMovementDuration::Ticks { number:number_total } => (number as f32)/(number_total as f32),
                _ => panic!("NOT POSSIBLE"), 
            },
            CameraMovementDuration::RealTime { duration } => match self.duration {
                CameraMovementDuration::RealTime { duration:duration_total } => duration.as_secs_f32()/duration_total.as_secs_f32(),
                _ => panic!("NOT POSSIBLE EITHER"),
            },
        };
        if coefficient < 1.0 {
            let mut cam = Camera::empty();
            for element in &self.elements {
                match element {
                    CameraMovementElement::MoveFromToLinear { from, to } => {
                        cam.pos = from + (to - from) * coefficient;
                    },
                    CameraMovementElement::RotateFromToLinear { from, to } => {
                        cam.orient = *from + ((to - from) * coefficient);
                    },
                    CameraMovementElement::MovementShake { ranges } => {
                        cam.pos += Vec3Df::new(ranges.x * ((fastrand::f32() - 0.5) * 2.0),ranges.y * ((fastrand::f32() - 0.5) * 2.0),ranges.z * ((fastrand::f32() - 0.5) * 2.0))
                    },
                    CameraMovementElement::RotationShake { ranges } => {
                        cam.orient += Orientation::new(ranges.yaw * ((fastrand::f32() - 0.5) * 2.0),ranges.pitch * ((fastrand::f32() - 0.5) * 2.0),ranges.roll * ((fastrand::f32() - 0.5) * 2.0))
                    },
                    CameraMovementElement::PointAt { position } => {
                        cam.orient = Orientation::from_to(cam.pos, *position);
                    },
                    CameraMovementElement::ConstantOrientChange { change } => {
                        cam.orient += change.clone();
                    },
                    _ => ()
                }
            }
            cam
        }
        else {
            let mut cam = Camera::empty();
            for element in &self.elements {
                match element {
                    CameraMovementElement::MoveFromToLinear { from, to } => {
                        cam.pos = *to;
                    },
                    CameraMovementElement::RotateFromToLinear { from, to } => {
                        cam.orient = *to;
                    },
                    CameraMovementElement::PointAt { position } => {
                        cam.orient = Orientation::from_to(cam.pos, *position);
                    },
                    CameraMovementElement::ConstantOrientChange { change } => {
                        cam.orient += change.clone();
                    },
                    _ => ()
                }
            }
            cam
        }
        
    }
    pub fn reached_total_time(&self, time_since_started:CameraMovementDuration) -> bool {
        let mut coefficient = match time_since_started {
            CameraMovementDuration::Ticks { number } => match self.duration {
                CameraMovementDuration::Ticks { number:number_total } => (number as f32)/(number_total as f32),
                _ => panic!("NOT POSSIBLE"), 
            },
            CameraMovementDuration::RealTime { duration } => match self.duration {
                CameraMovementDuration::RealTime { duration:duration_total } => duration.as_secs_f32()/duration_total.as_secs_f32(),
                _ => panic!("NOT POSSIBLE EITHER"),
            },
        };
        coefficient >= 1.0
    }
}