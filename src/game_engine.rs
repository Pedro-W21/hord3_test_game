use std::{path::PathBuf, sync::{atomic::AtomicUsize, Arc, RwLock}};

use engine_derive::GameEngine;
use hord3::{defaults::default_rendering::vectorinator_binned::{rendering_spaces::ViewportData, shaders::NoOpShader, Vectorinator}, horde::{game_engine::{engine::{GameEngine, MovingObjectID}, entity::{Entity, EntityVec, MultiplayerEntity, Renderable}, multiplayer::Identify, world::{WorldComputeHandler, WorldHandler, WorldOutHandler, WorldWriteHandler}}, geometry::vec3d::{Vec3D, Vec3Df}, rendering::camera::Camera, scheduler::IndividualTask, sound::{ARWWaves, WavesHandler}}};

use crate::{colliders::AABB, cutscene::{game_shader::GameShader, reverse_camera_coords::reverse_from_raster_to_worldpos}, game_entity::{ColliderEvent, ColliderEventVariant, GameEntity, GameEntityVecRead, GameEntityVecWrite, MovementEvent, MovementEventVariant}, game_map::{get_voxel_pos, GameMap, GameMapEvent, Voxel, VoxelLight, VoxelModel, VoxelType}};


#[derive(Clone)]
pub struct CoolVoxel {
    pub voxel_type:u16,
    pub orient:u8,
    pub light:VoxelLight,
    pub extra_voxel_data:Option<Box<Vec<ExtraVoxelData>>>,
}

#[derive(Clone)]

/// A passage can be
/// - flood-opened (all adjacent same passage)
/// - tied to a key
/// - a corridor opening (may be pre-opened by generation)
/// - an explicit point of entry into the room (only link to exits from others (corridor=false && entry=false) or corridors)
pub struct PassageData {
    open_with_adjacent:bool,
    key_id:u16,
    possible_corridor:bool,
    is_entry:bool,
}

/// A trap is 
/// - activated one way (or periodically activated automatically)
/// - does a trap action at a position in a direction
/// - can have a cooldown, or be single use
/// - can activate with other adjacent traps or not
#[derive(Clone)]
pub struct TrapData {
    activation_type:ActivationType,
    action:TrapAction,
    cooldown:TrapCooldown,
    activate_with_all_adjacent:bool
}
#[derive(Clone)]
pub enum TrapCooldown {
    SingleUse{activated:bool},
    Ticks{max:usize, current:usize}
}
#[derive(Clone)]
pub enum TrapAction {
    Projectile {},
    StraightDamage {hitbox:AABB},
    Effect {}
}

#[derive(Clone)]
pub enum ActivationType {
    AnyEntityContact,
    ProjectileContact,
    MonsterContact,
    PlayerContact,
    PlayerInteract,
    Periodic(usize)
}

#[derive(Clone)]
pub enum ExtraVoxelData {
    IsPassage(PassageData),
    IsLightSource(VoxelLight),
    IsTrap(TrapData)
    
}

impl CoolVoxel {
    pub fn new(voxel_type:u16, orient:u8, light:VoxelLight, extra_voxel_data:Option<Box<Vec<ExtraVoxelData>>>) -> Self {
        Self { voxel_type, orient, light, extra_voxel_data }
    }
}

#[derive(Clone)]
pub struct CoolVoxelType {
    pub empty_sides:u8,
    pub texture:usize,
    pub light_passthrough:VoxelLight,
    pub is_light_source:Option<VoxelLight>,
    pub name:String,
    pub texture_path:Option<PathBuf>,
    pub base_extra_voxel_data:Option<ExtraVoxelData>
}

impl CoolVoxelType {
    pub fn new(empty_sides:u8, texture:usize, light_passthrough:VoxelLight, is_light_source:Option<VoxelLight>, name:String, texture_path:Option<PathBuf>, base_extra_voxel_data:Option<ExtraVoxelData>) -> Self {
        Self { empty_sides, texture, light_passthrough, is_light_source, name, texture_path, base_extra_voxel_data }
    }
}

impl VoxelType for CoolVoxelType {
    fn easy_texture(&self) -> usize {
        self.texture
    }
    fn sides_empty(&self) -> u8 {
        self.empty_sides
    }
    fn vertices_taken(&self) -> u8 {
        0
    }
    fn kind_of_model(&self) -> crate::game_map::VoxelModel {
        VoxelModel::WrappedTexture(self.texture)
    }
    fn light_passthrough(&self) -> VoxelLight {
        self.light_passthrough.clone()
    }
}

impl Voxel for CoolVoxel {
    type VT = CoolVoxelType;
    fn voxel_id(&self) -> usize {
        self.voxel_type as usize
    }
    fn orientation(&self) -> u8 {
        self.orient
    }
    fn light_level(&self) -> crate::game_map::VoxelLight {
        self.light
    }
}

fn get_push_to_next_integer_coords_in_dir(start:Vec3Df, dir:Vec3Df) -> Vec3Df {
    
    let push_to_x = if dir.x.is_sign_negative() {
        if start.x.is_sign_negative() {
            -start.x.fract()
        }
        else {
            1.0 - start.x.fract()
        }
    }
    else if dir.x != 0.0 {
        if start.x.is_sign_negative() {
            -(1.0 + start.x.fract())
        }
        else {
            -start.x.fract()
        }
    }
    else {
        0.0
    };
    let push_to_y = if dir.y.is_sign_negative() {
        if start.y.is_sign_negative() {
            -start.y.fract()
        }
        else {
            1.0 - start.y.fract()
        }
    }
    else if dir.y != 0.0 {
        if start.y.is_sign_negative() {
            -(1.0 + start.y.fract())
        }
        else {
            -start.y.fract()
        }
    }
    else {
        0.0
    };
    let push_to_z = if dir.z.is_sign_negative() {
        if start.z.is_sign_negative() {
            -start.z.fract()
        }
        else {
            1.0 - start.z.fract()
        }
        
    }
    else if dir.z != 0.0 {
        if start.z.is_sign_negative() {
            -(1.0 + start.z.fract())
        }
        else {
            -start.z.fract()
        }
    }
    else {
        0.0
    };
    Vec3Df::new(push_to_x, push_to_y, push_to_z)
}

fn get_push_to_next_integer_coords_in_dir_with_world(start:Vec3Df, dir:Vec3Df, world:&WorldComputeHandler<GameMap<CoolVoxel>, CoolGameEngineTID>) -> Option<Vec3Df> {
    let test_pos = start + dir;
    match world.world.get_voxel_at(get_voxel_pos(test_pos)) {
        Some(voxel) => {    
            if !world.world.get_voxel_types()[voxel.voxel_type as usize].is_completely_empty() {
                Some(get_push_to_next_integer_coords_in_dir(test_pos, dir))
            }
            else {
                None
            }
        },
        None => {
            Some(get_push_to_next_integer_coords_in_dir(test_pos, dir))
        }
    }
}

const GRAVITY:f32 = 9.81/180.0;
const AIR_RESISTANCE:f32 = 0.99;
const DOWN_DIR:Vec3Df = Vec3Df::new(0.0,0.0, -0.5);
const OTHER_DIRS:[Vec3Df ; 5] = [
    Vec3Df::new(0.0,0.0, 0.5),
    Vec3Df::new(0.0,0.5, 0.0),
    Vec3Df::new(0.0,-0.5, 0.0),
    Vec3Df::new(0.5,0.0, 0.0),
    Vec3Df::new(-0.5,0.0, 0.0),
]; 

fn compute_tick<'a>(turn:EntityTurn, id:usize, first_ent:&GameEntityVecRead<'a, CoolGameEngineTID>, second_ent:&GameEntityVecRead<'a, CoolGameEngineTID>, world:&WorldComputeHandler<GameMap<CoolVoxel>, CoolGameEngineTID>, extra_data:&ExtraData) {
    match turn {
        EntityTurn::entity_1 => {
            let movement = &first_ent.movement[id];
            let mut total_push = Vec3D::zero();
            for i in world.world.set_grid.get_iter_from_to(movement.pos, movement.pos + movement.speed, 2, 1.0) {
                if i != id {
                    let other_move = &first_ent.movement[i];
                    let distance = other_move.pos.dist(&movement.pos);
                    if distance < 0.5 {
                        total_push += (movement.pos - other_move.pos) * ((0.5 - distance) * 2.0)
                    }
                }
            }
            first_ent.tunnels.movement_out.send(MovementEvent::new(id, None, MovementEventVariant::UpdateSpeed(movement.speed + total_push)));
            
        },
        EntityTurn::entity_2 => {

        }
    }
}

fn after_main_tick<'a>(turn:EntityTurn, id:usize, first_ent:&GameEntityVecRead<'a, CoolGameEngineTID>, second_ent:&GameEntityVecRead<'a, CoolGameEngineTID>, world:&WorldComputeHandler<GameMap<CoolVoxel>, CoolGameEngineTID>, extra_data:&ExtraData) {
    match turn {
        EntityTurn::entity_1 => {
            

            let movement = &first_ent.movement[id];
            let collider = &first_ent.collider[id];
            let static_type = &first_ent.static_types[first_ent.stats[id].static_type_id];
            let mut movement_pos = movement.pos;
            //let mut movement_add = Vec3D::zero();
            let mut spd = movement.speed;
            spd *= AIR_RESISTANCE;
            let mut touching_ground = false;
            match get_push_to_next_integer_coords_in_dir_with_world(movement_pos + spd + DOWN_DIR, DOWN_DIR, world) {
                Some(push) => {
                    spd.z = spd.z.clamp(-0.5, 0.5);
                    touching_ground = true;
                    movement_pos += push;
                },
                None => ()
            }
            for dir in OTHER_DIRS {
                match get_push_to_next_integer_coords_in_dir_with_world(movement_pos + spd + dir, dir, world) {
                    Some(push) => {
                        //spd.z = 0.0;
                        movement_pos += push;
                    },
                    None => ()
                }
            }
            /*for arete in collider.collider.get_ground_vertices() {
                match world.world.get_voxel_at(get_voxel_pos(arete)) {
                    Some(voxel) => {    
                        if !world.world.get_voxel_types()[voxel.voxel_type as usize].is_completely_empty() {
                            touching_ground = true;
                            spd.z = spd.z.abs();
                            spd += movement.pos - arete;
                        }
                    },
                    None => {
                        spd += movement.pos - arete;
                        spd.z = spd.z.abs();
                        touching_ground = true;
                    }
                }
            }
            for arete in collider.collider.get_top_vertices() {
                match world.world.get_voxel_at(get_voxel_pos(arete)) {
                    Some(voxel) => {    
                        if !world.world.get_voxel_types()[voxel.voxel_type as usize].is_completely_empty() {
                            spd +=  movement.pos - arete;
                        }
                    },
                    None => {
                        spd += movement.pos - arete;
                    }
                }
            }*/
            match world.world.get_type_of_voxel_at(get_voxel_pos((movement_pos + spd + DOWN_DIR /*+ Vec3D::new(0.0, 0.0, -GRAVITY)*/))) {
                Some(voxel_type) => if !touching_ground && voxel_type.is_completely_empty() {
                    spd.z -= GRAVITY;
                }
                else {

                },
                None => if !touching_ground {
                    spd.z -= GRAVITY;
                }
            }
            match world.world.set_grid.get_point_move_update(&movement.pos, &(movement_pos + spd), id, 2) {
                Some(update) => {
                    //dbg!(update.clone());
                    world.tunnels.send_event(GameMapEvent::UpdateSetGrid(update))
                },
                None => ()
            }
            first_ent.tunnels.movement_out.send(MovementEvent::new(id, None, MovementEventVariant::UpdatePos(movement_pos + spd)));
            first_ent.tunnels.movement_out.send(MovementEvent::new(id, None, MovementEventVariant::UpdateSpeed(spd)));
            first_ent.tunnels.collider_out.send(ColliderEvent::new(id, None, ColliderEventVariant::UpdateCollider(static_type.collider.init_aabb + (movement_pos + spd ))));
            
        },
        _ => ()
    }
}

#[derive(Clone)]
pub struct ExtraData {
    pub tick:Arc<AtomicUsize>,
    pub waves:WavesHandler<CoolGameEngine>,
    pub current_render_data:Arc<RwLock<(Camera, ViewportData)>>
}

#[derive(GameEngine, Clone)]
#[rendering_engine = "Vectorinator"]
#[rendering_engine_generic = "GameShader"]
pub struct CoolGameEngine {
    entity_1:GameEntity,
    entity_2:GameEntity,
    world:GameMap<CoolVoxel>,
    #[extra_data]
    extra_data:ExtraData
}