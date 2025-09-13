#![feature(portable_simd)]
#![feature(int_roundings)]
#![feature(mpmc_channel)]
use std::{collections::HashMap, f32::consts::PI, path::PathBuf, simd::Simd, sync::{atomic::{AtomicUsize, Ordering}, mpmc, Arc, RwLock}, thread, time::{Duration, Instant}};

use game_entity::colliders::AABB;
use cosmic_text::{Color, Font, Metrics};
use crossbeam::channel::unbounded;
use cutscene::{camera_movement::{CameraMovement, CameraMovementDuration, CameraMovementElement, CameraSequence}, demo_cutscene::{get_demo_cutscene, get_empty_cutscene}, game_shader::GameShader, real_demo_cutscene::get_real_demo_cutscene, write_in_the_air::get_positions_of_air_written_text, written_texture::get_written_texture_buffer};
use day_night::DayNight;
use game_3d_models::{clustered_ent_mesh, grey_sphere_mesh, lit_selection_cube, second_spread_out_ent_mesh, simple_line, sphere_mesh, spread_out_ent_mesh, textured_sphere_mesh, wireframe_sphere_mesh, xyz_mesh};
use game_engine::{CoolGameEngineBase, CoolVoxel, CoolVoxelType, ExtraData};
use game_entity::{Collider, GameEntityVec, Movement, NewGameEntity, StaticCollider, StaticGameEntity, StaticMeshInfo, StaticMovement, StaticStats, Stats};
use game_input_handler::GameInputHandler;
use game_map::{get_f64_pos, get_float_pos, light_spreader::{LightPos, LightSpread}, ChunkDims, GameMap, VoxelLight};
use game_tasks::{GameTask, GameTaskTaskHandler, GameUserEvent};
use gui_elements::{list_choice::get_list_choice, number_config::get_number_config};
use hord3::{defaults::{default_frontends::minifb_frontend::MiniFBWindow, default_rendering::vectorinator_binned::{meshes::{Mesh, MeshID, MeshLODS, MeshLODType}, rendering_spaces::ViewportData, shaders::NoOpShader, textures::{argb_to_rgb, rgb_to_argb, TextureSetID}, triangles::{color_u32_to_u8_simd, simd_rgb_to_argb}, Vectorinator}, default_ui::simple_ui::{SimpleUI, UIDimensions, UIElement, UIElementBackground, UIElementContent, UIElementID, UIEvent, UIUnit, UIUserAction, UIVector}}, horde::{frontend::{HordeWindowDimensions, WindowingHandler}, game_engine::{entity::Renderable, world::{WorldComputeHandler, WorldHandler}}, geometry::{plane::EquationPlane, rotation::{Orientation, Rotation}, vec3d::{Vec3D, Vec3Df}}, rendering::{camera::Camera, framebuffer::HordeColorFormat}, scheduler::{HordeScheduler, HordeTaskQueue, HordeTaskSequence, SequencedTask}, sound::{SoundRequest, WaveIdentification, WavePosition, WaveRequest, WaveSink, Waves}}};
use noise::{NoiseFn, Perlin, Seedable};
use tile_editor::{get_tile_voxels, TileEditorData};

use crate::{game_entity::{actions::{Action, ActionKind, ActionSource, ActionTimer, ActionsEvent, ActionsUpdate, StaticGameActions}, director::{llm_director::LLMDirector, Director, DirectorKind, StaticDirector}, planner::StaticPlanner}, game_map::get_voxel_pos, proxima_link::ProximaLink};

pub mod game_map;
pub mod flat_game_map;
pub mod game_entity;
pub mod game_engine;
pub mod game_tasks;
pub mod game_input_handler;
pub mod tile_editor;
pub mod gui_elements;
pub mod game_3d_models;
pub mod game_tiles;
pub mod cutscene;
pub mod day_night;
pub mod proxima_link;

fn main() {
    let mut world = GameMap::new(100, ChunkDims::new(8, 8, 8), get_tile_voxels(), (255,255,255), 1);
    let mut perlin = Perlin::new().set_seed(13095);
    let mut world_height = 15.0;
    let mut water_level = 10.0;
    let start = Vec3D::new(-30, -20, -2);
    let end = Vec3D::new(20, 20, 30);

    let mut ground_at = vec![0; ((end.x - start.x) * 8 * (end.y - start.y) * 8) as usize];
    let length_f64 = ((end.x - start.x) * 8 ) as f64;
    world.generate_chunks(start, end, &mut |pos| {
        let pos_3D = (get_f64_pos(pos) * 0.01);
        let value_2D = (perlin.get([pos_3D.x, pos_3D.y]) + 1.0) * 0.5;
        let local_world_height = world_height - (((pos.x - start.x) * 8) as f64/length_f64) * world_height;
        let actual_height = local_world_height + world_height * value_2D * 2.0;
        if (pos.z as f64) < actual_height || (pos.z as f64) < water_level {
            let ground_pos = (pos.x - (start.x * 8) + (pos.y - (start.y * 8)) * ((end.y - start.y) * 8)) as usize;
            if (pos.z as f64) < water_level {
                if ground_at[ground_pos] < pos.z {
                    ground_at[ground_pos] = water_level as i32;
                }
                CoolVoxel {voxel_type:7, orient:0, light:VoxelLight::random_light(), extra_voxel_data:None}
            }
            else {
                if ground_at[ground_pos] < pos.z {
                    ground_at[ground_pos] = pos.z;
                }
                CoolVoxel {voxel_type:1 + ((actual_height - water_level)/(6.0*world_height * (1.0/6.0))).clamp(0.0, 5.99) as u16, orient:0, light:VoxelLight::random_light(), extra_voxel_data:None}
            }
        } else {
            CoolVoxel {voxel_type:0, orient:0, light:VoxelLight::zero_light(), extra_voxel_data:None}
        }
    }
    );
    let mut world_clone = world.clone();
    let mut spare_world = world.clone();
    {
        world_clone.change_mesh_vec(10);
        world_clone.set_min_light_levels((50,50,50));
        for i in 0..1 {
            let (x,y) = (fastrand::i32((start.x * 8)..(end.x * 8)), fastrand::i32((start.y * 8)..(end.y * 8)));
            let light_source = LightPos::new(Vec3D::new(x, y, ground_at[(x - (start.x * 8) + (y - (start.y * 8)) * ((end.y - start.y) * 8)) as usize] + 1), VoxelLight::slightly_less_random_light());
            let total_light_spread = LightSpread::calc_max_spread(&world_clone, light_source).get_all_spread();
            for light_pos in total_light_spread {
                world_clone.get_voxel_at_mut(light_pos.pos()).unwrap().light = light_pos.value().merge_with_other(&world_clone.get_voxel_at(light_pos.pos()).unwrap().light);
            }
            println!("light {i} done !");
        }
    }
    
    let entity_vec = GameEntityVec::new(1000);
    {
        let mut writer = entity_vec.get_write();
        writer.new_sct(StaticGameEntity{planner:StaticPlanner{},director:StaticDirector {kind:DirectorKind::Nothing},actions:StaticGameActions {base_actions:Vec::with_capacity(8)},movement:StaticMovement{}, mesh_info:StaticMeshInfo{mesh_id:MeshID::Named("EntityMesh".to_string()),mesh_data:Mesh::new(MeshLODS::new(vec![MeshLODType::Mesh(Arc::new(simple_line(-Vec3D::all_ones()*0.5, Vec3D::all_ones()*0.5, 2, (255,255,255))))]), "EntityMesh".to_string(), 2.0)}, stats:StaticStats{}, collider:StaticCollider{init_aabb:AABB::new(-Vec3D::all_ones()*0.5, Vec3D::all_ones()*0.5)}});

        writer.new_sct(StaticGameEntity{planner:StaticPlanner{},director:StaticDirector {kind:DirectorKind::Nothing},actions:StaticGameActions {base_actions:Vec::with_capacity(8)},movement:StaticMovement{}, mesh_info:StaticMeshInfo{mesh_id:MeshID::Named("GREY_MESH".to_string()),mesh_data:grey_sphere_mesh()}, stats:StaticStats{}, collider:StaticCollider{init_aabb:AABB::new(-Vec3D::all_ones()*0.5, Vec3D::all_ones()*0.5)}});


        let test_goals = vec![
            //format!("Build a vertical staircase and get on top of it"),
            //format!("Create a square structure on flat ground near you."),
            vec![
                format!("Meet up with all other agents in one place by agreeing on a place and moving there."),
                format!("Build a large wall with other agents.")
            ],
            /*vec![
                format!("Build a 5x5 flat square platform"),
            ],*/
            /*vec![
                format!("Make a 3x3 square hole that is 4 voxels deep next to you without falling into it")
            ]*/
            /*vec![
                format!("Find another agent, move towards them and dig a hole beneath them.")
            ],*/
            /*vec![
                format!("Create a house with 4 walls and a ceiling. The house must have at least a 3x3 empty space inside.")
            ],*/
            /*vec![
                format!("Create a staircase reaching at least 10 voxels tall (on the z axis) from your position. Keep in mind that a staircase is diagonal."),
                format!("Get on top of the staircase you created")
            ]*/
            /*vec![
                format!("Meet up with other agents and build a house together with enough space inside to fit everyone")
            ]*/
            //format!("Build a house with 4 walls, an entrance and a roof"),
            /*vec![
                format!("Find someone else in this world, and move to them. They may be far away.")
            ]*/
        ];

        for i in 0..10 {
            let pos = Vec3D::new((fastrand::f32() - 0.5) * 2.0 * 150.0, (fastrand::f32() - 0.5) * 2.0 * 150.0, 150.0);
            writer.new_ent(NewGameEntity::new(Movement{against_wall:false, touching_ground:false,pos:pos, speed:Vec3D::zero(), orient:Orientation::zero(), rotat:Rotation::from_orientation(Orientation::zero())}, Stats {static_type_id:1, health:0, damage:0, stamina:0, ground_speed:0.2, jump_height:1.0}, Collider{team:0, collider:AABB::new(pos - Vec3D::all_ones() * 0.5, pos + Vec3D::all_ones() * 0.5)}, Director::new_with_random_name(DirectorKind::LLM(LLMDirector::new_with_goals(fastrand::choice(test_goals.iter()).unwrap().clone())))));
            //writer.new_ent(NewGameEntity::new(Movement{against_wall:false, touching_ground:false,pos:pos, speed:Vec3D::zero(), orient:Orientation::zero(), rotat:Rotation::from_orientation(Orientation::zero())}, Stats {static_type_id:1, health:0, damage:0, stamina:0, ground_speed:0.2, jump_height:1.0}, Collider{team:0, collider:AABB::new(pos - Vec3D::all_ones() * 0.5, pos + Vec3D::all_ones() * 0.5)}, Director::new_with_random_name(DirectorKind::LLM(LLMDirector::new_with_goals(test_goals[i].clone())))));
        }

        let positions = get_positions_of_air_written_text("Hord3".to_string(), Metrics::new(100.0, 80.0), "don't_care".to_string(), 1000, 1000, Color(rgb_to_argb((255,255,255))), (0,0), Vec3D::new(0.0, -1.0, 0.0), Vec3D::new(0.01, 0.0, -1.0), Vec3D::new(-155.0, 155.0, 180.0));
        for pos in positions {
            //writer.new_ent(NewGameEntity::new(Movement{pos:pos, speed:Vec3D::new(1.0, 0.0, 0.0), orient:Orientation::zero(), rotat:Rotation::from_orientation(Orientation::zero())}, Stats {static_type_id:8, health:0, damage:0, stamina:0}, Collider{team:0, collider:AABB::new(pos - Vec3D::all_ones() * 0.5, pos + Vec3D::all_ones() * 0.5)}));
        }
    }

    let (payload_sender, response_receiver) = match ProximaLink::initialize(String::from("HORDE"), String::from("HORDE"), String::from("http://localhost:8085")) {
        Ok((s, r)) => (s, r),
        Err(_) => (mpmc::channel().0, mpmc::channel().1)
    };
    
    let entity_vec_2 = GameEntityVec::new(1000);
    
    let windowing = WindowingHandler::new::<MiniFBWindow>(HordeWindowDimensions::new(1280, 720), HordeColorFormat::ARGB8888);
    let framebuf = windowing.get_outside_framebuf();
    let mut shader = Arc::new(GameShader::new_default());
    let viewport_data = {
        let framebuf = framebuf.read().unwrap();
        ViewportData {
            near_clipping_plane: 1.0,
            half_image_width: (framebuf.get_dims().get_width()/2) as f32,
            half_image_height: (framebuf.get_dims().get_height()/2) as f32,
            aspect_ratio: (framebuf.get_dims().get_width() as f32)/(framebuf.get_dims().get_height() as f32),
            camera_plane: EquationPlane::new(Vec3D::new(0.0, 0.0, 1.0), -1.0),
            image_height: (framebuf.get_dims().get_height() as f32),
            image_width: (framebuf.get_dims().get_width() as f32),
            poscam: Vec3D::zero(),
            rotat_cam: Rotation::new_from_inverted_orient(Orientation::zero())
        }
    };
    let vectorinator = Vectorinator::new(framebuf.clone(), shader);
    let (waves, waves_handler, stream) = Waves::new(Vec::new(), 10);
    let world_handler = WorldHandler::new(world);
    
    let engine = CoolGameEngineBase::new(entity_vec, entity_vec_2, world_handler.clone(), Arc::new(vectorinator.clone()), ExtraData {payload_sender, tick: Arc::new(AtomicUsize::new(0)), waves:waves_handler.clone(), current_render_data:Arc::new(RwLock::new((Camera::empty(), viewport_data.clone())))});
    waves_handler.send_gec(engine.clone());
    let mouse = windowing.get_mouse_state();
    let mouse2 = windowing.get_mouse_state();

    let outside_events = windowing.get_outside_events();
    let (mut simpleui, user_events) = SimpleUI::new(20, 20, framebuf.clone(), mouse, unbounded().1);

    // TRES IMPORTANTTTTTTTTTTTTT
    simpleui.add_many_connected_elements(get_list_choice(vec!["TerrainModifier".to_string(), "TileChooser".to_string(), "TerrainZoneModifier".to_string(), "LightSpreader".to_string()], UIVector::new(UIUnit::ParentWidthProportion(0.9), UIUnit::ParentHeightProportion(0.3)), UIDimensions::Decided(UIVector::new(UIUnit::ParentWidthProportion(0.1), UIUnit::ParentHeightProportion(0.3))), "Tools".to_string(), "rien".to_string()));
    
    {
        println!("START TEXTURE");
        let mut writer = vectorinator.get_write();
        writer.textures.add_set_with_many_textures(
            "Testing_Texture".to_string(),
            vec![
                (
                    "neige.png".to_string(),
                    1,
                    None
                ),
            ]
        );
        writer.textures.add_set_with_many_textures(
            "Testing_Texture_2".to_string(),
            vec![
                (
                    "sable.png".to_string(),
                    1,
                    None
                ),
            ]
        );
        writer.textures.add_set_with_many_textures(
            "Testing_Texture_3".to_string(),
            vec![
                (
                    "terre_herbe.png".to_string(),
                    1,
                    None
                )
            ]
        );
        writer.textures.add_set_with_many_textures(
            "Testing_Texture_4".to_string(),
            vec![
                (
                    "terre_cail.png".to_string(),
                    1,
                    None
                )
            ]
        );
        writer.textures.add_set_with_many_textures(
            "Testing_Texture_5".to_string(),
            vec![
                (
                    "terre.png".to_string(),
                    1,
                    None
                )
            ]
        );
        writer.textures.add_set_with_many_textures(
            "Testing_Texture_6".to_string(),
            vec![
                (
                    "roche.png".to_string(),
                    1,
                    None
                )
            ]
        );
        writer.textures.add_set_with_many_textures(
            "Testing_Texture_7".to_string(),
            vec![
                (
                    "eau.png".to_string(),
                    1,
                    None
                )
            ]
        );
        writer.textures.add_set_with_many_textures(
            "Testing_Texture_8".to_string(),
            vec![
                (
                    "eau_prof.png".to_string(),
                    1,
                    None
                )
            ]
        );
        writer.textures.add_set_with_many_textures(
            "Testing_Texture_8".to_string(),
            vec![
                (
                    "metal_0.png".to_string(),
                    1,
                    None
                )
            ]
        );
        writer.textures.add_generated_texture_set("Testing_text_texture".to_string(), get_written_texture_buffer("TEST\nLOL".to_string(), Metrics::new(300.0, 310.0), "don't_care".to_string(), vec![rgb_to_argb((0,200,0)) ; 1000*1000], 1000, 1000, Color(rgb_to_argb((255,255,255))), (0,0)), 1000, 1000);
        writer.textures.add_generated_texture_set("FULLRED".to_string(), get_written_texture_buffer("".to_string(), Metrics::new(300.0, 310.0), "don't_care".to_string(), vec![rgb_to_argb((255,0,0)) ; 1000*1000], 1000, 1000, Color(rgb_to_argb((255,255,255))), (0,0)), 1000, 1000);
        writer.textures.add_generated_texture_set("FULLGREEN".to_string(), get_written_texture_buffer("".to_string(), Metrics::new(300.0, 310.0), "don't_care".to_string(), vec![rgb_to_argb((0,255,0)) ; 1000*1000], 1000, 1000, Color(rgb_to_argb((255,255,255))), (0,0)), 1000, 1000);
        writer.textures.add_generated_texture_set("FULLBLUE".to_string(), get_written_texture_buffer("".to_string(), Metrics::new(300.0, 310.0), "don't_care".to_string(), vec![rgb_to_argb((0,0,255)) ; 1000*1000], 1000, 1000, Color(rgb_to_argb((255,255,255))), (0,0)), 1000, 1000);
        writer.textures.add_generated_texture_set("FULLWHITE".to_string(), get_written_texture_buffer("".to_string(), Metrics::new(300.0, 310.0), "don't_care".to_string(), vec![rgb_to_argb((255,255,255)) ; 1000*1000], 1000, 1000, Color(rgb_to_argb((255,255,255))), (0,0)), 1000, 1000);
        let text_herbe = writer.textures.get_text_with_id(match writer.textures.get_id_with_name(&"Testing_Texture_3".to_string()).unwrap() {TextureSetID::ID(id) => id, _ => panic!()});
        let mut datas = Vec::with_capacity(200);
        for i in 0..200 {
            let mut new_data = text_herbe.get_mip_map(0).data.clone();
            let len = new_data.len();
            for j in 1..(text_herbe.get_mip_map(0).largeur_usize.pow(2) - (i* text_herbe.get_mip_map(0).largeur_usize.pow(2))/200) {
                new_data[len - j] = 0
            }
            datas.push(new_data);
        }
        writer.textures.add_generated_texture_multiset("RASTERSHOW".to_string(), datas, 16, 16, 1, Some((0,0,0)));
        writer.textures.add_generated_texture_set("FULLPINK".to_string(), get_written_texture_buffer("".to_string(), Metrics::new(300.0, 310.0), "don't_care".to_string(), vec![rgb_to_argb((255,0,255)) ; 1000*1000], 1000, 1000, Color(rgb_to_argb((255,255,255))), (0,0)), 1000, 1000);
        
        println!("DONE TEXTURE");
    }
    let handler = GameTaskTaskHandler::new(engine.clone(), windowing, vectorinator.clone(), simpleui.clone(), waves);
    
    let queue = HordeTaskQueue::new(vec![HordeTaskSequence::new(vec![

        SequencedTask::StartTask(GameTask::PrepareRendering),
        SequencedTask::WaitFor(GameTask::PrepareRendering),
        SequencedTask::StartSequence(1),
        SequencedTask::StartTask(GameTask::ApplyEvents),
        SequencedTask::StartTask(GameTask::UpdateSoundPositions),
        SequencedTask::WaitFor(GameTask::ApplyEvents),
        SequencedTask::StartTask(GameTask::Main),
        SequencedTask::WaitFor(GameTask::Main),
        SequencedTask::WaitFor(GameTask::UpdateSoundPositions),
        SequencedTask::StartTask(GameTask::UpdateSoundEverythingElse),
        SequencedTask::StartTask(GameTask::ApplyEvents),
        SequencedTask::WaitFor(GameTask::ApplyEvents),
        SequencedTask::StartTask(GameTask::AfterMain),
        SequencedTask::WaitFor(GameTask::AfterMain),
        SequencedTask::StartTask(GameTask::ApplyEvents),
        SequencedTask::WaitFor(GameTask::ApplyEvents),
        SequencedTask::WaitFor(GameTask::UpdateSoundEverythingElse),
        ]
    ),
    HordeTaskSequence::new(vec![
        SequencedTask::StartTask(GameTask::RenderEverything),
        SequencedTask::StartTask(GameTask::DoAllUIRead),
        SequencedTask::StartTask(GameTask::DoEventsAndMouse),
        //SequencedTask::StartTask(GameTask::ResetCounters),
        //SequencedTask::WaitFor(GameTask::ResetCounters),
        
        SequencedTask::WaitFor(GameTask::DoAllUIRead),
        SequencedTask::StartTask(GameTask::DoAllUIWrite),
        SequencedTask::WaitFor(GameTask::DoAllUIWrite),

        SequencedTask::WaitFor(GameTask::DoEventsAndMouse),
        SequencedTask::StartTask(GameTask::SendFramebuf),
        SequencedTask::WaitFor(GameTask::SendFramebuf),
        SequencedTask::StartTask(GameTask::WaitForPresent),
        SequencedTask::WaitFor(GameTask::WaitForPresent),

        SequencedTask::WaitFor(GameTask::RenderEverything),

        SequencedTask::StartTask(GameTask::ChangePhase),
        SequencedTask::StartTask(GameTask::ClearZbuf),

        SequencedTask::WaitFor(GameTask::ChangePhase),
        SequencedTask::StartTask(GameTask::ClearFramebuf),
        SequencedTask::StartTask(GameTask::TickAllSets),
        SequencedTask::WaitFor(GameTask::ClearZbuf),
        SequencedTask::WaitFor(GameTask::ClearFramebuf),
        SequencedTask::WaitFor(GameTask::TickAllSets),
        ]
    )], Vec::new());
    println!("Hello, world!");
    let mut scheduler = HordeScheduler::new(queue.clone(), handler, 16);
    let mut input_handler = GameInputHandler::new(mouse2.clone(), 3.0, outside_events);
    let mut tile_editor = TileEditorData::new(simpleui.clone(), input_handler.get_new_camera(), mouse2);
    {
        tile_editor.initial_ui_work(&vectorinator.get_texture_read());
    }
    println!("FINISHED INITIAL");
    /*let mut sequence = CameraSequence::new(vec![
        CameraMovement::new(
            vec![
                CameraMovementElement::MoveFromToLinear { from: Vec3Df::zero(), to: Vec3Df::all_ones() * 180.0 },
                CameraMovementElement::MovementShake { ranges: Vec3Df::all_ones() * 0.005 },
                CameraMovementElement::PointAt { position: Vec3Df::new(-10.0, 30.0, 10.0) },
                CameraMovementElement::ConstantOrientChange { change: Orientation::new(-PI/4.0, 0.0, 0.0) }
            ],
            CameraMovementDuration::RealTime { duration: Duration::from_secs_f32(2.0) }
        ),
        CameraMovement::new(
            vec![
                CameraMovementElement::MoveFromToLinear { from: Vec3Df::all_ones() * 80.0, to: Vec3Df::all_ones() * 80.0 },
                CameraMovementElement::RotateFromToLinear { from: Orientation::zero(), to: Orientation::new(0.0, 2.0, 0.0) }
            ],
            CameraMovementDuration::RealTime { duration: Duration::from_secs_f32(2.0) }
        ),
    ]);*/
    //let mut cutscene = get_demo_cutscene(&viewport_data);
    let mut day_night = DayNight::new(
        (148,236,255),
        (238,175,97),
        (19,24,98),
        
        Vec3Df::new(0.0, 1.0, -1.0),
        Vec3Df::new(0.0, 1.0, 0.0),
        Vec3Df::new(0.0, -1.0, 1.0),

        475
    );
    let mut prev_night_status = false;
    {
        let mut writer = vectorinator.get_write();
        world_clone.set_min_light_levels((50, 50, 50));
        world_clone.change_mesh_vec(10);
        world_clone.do_render_changes(&mut writer);
        world_clone.make_meshes_invisible(&mut writer);
    }   
    // let mut cutscene = get_real_demo_cutscene(&viewport_data);
    for i in 0..75000 {
        //println!("{i}");

        let mut start = Instant::now();
        input_handler.update_keyboard();
        let (new_fog_col, new_normal_vec, new_night_state) = day_night.get_next_color();
        //if prev_night_status != new_night_state {
        //    let mut writer = vectorinator.get_write();
        //    spare_world = world_handler.world.read().unwrap().clone();
        //    spare_world.make_meshes_invisible(&mut writer);
        //    world_clone.make_meshes_visible(&mut writer);
        //    world_clone.set_grid = spare_world.set_grid.clone();
        //    *world_handler.world.write().unwrap() = world_clone.clone();
        //}
        //prev_night_status = new_night_state;
        let new_camera = {
            let mut writer = vectorinator.get_write();
            //vectorinator.shader_data.do_normals.store(!new_night_state, Ordering::Relaxed);
            *vectorinator.shader_data.sun_dir.write().unwrap() = -new_normal_vec;
            *vectorinator.shader_data.fog_color.write().unwrap() = rgb_to_argb(new_fog_col);
            let new_camera =input_handler.get_new_camera();
            *writer.camera = new_camera.clone();//(i as f32 / 500.0) * PI/2.0));
            if i > 400 {
                let mut reader = engine.entity_1.get_read();

                let ent = fastrand::usize(0..reader.actions.len());
                let pos = Vec3D::new((fastrand::f32() - 0.5) * 2.0 * 150.0, (fastrand::f32() - 0.5) * 2.0 * 150.0, 50.0);
                let voxel_pos = get_voxel_pos(pos);
                let target_pos = engine.world.world.read().unwrap().get_ceiling_at(voxel_pos, 100) + Vec3D::new(0, 0, 1);
                let mut counter = reader.actions[ent].get_counter().clone();
                let next_action = counter.get_next_id();
                reader.tunnels.actions_out.send(ActionsEvent::new(ent, None, ActionsUpdate::AddAction(Action::new(next_action, engine.extra_data.tick.load(Ordering::Relaxed), ActionTimer::Delay(500), ActionKind::PathToPosition(Vec3Df::new(target_pos.x as f32, target_pos.y as f32, target_pos.z as f32), 0.7), ActionSource::Director))));
                reader.tunnels.actions_out.send(ActionsEvent::new(ent, None, ActionsUpdate::UpdateCounter(counter)));
            }
            // dbg!(new_camera.clone());
            engine.extra_data.current_render_data.write().unwrap().0 = new_camera.clone();
            engine.extra_data.tick.fetch_add(1, Ordering::Relaxed);

            /*thread::sleep(Duration::from_millis(10));*/
            new_camera
        };
        {
            let first_ent = engine.entity_1.get_read();
            let second_ent = engine.entity_2.get_read();
            let world = WorldComputeHandler::from_world_handler(&engine.world);
            loop {
                match response_receiver.try_recv() {
                    Ok(response) => response.apply(&first_ent, &second_ent, &world),
                    Err(_) => break
                }
            }
        }
        
        
        tile_editor.cam = new_camera;
        match user_events.try_recv() {
            Ok(evt) => {
                tile_editor.handle_user_event(evt);
            }
            Err(_) => ()
        }
        tile_editor.do_mouse_handling(&mut world_handler.world.write().unwrap());
        tile_editor.handle_keyboard(&input_handler, &mut world_handler.world.write().unwrap());
        tile_editor.do_rendering(&vectorinator, &world_handler.world.read().unwrap());
        scheduler.initialise(queue.clone());
        scheduler.tick();
        let mut fps = 1.0/Instant::now().checked_duration_since(start).unwrap().as_secs_f64();
        println!("FPS : {}", fps);
        if fps >= 78.0 {
            thread::sleep(Duration::from_secs_f64(1.0/75.0 - Instant::now().checked_duration_since(start).unwrap().as_secs_f64()));
        }
    }
    scheduler.end_threads();
}
