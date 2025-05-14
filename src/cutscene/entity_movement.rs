use std::time::{Duration, Instant};

use hord3::horde::{game_engine::multiplayer::Identify, geometry::{rotation::Orientation, vec3d::Vec3Df}};

#[derive(Debug, Clone, Copy)]
pub struct EntityPos {
    pub pos:Vec3Df,
    pub orient:Orientation
}

impl EntityPos {
    pub fn empty() -> Self {
        Self { pos: Vec3Df::zero(), orient: Orientation::zero() }
    }
}

#[derive(Clone)]
pub enum EntityMovementElement {
    MoveFromToLinear {from:Vec3Df, to:Vec3Df},
    RotateFromToLinear {from:Orientation, to:Orientation},
    RotationShake {ranges:Orientation},
    MovementShake {ranges:Vec3Df},
    PointAt {position:Vec3Df},
    StayAt {position:Vec3Df},
    HoldOrient {orient:Orientation},
    ConstantOrientChange {change:Orientation},
    StayPut,
}


#[derive(Clone)]
pub enum EntityMovementDuration {
    Ticks{number:usize},
    RealTime{duration:Duration}
}

impl EntityMovementDuration {
    pub fn from_start(&self, start:StartTime) -> Self {
        match start {
            StartTime::Tick { number } => match self {
                EntityMovementDuration::Ticks { number:number_elapsed } => Self::Ticks { number: *number_elapsed + 1 },
                _ => panic!("NOT TICKS"),
            },
            StartTime::RealTime { instant } => match self {
                EntityMovementDuration::RealTime { duration } => Self::RealTime { duration: instant.elapsed() },
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

pub struct EntitySequence<ID:Identify> {
    entity_movements:Vec<EntityMovement>,
    current_sequence_number:usize,
    current_movement:Option<EntityMovement>,
    movement_start_time:StartTime,
    duration_since_start_time:EntityMovementDuration,
    pub entity_id:ID
}

impl<ID:Identify> EntitySequence<ID> {
    pub fn advance_sequence(&mut self) -> Option<EntityPos> {
        self.duration_since_start_time = self.duration_since_start_time.from_start(self.movement_start_time.clone());
        match &self.current_movement {
            Some(movement) => {
                if movement.reached_total_time(self.duration_since_start_time.clone()) {
                    self.current_sequence_number += 1;
                    self.current_movement = self.entity_movements.get(self.current_sequence_number).cloned();
                    match &self.current_movement {
                        Some(movement) => {
                            match movement.duration {
                                EntityMovementDuration::RealTime { duration } => {
                                    self.movement_start_time = StartTime::RealTime { instant: Instant::now() };
                                    self.duration_since_start_time = EntityMovementDuration::RealTime { duration: Duration::from_secs_f32(0.0) };
                                },
                                EntityMovementDuration::Ticks { number } => {
                                    self.movement_start_time = StartTime::Tick { number: 0 };
                                    self.duration_since_start_time = EntityMovementDuration::Ticks { number: 0 }
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
                if self.current_sequence_number < self.entity_movements.len() {
                    self.current_movement = self.entity_movements.get(self.current_sequence_number).cloned();
                    match &self.current_movement {
                        Some(movement) => {
                            match movement.duration {
                                EntityMovementDuration::RealTime { duration } => {
                                    self.movement_start_time = StartTime::RealTime { instant: Instant::now() };
                                    self.duration_since_start_time = EntityMovementDuration::RealTime { duration: Duration::from_secs_f32(0.0) };
                                },
                                EntityMovementDuration::Ticks { number } => {
                                    self.movement_start_time = StartTime::Tick { number: 0 };
                                    self.duration_since_start_time = EntityMovementDuration::Ticks { number: 0 }
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
    pub fn new(movements:Vec<EntityMovement>, for_entity:ID) -> Self {
        Self {entity_id:for_entity, entity_movements:movements, current_sequence_number:0, current_movement:None, movement_start_time:StartTime::Tick { number: 0 }, duration_since_start_time: EntityMovementDuration::Ticks { number: 0 }}
    }
}
#[derive(Clone)]
pub struct EntityMovement {
    elements:Vec<EntityMovementElement>,
    duration:EntityMovementDuration,
}

impl EntityMovement {
    pub fn new(elements:Vec<EntityMovementElement>, duration:EntityMovementDuration) -> Self {
        Self { elements, duration }
    }
    pub fn get_position_and_rotation(&self, time_since_started:EntityMovementDuration) -> EntityPos {
        let mut coefficient = match time_since_started {
            EntityMovementDuration::Ticks { number } => match self.duration {
                EntityMovementDuration::Ticks { number:number_total } => (number as f32)/(number_total as f32),
                _ => panic!("NOT POSSIBLE"), 
            },
            EntityMovementDuration::RealTime { duration } => match self.duration {
                EntityMovementDuration::RealTime { duration:duration_total } => duration.as_secs_f32()/duration_total.as_secs_f32(),
                _ => panic!("NOT POSSIBLE EITHER"),
            },
        };
        if coefficient < 1.0 {
            let mut cam = EntityPos::empty();
            for element in &self.elements {
                match element {
                    EntityMovementElement::MoveFromToLinear { from, to } => {
                        cam.pos = from + (to - from) * coefficient;
                    },
                    EntityMovementElement::RotateFromToLinear { from, to } => {
                        cam.orient = *from + ((to - from) * coefficient);
                    },
                    EntityMovementElement::MovementShake { ranges } => {
                        cam.pos += Vec3Df::new(ranges.x * ((fastrand::f32() - 0.5) * 2.0),ranges.y * ((fastrand::f32() - 0.5) * 2.0),ranges.z * ((fastrand::f32() - 0.5) * 2.0))
                    },
                    EntityMovementElement::RotationShake { ranges } => {
                        cam.orient += Orientation::new(ranges.yaw * ((fastrand::f32() - 0.5) * 2.0),ranges.pitch * ((fastrand::f32() - 0.5) * 2.0),ranges.roll * ((fastrand::f32() - 0.5) * 2.0))
                    },
                    EntityMovementElement::PointAt { position } => {
                        cam.orient = Orientation::from_to(cam.pos, *position);
                    },
                    EntityMovementElement::StayAt { position } => {
                        cam.pos = *position;
                    },
                    EntityMovementElement::HoldOrient { orient } => {
                        cam.orient = *orient;
                    }
                    EntityMovementElement::ConstantOrientChange { change } => {
                        cam.orient += change.clone();
                    },
                    _ => ()
                }
            }
            cam
        }
        else {
            let mut cam = EntityPos::empty();
            for element in &self.elements {
                match element {
                    EntityMovementElement::MoveFromToLinear { from, to } => {
                        cam.pos = *to;
                    },
                    EntityMovementElement::RotateFromToLinear { from, to } => {
                        cam.orient = *to;
                    },
                    EntityMovementElement::PointAt { position } => {
                        cam.orient = Orientation::from_to(cam.pos, *position);
                    },
                    EntityMovementElement::ConstantOrientChange { change } => {
                        cam.orient += change.clone();
                    },
                    _ => ()
                }
            }
            cam
        }
        
    }
    pub fn reached_total_time(&self, time_since_started:EntityMovementDuration) -> bool {
        let mut coefficient = match time_since_started {
            EntityMovementDuration::Ticks { number } => match self.duration {
                EntityMovementDuration::Ticks { number:number_total } => (number as f32)/(number_total as f32),
                _ => panic!("NOT POSSIBLE"), 
            },
            EntityMovementDuration::RealTime { duration } => match self.duration {
                EntityMovementDuration::RealTime { duration:duration_total } => duration.as_secs_f32()/duration_total.as_secs_f32(),
                _ => panic!("NOT POSSIBLE EITHER"),
            },
        };
        coefficient >= 1.0
    }
}