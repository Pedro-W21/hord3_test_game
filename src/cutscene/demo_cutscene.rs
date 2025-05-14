use std::{f32::consts::PI, time::Duration};

use hord3::{defaults::{default_rendering::vectorinator_binned::rendering_spaces::ViewportData, default_ui::simple_ui::{UIDimensions, UIUnit, UIVector}}, horde::{geometry::{rotation::Orientation, vec3d::Vec3D}, rendering::camera::Camera}};

use crate::{game_engine::CoolGameEngineTID, gui_elements::{centered_title::get_centered_title, title_desc::get_title_desc, title_desc_image::get_title_desc_image}};

use super::{camera_movement::{CameraMovement, CameraMovementDuration, CameraMovementElement, CameraSequence}, cutscene_gui::{UIMovement, UIMovementDuration, UIMovementElement, UISequence}, cutscene_shader::{ShaderChange, ShaderChangeDuration, ShaderChangeElement, ShaderSequence}, entity_movement::{EntityMovement, EntityMovementDuration, EntityMovementElement, EntitySequence}, reverse_camera_coords::reverse_from_raster_to_worldpos, CameraCutscene, EntityCutscene, FullCutscene, GUICutscene, ShaderCutscene};

/// idea :
/// - starts off as a normal powerpoint presentation with theoretical stuff
///     - Say that the engine is a compile-time ECS kind of deal
///     - Rendering in 3 passes : geometry projection, rasterisation (binned !), post-processing shaders
///     - the star of the show : the Orchestrator for mostly painless parallelisation
///     - the other star : the macro system
/// - first image of the engine is actually static cutout into the game world, but *looks* like an image
/// - then we have part of the screen used for 3D presentations, like an entity and its ECS attributes (entity is rotating after a bit, of course, to show off normal-based shaders and hint that it's real-time)
/// - to show off shaders further, do a fade between different fog colors and distances to reveal something else and go to it
/// 
/// actual flow :
/// - Hord3 : jamais 2 sans 3
/// - contexte :
///     - besoin : un moteur de jeu relativement généraliste avec du rendu sur CPU, utilisable en Rust
///     - antécédents : horde et horde2, complètement jetés
///     - solution : hord3
/// - Sommaire :
///     - Orchestrateur et macros
///     - Rendu en rasterisation
///     - ECS
/// - Orchestrateur (tout ici est en théorique) (mettre une image explicative avec de l'espace pour le texte en fond)
///     - expliciter plusieurs fils d'exécution avec des phases parallèles et séquentielles
///     - manager des ressources matérielles limitées
///     - pour rendre le système robuste, les implémentations d'Orchestrateur sont générées automatiquement par macros
///     - permet d'assigner les types et leurs paramètres génériques à utiliser
///     - chaque type peut avoir une infinité de tâches associées
/// - Rendu (couper la fenêtre en 2 pour montrer le monde) (faire la coupe en rétrécissant vers la gauche l'UI)
///     - Rendu en rasterisation multi-coeur, vectorisé (avec SIMD en Rust qui compile en AVX sur x86_64)
///     - 3 étapes:
///         - projection (tout est en wireframe et on montre des modèles se faire projeter)
///             - on prend toutes les instances de maillage et on en fait le projeté d'abord dans le référentiel centré sur la caméra, puis sur l'image, en prenant compte les LOD (montrer différentes LOD de voxels)!
///             - tout cela est vectorisé de fou
///             - on rajoute tous les triangles dans une pile de triangles, et les index de cette pile dans des "bins" représentant des régions de l'image (pour ne pas avoir quelques triangles qui prennent tout le temps de calcul sur 1 coeur parce qu'ils sont trop grands)
///         - rasterisation (on colorie les triangles et une scène plus claire )
///             - on prend les index dans les bins et les triangles correspondants dans la pile de triangles
///             - on colorie chaque triangle avec sa texture, sa lumière (filtre de couleur), et on rajoute la distance de chaque pixel du triangle à la caméra dans le depth buffer
///             - pour les triangles au-delà d'une certaine taille, on vectorise leur rasterisation (pixels N par N)
///         - shaders de post processing (on fait un shader de normales et )
///             - avec accès au zbuffer, et au framebuffer final, on peut décider de changer les couleurs pour l'image finale
///             - parallèle comme le reste, peut prendre des calculs arbitraires en 1 seule passe
///     - bien préciser que tout le rendu est modulaire et générique, c'est totalement possible de remplacer tout ce pipeline par un autre, de faire du raycasting, des BSP, comme on veut !
/// - ECS (wipe tout et avoir une entité shadé qui apparaît, une sphère suffit)
///     - c'est orienté data, pour améliorer la localité du cache
///     - avec des macros, on peut définir le type "général" d'une entité, et les macros codent l'ECS en fond, avec des attributs permettant d'indiquer ce que l'on veut utiliser pour le rendu, la création de nouvelles entités, et plus
///     - possibilité de combiner plusieurs types d'entités et un monde en un "moteur" aussi généré par macros
///     - on code ensuite les fonctions qui décrivent comment on passe du "tick" i à i+1, et on met les tâches dans le bon ordre dans l'orchestrateur
/// - fin
///     - et tout cela, sans même parler du système audio 3D, du multijoueur implémenté automatiquement sur commande, ou de la généralisation possible de l'Orchestrateur à d'autres problèmes, est ce qui compose... Hord3
///     - (la caméra tourne à un grand monde voxel alors que la distance de rendu augmente avec un brouillard plus loin, avec "Hord3" écrit en entités dans le ciel au dessus de pentes, et les entités dégringolent les pentes, puis un "Merci de votre écoute" arrive pour une cerise sur le gâteau)
/// 
/// 
/// 

const TITLE_SCREEN_TIME:f32 = 3.0;
const SUMMARY_TIME:f32 = 10.0;
const ORCHESTRATOR_TIME:f32 = 10.0;
const ORCH_RASTER_TRANS_TIME:f32 = 2.0;
const RASTER_START_TIME:f32 = 10.0;
const RASTER_MID_TIME:f32 = 10.0;
const RASTER_END_TIME:f32 = 10.0;
const RASTER_END2_TIME:f32 = 10.0;
const ECS_START_TIME:f32 = 10.0;
const ECS_MID_TIME:f32 = 10.0;
const ECS_END_TIME:f32 = 10.0;
const ECS_END2_TIME:f32 = 10.0;
const PERSPECTIVES_TIME:f32 = 10.0;

const TIME_TO_RASTER_TRANS_START:f32 = TITLE_SCREEN_TIME + SUMMARY_TIME + ORCHESTRATOR_TIME + ORCH_RASTER_TRANS_TIME;
const TIME_TO_RASTER_END_SECTION:f32 = TIME_TO_RASTER_TRANS_START + RASTER_START_TIME + RASTER_MID_TIME;


pub fn get_empty_cutscene() -> FullCutscene {
    FullCutscene::new(GUICutscene::new(vec![]), EntityCutscene::new(vec![]), CameraCutscene::new(vec![]), ShaderCutscene::new(vec![]))
}

pub fn get_demo_cutscene(viewport_data:&ViewportData) -> FullCutscene {
    FullCutscene::new(
        GUICutscene::new(
            vec![
                UISequence::new(
                    vec![UIMovement::new(vec![
                        UIMovementElement::StayPut
                    ], UIMovementDuration::RealTime { duration: Duration::from_secs_f32(TITLE_SCREEN_TIME) } )],
                    get_centered_title(
                        UIVector::new(UIUnit::RelativeToParentOrigin(0), UIUnit::RelativeToParentOrigin(0)),
                        UIDimensions::Decided(UIVector::new(UIUnit::ParentWidthProportion(1.0), UIUnit::ParentHeightProportion(1.0))),
                        "HORD3_TITLE".to_string(), "Hord3\njamais 2 sans 3".to_string(), "S'en_fout".to_string()
                    )[0].clone(),
                    || {
                        get_centered_title(
                            UIVector::new(UIUnit::RelativeToParentOrigin(0), UIUnit::RelativeToParentOrigin(0)),
                            UIDimensions::Decided(UIVector::new(UIUnit::ParentWidthProportion(1.0), UIUnit::ParentHeightProportion(1.0))),
                            "HORD3_TITLE".to_string(), "Hord3\njamais 2 sans 3".to_string(), "S'en_fout".to_string()
                        )
                    }
                ),
                UISequence::new(
                    vec![UIMovement::new(vec![
                        UIMovementElement::StayPut
                    ], UIMovementDuration::RealTime { duration: Duration::from_secs_f32(SUMMARY_TIME) } )],
                    get_title_desc(UIVector::new(UIUnit::RelativeToParentOrigin(0), UIUnit::RelativeToParentOrigin(0)),
                        UIDimensions::Decided(UIVector::new(UIUnit::ParentWidthProportion(1.0), UIUnit::ParentHeightProportion(1.0))),
                        "HORD3_SUMMARY".to_string(), " Sommaire".to_string(), " - Orchestrateur et macros\n - Rendu en rasterisation\n - ECS".to_string(), "S'en_fout".to_string()
                        )[0].clone(),
                    || {
                        get_title_desc(UIVector::new(UIUnit::RelativeToParentOrigin(0), UIUnit::RelativeToParentOrigin(0)),
                        UIDimensions::Decided(UIVector::new(UIUnit::ParentWidthProportion(1.0), UIUnit::ParentHeightProportion(1.0))),
                        "HORD3_SUMMARY".to_string(), " Sommaire".to_string(), " - Orchestrateur et macros\n - Rendu en rasterisation\n - ECS".to_string(), "S'en_fout".to_string()
                        )
                    }
                ),
                // - Orchestrateur (tout ici est en théorique) (mettre une image explicative avec de l'espace pour le texte en fond)
                //     - expliciter plusieurs fils d'exécution avec des phases parallèles et séquentielles
                //     - manager des ressources matérielles limitées
                //     - pour rendre le système robuste, les implémentations d'Orchestrateur sont générées automatiquement par macros
                //     - permet d'assigner les types et leurs paramètres génériques à utiliser
                //     - chaque type peut avoir une infinité de tâches associées
                UISequence::new(
                    vec![UIMovement::new(vec![
                        UIMovementElement::StayPut
                    ], UIMovementDuration::RealTime { duration: Duration::from_secs_f32(ORCHESTRATOR_TIME) } )],
                    get_title_desc_image(UIVector::new(UIUnit::RelativeToParentOrigin(0), UIUnit::RelativeToParentOrigin(0)),
                        UIDimensions::Decided(UIVector::new(UIUnit::ParentWidthProportion(1.0), UIUnit::ParentHeightProportion(1.0))),
                        "HORD3_ORCHESTRATOR".to_string(), "Test".to_string(), " Orchestrateur et macros".to_string(), " Permet : \n  - d'expliciter le déroulement de plusieurs fils d'exécutions parallèles avec des phases séquentielles\n  - de gérer des ressources matérielles limitées en temps réel\n\n Les implémentations de tâches valides sont automatisées à l'aide de macros\n  - on donne les types dont chaque tâche a besoin et ses paramètres génériques\n  - Chaque type peut avoir 2^64 tâches associées".to_string(), "S'en_fout".to_string()
                        )[0].clone(),
                    || {
                        get_title_desc_image(UIVector::new(UIUnit::RelativeToParentOrigin(0), UIUnit::RelativeToParentOrigin(0)),
                        UIDimensions::Decided(UIVector::new(UIUnit::ParentWidthProportion(1.0), UIUnit::ParentHeightProportion(1.0))),
                        "HORD3_ORCHESTRATOR".to_string(), "Test".to_string(), " Orchestrateur et macros".to_string(), " Permet : \n  - d'expliciter le déroulement de plusieurs fils d'exécutions parallèles avec des phases séquentielles\n  - de gérer des ressources matérielles limitées en temps réel\n\n Les implémentations de tâches valides sont automatisées à l'aide de macros\n  - on donne les types dont chaque tâche a besoin et ses paramètres génériques\n  - Chaque type peut avoir 2^64 tâches associées".to_string(), "S'en_fout".to_string()
                        )
                    }
                ),
                // - Rendu (couper la fenêtre en 2 pour montrer le monde) (faire la coupe en rétrécissant vers la gauche l'UI)
                //     - Rendu en rasterisation multi-coeur, vectorisé (avec SIMD en Rust qui compile en AVX sur x86_64)
                //     - 3 étapes:
                //         - projection (tout est en wireframe et on montre des modèles se faire projeter)
                //             - on prend toutes les instances de maillage et on en fait le projeté d'abord dans le référentiel centré sur la caméra, puis sur l'image, en prenant compte les LOD (montrer différentes LOD de voxels)!
                //             - tout cela est vectorisé de fou
                //             - on rajoute tous les triangles dans une pile de triangles, et les index de cette pile dans des "bins" représentant des régions de l'image (pour ne pas avoir quelques triangles qui prennent tout le temps de calcul sur 1 coeur parce qu'ils sont trop grands)
                //         - rasterisation (on colorie les triangles et une scène plus claire )
                //             - on prend les index dans les bins et les triangles correspondants dans la pile de triangles
                //             - on colorie chaque triangle avec sa texture, sa lumière (filtre de couleur), et on rajoute la distance de chaque pixel du triangle à la caméra dans le depth buffer
                //             - pour les triangles au-delà d'une certaine taille, on vectorise leur rasterisation (pixels N par N)
                //         - shaders de post processing (on fait un shader de normales et le brouillard)
                //             - avec accès au zbuffer, et au framebuffer final, on peut décider de changer les couleurs pour l'image finale
                //             - parallèle comme le reste, peut prendre des calculs arbitraires en 1 seule passe
                //     - bien préciser que tout le rendu est modulaire et générique, c'est totalement possible de remplacer tout ce pipeline par un autre, de faire du raycasting, des BSP, comme on veut !
                UISequence::new(
                    vec![UIMovement::new(vec![
                        UIMovementElement::ChangeDimsFromToLinear { from: UIVector::new(UIUnit::ParentWidthProportion(1.0), UIUnit::ParentHeightProportion(1.0)), to: UIVector::new(UIUnit::ParentWidthProportion(0.5), UIUnit::ParentHeightProportion(1.0)) }
                    ], UIMovementDuration::RealTime { duration: Duration::from_secs_f32(ORCH_RASTER_TRANS_TIME) } )],
                    get_title_desc_image(UIVector::new(UIUnit::RelativeToParentOrigin(0), UIUnit::RelativeToParentOrigin(0)),
                        UIDimensions::Decided(UIVector::new(UIUnit::ParentWidthProportion(1.0), UIUnit::ParentHeightProportion(1.0))),
                        "HORD3_RASTER_TRANSITION".to_string(), "Test".to_string(), " Orchestrateur et macros".to_string(), " Permet : \n  - d'expliciter le déroulement de plusieurs fils d'exécutions parallèles avec des phases séquentielles\n  - de gérer des ressources matérielles limitées en temps réel\n\n Les implémentations de tâches valides sont automatisées à l'aide de macros\n  - on donne les types dont chaque tâche a besoin et ses paramètres génériques\n  - Chaque type peut avoir 2^64 tâches associées".to_string(), "S'en_fout".to_string()
                        )[0].clone(),
                    || {
                        get_title_desc_image(UIVector::new(UIUnit::RelativeToParentOrigin(0), UIUnit::RelativeToParentOrigin(0)),
                        UIDimensions::Decided(UIVector::new(UIUnit::ParentWidthProportion(1.0), UIUnit::ParentHeightProportion(1.0))),
                        "HORD3_RASTER_TRANSITION".to_string(), "Test".to_string(), " Orchestrateur et macros".to_string(), " Permet : \n  - d'expliciter le déroulement de plusieurs fils d'exécutions parallèles avec des phases séquentielles\n  - de gérer des ressources matérielles limitées en temps réel\n\n Les implémentations de tâches valides sont automatisées à l'aide de macros\n  - on donne les types dont chaque tâche a besoin et ses paramètres génériques\n  - Chaque type peut avoir 2^64 tâches associées".to_string(), "S'en_fout".to_string()
                        )
                    }
                ),
                //         - projection (tout est en wireframe et on montre des modèles se faire projeter)
                //             - on prend toutes les instances de maillage et on en fait le projeté d'abord dans le référentiel centré sur la caméra, puis sur l'image, en prenant compte les LOD (montrer différentes LOD de voxels)!
                //             - tout cela est vectorisé de fou
                //             - on rajoute tous les triangles dans une pile de triangles, et les index de cette pile dans des "bins" représentant des régions de l'image (pour ne pas avoir quelques triangles qui prennent tout le temps de calcul sur 1 coeur parce qu'ils sont trop grands)
                
                UISequence::new(
                    vec![UIMovement::new(vec![
                        UIMovementElement::StayPut
                    ], UIMovementDuration::RealTime { duration: Duration::from_secs_f32(RASTER_START_TIME) } )],
                    get_title_desc(UIVector::new(UIUnit::RelativeToParentOrigin(0), UIUnit::RelativeToParentOrigin(0)),
                        UIDimensions::Decided(UIVector::new(UIUnit::ParentWidthProportion(0.5), UIUnit::ParentHeightProportion(1.0))),
                        "HORD3_RASTER_START".to_string()," Le rendu 3D en rasterisation : Projection".to_string(), " Les modèles 3D ont une position associée dans le monde, et nous connaissons la position de la caméra, Il faut donc traduire la position de chaque point sur les modèles 3D en position sur l'écran".to_string(), "S'en_fout".to_string()
                        )[0].clone(),
                    || {
                        get_title_desc(UIVector::new(UIUnit::RelativeToParentOrigin(0), UIUnit::RelativeToParentOrigin(0)),
                        UIDimensions::Decided(UIVector::new(UIUnit::ParentWidthProportion(0.5), UIUnit::ParentHeightProportion(1.0))),
                        "HORD3_RASTER_START".to_string()," Le rendu 3D en rasterisation : Projection".to_string(), " Les modèles 3D ont une position associée dans le monde, et nous connaissons la position de la caméra, Il faut donc traduire la position de chaque point sur les modèles 3D en position sur l'écran".to_string(), "S'en_fout".to_string()
                        )
                    }
                ),
                //         - rasterisation (on colorie les triangles et une scène plus claire )
                //             - on prend les index dans les bins et les triangles correspondants dans la pile de triangles
                //             - on colorie chaque triangle avec sa texture, sa lumière (filtre de couleur), et on rajoute la distance de chaque pixel du triangle à la caméra dans le depth buffer
                //             - pour les triangles au-delà d'une certaine taille, on vectorise leur rasterisation (pixels N par N)
                
                UISequence::new(
                    vec![UIMovement::new(vec![
                        UIMovementElement::StayPut
                    ], UIMovementDuration::RealTime { duration: Duration::from_secs_f32(RASTER_MID_TIME) } )],
                    get_title_desc(UIVector::new(UIUnit::RelativeToParentOrigin(0), UIUnit::RelativeToParentOrigin(0)),
                        UIDimensions::Decided(UIVector::new(UIUnit::ParentWidthProportion(0.5), UIUnit::ParentHeightProportion(1.0))),
                        "HORD3_RASTER_RASTERISATION".to_string()," Le rendu 3D en rasterisation : Rasterisation".to_string(), " Une fois les modèles 3D projetés dans l'espace de l'écran, Il faut colorier les pixels à l'intérieur de leurs triangles en fonction de la texture utilisée et de la luminosité du triangle.".to_string(), "S'en_fout".to_string()
                        )[0].clone(),
                    || {
                        get_title_desc(UIVector::new(UIUnit::RelativeToParentOrigin(0), UIUnit::RelativeToParentOrigin(0)),
                        UIDimensions::Decided(UIVector::new(UIUnit::ParentWidthProportion(0.5), UIUnit::ParentHeightProportion(1.0))),
                        "HORD3_RASTER_RASTERISATION".to_string()," Le rendu 3D en rasterisation : Rasterisation".to_string(), " Une fois les modèles 3D projetés dans l'espace de l'écran, Il faut colorier les pixels à l'intérieur de leurs triangles en fonction de la texture utilisée et de la luminosité du triangle.".to_string(), "S'en_fout".to_string()
                        )
                    }
                ),
                //         - shaders de post processing (on fait un shader de normales et le brouillard)
                //             - avec accès au zbuffer, et au framebuffer final, on peut décider de changer les couleurs pour l'image finale
                //             - parallèle comme le reste, peut prendre des calculs arbitraires en 1 seule passe
                UISequence::new(
                    vec![UIMovement::new(vec![
                        UIMovementElement::StayPut
                    ], UIMovementDuration::RealTime { duration: Duration::from_secs_f32(RASTER_END_TIME) } )],
                    get_title_desc(UIVector::new(UIUnit::RelativeToParentOrigin(0), UIUnit::RelativeToParentOrigin(0)),
                        UIDimensions::Decided(UIVector::new(UIUnit::ParentWidthProportion(0.5), UIUnit::ParentHeightProportion(1.0))),
                        "HORD3_RASTER_SHADERS".to_string()," Le rendu 3D en rasterisation : Shaders".to_string(), " Quand tous les triangles ont été projetés et texturés, il est possible de repasser 1 fois sur tous les pixels avec un calcul arbitraire ayant les données de couleurs, profondeur, et normales environnantes".to_string(), "S'en_fout".to_string()
                        )[0].clone(),
                    || {
                        get_title_desc(UIVector::new(UIUnit::RelativeToParentOrigin(0), UIUnit::RelativeToParentOrigin(0)),
                        UIDimensions::Decided(UIVector::new(UIUnit::ParentWidthProportion(0.5), UIUnit::ParentHeightProportion(1.0))),
                        "HORD3_RASTER_SHADERS".to_string()," Le rendu 3D en rasterisation : Shaders".to_string(), " Quand tous les triangles ont été projetés et texturés, il est possible de repasser 1 fois sur tous les pixels avec un calcul arbitraire ayant les données de couleurs, profondeur, et normales environnantes".to_string(), "S'en_fout".to_string()
                        )
                    }
                ),
                //     - bien préciser que tout le rendu est modulaire et générique, c'est totalement possible de remplacer tout ce pipeline par un autre, de faire du raycasting, des BSP, comme on veut !
                UISequence::new(
                    vec![UIMovement::new(vec![
                        UIMovementElement::StayPut
                    ], UIMovementDuration::RealTime { duration: Duration::from_secs_f32(RASTER_END2_TIME) } )],
                    get_title_desc(UIVector::new(UIUnit::RelativeToParentOrigin(0), UIUnit::RelativeToParentOrigin(0)),
                        UIDimensions::Decided(UIVector::new(UIUnit::ParentWidthProportion(0.5), UIUnit::ParentHeightProportion(1.0))),
                        "HORD3_RASTER_REST".to_string()," Le rendu de Hord3 : Modularité".to_string(), " Tout ceci n'est qu'un moteur de rendu, Il est possible de remplacer tout cela par du rendu sur GPU, du raycassting sur des voxel ou tout autre mode de rendu !".to_string(), "S'en_fout".to_string()
                        )[0].clone(),
                    || {
                        get_title_desc(UIVector::new(UIUnit::RelativeToParentOrigin(0), UIUnit::RelativeToParentOrigin(0)),
                        UIDimensions::Decided(UIVector::new(UIUnit::ParentWidthProportion(0.5), UIUnit::ParentHeightProportion(1.0))),
                        "HORD3_RASTER_REST".to_string()," Le rendu de Hord3 : Modularité".to_string(), " Tout ceci n'est qu'un moteur de rendu, Il est possible de remplacer tout cela par du rendu sur GPU, du raycassting sur des voxel ou tout autre mode de rendu !".to_string(), "S'en_fout".to_string()
                        )
                    }
                ),
                // - ECS (wipe tout et avoir une entité shadé qui apparaît, une sphère suffit)
                //     - c'est orienté data, pour améliorer la localité du cache
                //     - avec des macros, on peut définir le type "général" d'une entité, et les macros codent l'ECS en fond, avec des attributs permettant d'indiquer ce que l'on veut utiliser pour le rendu, la création de nouvelles entités, et plus
                //     - possibilité de combiner plusieurs types d'entités et un monde en un "moteur" aussi généré par macros
                //     - on code ensuite les fonctions qui décrivent comment on passe du "tick" i à i+1, et on met les tâches dans le bon ordre dans l'orchestrateur
                UISequence::new(
                    vec![UIMovement::new(vec![
                        UIMovementElement::StayPut
                    ], UIMovementDuration::RealTime { duration: Duration::from_secs_f32(ECS_START_TIME) } )],
                    get_title_desc(UIVector::new(UIUnit::RelativeToParentOrigin(0), UIUnit::RelativeToParentOrigin(0)),
                        UIDimensions::Decided(UIVector::new(UIUnit::ParentWidthProportion(0.5), UIUnit::ParentHeightProportion(1.0))),
                        "HORD3_ECS_START".to_string()," ECS : Entity Component System".to_string(), " Chaque entité dans Hord3 n'est pas une structure monolithique, mais la combinaison de composants discrets se trouvant chacun au même indice dans des listes différentes.".to_string(), "S'en_fout".to_string()
                        )[0].clone(),
                    || {
                        get_title_desc(UIVector::new(UIUnit::RelativeToParentOrigin(0), UIUnit::RelativeToParentOrigin(0)),
                        UIDimensions::Decided(UIVector::new(UIUnit::ParentWidthProportion(0.5), UIUnit::ParentHeightProportion(1.0))),
                        "HORD3_ECS_START".to_string()," ECS : Entity Component System".to_string(), " Chaque entité dans Hord3 n'est pas une structure monolithique, mais la combinaison de composants discrets se trouvant chacun au même indice dans des listes différentes.".to_string(), "S'en_fout".to_string()
                        )
                    }
                ),

                UISequence::new(
                    vec![UIMovement::new(vec![
                        UIMovementElement::StayPut
                    ], UIMovementDuration::RealTime { duration: Duration::from_secs_f32(ECS_MID_TIME) } )],
                    get_title_desc(UIVector::new(UIUnit::RelativeToParentOrigin(0), UIUnit::RelativeToParentOrigin(0)),
                        UIDimensions::Decided(UIVector::new(UIUnit::ParentWidthProportion(0.5), UIUnit::ParentHeightProportion(1.0))),
                        "HORD3_ECS_MID".to_string()," ECS : Entity Component System".to_string(), " Des macros permettent de passer d'une combinaison de composants à une structure représentant cet archétype d'entité dans un ECS, en précisant quels composants ont une utilité spéciale".to_string(), "S'en_fout".to_string()
                        )[0].clone(),
                    || {
                        get_title_desc(UIVector::new(UIUnit::RelativeToParentOrigin(0), UIUnit::RelativeToParentOrigin(0)),
                        UIDimensions::Decided(UIVector::new(UIUnit::ParentWidthProportion(0.5), UIUnit::ParentHeightProportion(1.0))),
                        "HORD3_ECS_MID".to_string()," ECS : Entity Component System".to_string(), " Des macros permettent de passer d'une combinaison de composants à une structure représentant cet archétype d'entité dans un ECS, en précisant quels composants ont une utilité spéciale".to_string(), "S'en_fout".to_string()
                        )
                    }
                ),

                UISequence::new(
                    vec![UIMovement::new(vec![
                        UIMovementElement::StayPut
                    ], UIMovementDuration::RealTime { duration: Duration::from_secs_f32(ECS_END_TIME) } )],
                    get_title_desc(UIVector::new(UIUnit::RelativeToParentOrigin(0), UIUnit::RelativeToParentOrigin(0)),
                        UIDimensions::Decided(UIVector::new(UIUnit::ParentWidthProportion(0.5), UIUnit::ParentHeightProportion(1.0))),
                        "HORD3_ECS_END".to_string()," ECS : Entity Component System".to_string(), " Avec d'autres macros, nous pouvons combiner plusieurs entités, un monde et des données arbitraires supplémentaires en un `moteur` avec une backend complète".to_string(), "S'en_fout".to_string()
                        )[0].clone(),
                    || {
                        get_title_desc(UIVector::new(UIUnit::RelativeToParentOrigin(0), UIUnit::RelativeToParentOrigin(0)),
                        UIDimensions::Decided(UIVector::new(UIUnit::ParentWidthProportion(0.5), UIUnit::ParentHeightProportion(1.0))),
                        "HORD3_ECS_END".to_string()," ECS : Entity Component System".to_string(), " Avec d'autres macros, nous pouvons combiner plusieurs entités, un monde et des données arbitraires supplémentaires en un `moteur` avec une backend complète".to_string(), "S'en_fout".to_string()
                        )
                    }
                ),

                UISequence::new(
                    vec![UIMovement::new(vec![
                        UIMovementElement::StayPut
                    ], UIMovementDuration::RealTime { duration: Duration::from_secs_f32(ECS_END2_TIME) } )],
                    get_title_desc(UIVector::new(UIUnit::RelativeToParentOrigin(0), UIUnit::RelativeToParentOrigin(0)),
                        UIDimensions::Decided(UIVector::new(UIUnit::ParentWidthProportion(0.5), UIUnit::ParentHeightProportion(1.0))),
                        "HORD3_ECS_END2".to_string()," ECS : Entity Component System".to_string(), " Il suffit ensuite d'écrire le code spécifique à notre jeu (les fonctions gérant les intéractions entre entités à chaque tick) et le moteur est prêt à l'utilisation".to_string(), "S'en_fout".to_string()
                        )[0].clone(),
                    || {
                        get_title_desc(UIVector::new(UIUnit::RelativeToParentOrigin(0), UIUnit::RelativeToParentOrigin(0)),
                        UIDimensions::Decided(UIVector::new(UIUnit::ParentWidthProportion(0.5), UIUnit::ParentHeightProportion(1.0))),
                        "HORD3_ECS_END2".to_string()," ECS : Entity Component System".to_string(), " Il suffit ensuite d'écrire le code spécifique à notre jeu (les fonctions gérant les intéractions entre entités à chaque tick) et le moteur est prêt à l'utilisation".to_string(), "S'en_fout".to_string()
                        )
                    }
                ),

            //    - fin
            //     - et tout cela, sans même parler du système audio 3D, du multijoueur implémenté automatiquement sur commande, ou de la généralisation possible de l'Orchestrateur à d'autres problèmes, est ce qui compose... Hord3
                UISequence::new(
                    vec![UIMovement::new(vec![
                        UIMovementElement::StayPut
                    ], UIMovementDuration::RealTime { duration: Duration::from_secs_f32(PERSPECTIVES_TIME) } )],
                    get_title_desc(UIVector::new(UIUnit::RelativeToParentOrigin(0), UIUnit::RelativeToParentOrigin(0)),
                        UIDimensions::Decided(UIVector::new(UIUnit::ParentWidthProportion(0.5), UIUnit::ParentHeightProportion(1.0))),
                        "HORD3_END_START".to_string()," La suite".to_string(), " Ce moteur a d'autres fonctionnalités : \n - un système audio 3D en pure Rust \n - Un système de multijoueur client-serveur implémentable sur commande par macros \n - le système d'interface utilisé pour cette présentation \n - la généralisation possible de l'Orchestrateur à d'autres problèmes non vidéo-ludiques".to_string(), "S'en_fout".to_string()
                        )[0].clone(),
                    || {
                        get_title_desc(UIVector::new(UIUnit::RelativeToParentOrigin(0), UIUnit::RelativeToParentOrigin(0)),
                        UIDimensions::Decided(UIVector::new(UIUnit::ParentWidthProportion(0.5), UIUnit::ParentHeightProportion(1.0))),
                        "HORD3_END_START".to_string()," La suite".to_string(), " Ce moteur a d'autres fonctionnalités : \n - un système audio 3D en pure Rust \n - Un système de multijoueur client-serveur implémentable sur commande par macros \n - le système d'interface utilisé pour cette présentation \n - la généralisation possible de l'Orchestrateur à d'autres problèmes non vidéo-ludiques".to_string(), "S'en_fout".to_string()
                        )
                    }
                ),
            ]
        ),
        EntityCutscene::new(vec![
            // PRE COOL
            vec![
                EntitySequence::new(
                    vec![
                        EntityMovement::new(
                            vec![
                                EntityMovementElement::StayAt { position: Vec3D::all_ones() * 1000.0 }
                            ],
                            EntityMovementDuration::RealTime { duration: Duration::from_secs_f32(TIME_TO_RASTER_TRANS_START) }
                        )
                    ],
                    CoolGameEngineTID::entity_1(0)
                ),
                EntitySequence::new(
                    vec![
                        EntityMovement::new(
                            vec![
                                EntityMovementElement::StayAt { position: Vec3D::all_ones() * 1000.0 }
                            ],
                            EntityMovementDuration::RealTime { duration: Duration::from_secs_f32(TIME_TO_RASTER_TRANS_START) }
                        )
                    ],
                    CoolGameEngineTID::entity_1(1)
                ),
                EntitySequence::new(
                    vec![
                        EntityMovement::new(
                            vec![
                                EntityMovementElement::StayAt { position: Vec3D::all_ones() * 1000.0 }
                            ],
                            EntityMovementDuration::RealTime { duration: Duration::from_secs_f32(TIME_TO_RASTER_TRANS_START) }
                        )
                    ],
                    CoolGameEngineTID::entity_1(2)
                ),
                EntitySequence::new(
                    vec![
                        EntityMovement::new(
                            vec![
                                EntityMovementElement::StayAt { position: Vec3D::all_ones() * 1000.0 }
                            ],
                            EntityMovementDuration::RealTime { duration: Duration::from_secs_f32(TIME_TO_RASTER_TRANS_START) }
                        )
                    ],
                    CoolGameEngineTID::entity_1(3)
                ),
            ],
            // START
            vec![
                EntitySequence::new(
                    vec![
                        EntityMovement::new(
                            vec![
                                EntityMovementElement::MoveFromToLinear {
                                    from:reverse_from_raster_to_worldpos(Vec3D::new(viewport_data.half_image_width + viewport_data.half_image_width * 0.5, viewport_data.half_image_height, 1.0/1000.0), viewport_data, Camera::empty()),
                                    to:reverse_from_raster_to_worldpos(Vec3D::new(viewport_data.half_image_width + viewport_data.half_image_width * 0.5, viewport_data.half_image_height, 1.0/10.0), viewport_data, Camera::empty())
                                }
                            ],
                            EntityMovementDuration::RealTime { duration: Duration::from_secs_f32(0.2 * RASTER_START_TIME) }
                        )
                    ],
                    CoolGameEngineTID::entity_1(1)
                )
            ],
            vec![
                EntitySequence::new(
                    vec![
                        EntityMovement::new(
                            vec![
                                EntityMovementElement::StayAt { position: reverse_from_raster_to_worldpos(Vec3D::new(viewport_data.half_image_width + viewport_data.half_image_width * 0.5, viewport_data.half_image_height, 1.0/10.0), viewport_data, Camera::empty()) },
                                EntityMovementElement::RotateFromToLinear { from: Orientation::zero(), to: Orientation::new(PI, 0.0, 0.0) }
                            ],
                            EntityMovementDuration::RealTime { duration: Duration::from_secs_f32(0.2 * RASTER_START_TIME) }
                        )
                    ],
                    CoolGameEngineTID::entity_1(1)
                )
            ],
            vec![
                EntitySequence::new(
                    vec![
                        EntityMovement::new(
                            vec![
                                EntityMovementElement::StayAt { position: reverse_from_raster_to_worldpos(Vec3D::new(viewport_data.half_image_width + viewport_data.half_image_width * 0.5, viewport_data.half_image_height, 1.0/10.0), viewport_data, Camera::empty()) },
                                EntityMovementElement::RotateFromToLinear { from: Orientation::zero(), to: Orientation::new(PI, PI, PI) }
                            ],
                            EntityMovementDuration::RealTime { duration: Duration::from_secs_f32(0.6 * RASTER_START_TIME) }
                        )
                    ],
                    CoolGameEngineTID::entity_1(1)
                ),
                EntitySequence::new(
                    vec![
                        EntityMovement::new(
                            vec![
                                EntityMovementElement::StayAt { position: reverse_from_raster_to_worldpos(Vec3D::new(viewport_data.half_image_width + viewport_data.half_image_width * 0.5, viewport_data.half_image_height, 1.0/10.0), viewport_data, Camera::empty()) },
                                EntityMovementElement::RotateFromToLinear { from: Orientation::zero(), to: Orientation::new(PI, PI, PI) }
                            ],
                            EntityMovementDuration::RealTime { duration: Duration::from_secs_f32(0.6 * RASTER_START_TIME) }
                        )
                    ],
                    CoolGameEngineTID::entity_1(0)
                ),
            ],
            vec![
                EntitySequence::new(
                    vec![
                        EntityMovement::new(
                            vec![
                                EntityMovementElement::StayAt { position: reverse_from_raster_to_worldpos(Vec3D::new(viewport_data.half_image_width + viewport_data.half_image_width * 0.5, viewport_data.half_image_height, 1.0/10.0), viewport_data, Camera::empty()) },
                                EntityMovementElement::RotateFromToLinear { from: Orientation::zero(), to: Orientation::new(PI, PI, PI) }
                            ],
                            EntityMovementDuration::RealTime { duration: Duration::from_secs_f32(RASTER_MID_TIME) }
                        )
                    ],
                    CoolGameEngineTID::entity_1(2)
                ),
                EntitySequence::new(
                    vec![
                        EntityMovement::new(
                            vec![
                                EntityMovementElement::StayAt { position: Vec3D::all_ones() * 1000.0 }
                            ],
                            EntityMovementDuration::RealTime { duration: Duration::from_secs_f32(RASTER_MID_TIME) }
                        )
                    ],
                    CoolGameEngineTID::entity_1(0)
                ),
                EntitySequence::new(
                    vec![
                        EntityMovement::new(
                            vec![
                                EntityMovementElement::StayAt { position: Vec3D::all_ones() * 1000.0 }
                            ],
                            EntityMovementDuration::RealTime { duration: Duration::from_secs_f32(RASTER_MID_TIME) }
                        )
                    ],
                    CoolGameEngineTID::entity_1(1)
                ),
            ],
            vec![
                EntitySequence::new(
                    vec![
                        EntityMovement::new(
                            vec![
                                EntityMovementElement::StayAt { position: reverse_from_raster_to_worldpos(Vec3D::new(viewport_data.half_image_width + viewport_data.half_image_width * 0.5, viewport_data.half_image_height, 1.0/10.0), viewport_data, Camera::empty()) },
                                EntityMovementElement::RotateFromToLinear { from: Orientation::zero(), to: Orientation::new(PI, PI, PI) }
                            ],
                            EntityMovementDuration::RealTime { duration: Duration::from_secs_f32(RASTER_END_TIME) }
                        )
                    ],
                    CoolGameEngineTID::entity_1(3)
                ),

                EntitySequence::new(
                    vec![
                        EntityMovement::new(
                            vec![
                                EntityMovementElement::StayAt { position: Vec3D::all_ones() * 1000.0 }
                            ],
                            EntityMovementDuration::RealTime { duration: Duration::from_secs_f32(RASTER_END_TIME) }
                        )
                    ],
                    CoolGameEngineTID::entity_1(2)
                ),
            ],
            vec![
                EntitySequence::new(
                    vec![
                        EntityMovement::new(
                            vec![
                                EntityMovementElement::StayAt { position: reverse_from_raster_to_worldpos(Vec3D::new(viewport_data.half_image_width + viewport_data.half_image_width * 0.5, viewport_data.half_image_height, 1.0/20.0), viewport_data, Camera::empty()) },
                            ],
                            EntityMovementDuration::RealTime { duration: Duration::from_secs_f32(0.5 * ECS_START_TIME) }
                        )
                    ],
                    CoolGameEngineTID::entity_1(10)
                ),

                EntitySequence::new(
                    vec![
                        EntityMovement::new(
                            vec![
                                EntityMovementElement::StayAt { position: Vec3D::all_ones() * 1000.0 }
                            ],
                            EntityMovementDuration::RealTime { duration: Duration::from_secs_f32(0.5 * ECS_START_TIME) }
                        )
                    ],
                    CoolGameEngineTID::entity_1(3)
                ),
            ],
            vec![
                EntitySequence::new(
                    vec![
                        EntityMovement::new(
                            vec![
                                EntityMovementElement::StayAt { 
                                    position: reverse_from_raster_to_worldpos(Vec3D::new(viewport_data.half_image_width + viewport_data.half_image_width * 0.25, viewport_data.half_image_height * 0.5, 1.0/20.0), viewport_data, Camera::empty())
                                }
                            ],
                            EntityMovementDuration::RealTime { duration: Duration::from_secs_f32(0.5 * ECS_START_TIME) }
                        )
                    ],
                    CoolGameEngineTID::entity_1(10)
                ),
                EntitySequence::new(
                    vec![
                        EntityMovement::new(
                            vec![
                                EntityMovementElement::StayAt { position: reverse_from_raster_to_worldpos(Vec3D::new(viewport_data.half_image_width + viewport_data.half_image_width * 0.75, viewport_data.half_image_height * 0.5, 1.0/20.0), viewport_data, Camera::empty()) },
                            ],
                            EntityMovementDuration::RealTime { duration: Duration::from_secs_f32(0.5 * ECS_START_TIME) }
                        )
                    ],
                    CoolGameEngineTID::entity_1(11)
                ),
                EntitySequence::new(
                    vec![
                        EntityMovement::new(
                            vec![
                                EntityMovementElement::StayAt { position: reverse_from_raster_to_worldpos(Vec3D::new(viewport_data.half_image_width + viewport_data.half_image_width * 0.25, viewport_data.half_image_height * 1.5, 1.0/20.0), viewport_data, Camera::empty()) },
                            ],
                            EntityMovementDuration::RealTime { duration: Duration::from_secs_f32(0.5 * ECS_START_TIME) }
                        )
                    ],
                    CoolGameEngineTID::entity_1(12)
                ),
                EntitySequence::new(
                    vec![
                        EntityMovement::new(
                            vec![
                                EntityMovementElement::StayAt { position: reverse_from_raster_to_worldpos(Vec3D::new(viewport_data.half_image_width + viewport_data.half_image_width * 0.75, viewport_data.half_image_height * 1.5, 1.0/20.0), viewport_data, Camera::empty()) },
                            ],
                            EntityMovementDuration::RealTime { duration: Duration::from_secs_f32(0.5 * ECS_START_TIME) }
                        )
                    ],
                    CoolGameEngineTID::entity_1(13)
                )
            ],
            vec![
                EntitySequence::new(
                    vec![
                        EntityMovement::new(
                            vec![
                                EntityMovementElement::StayAt { 
                                    position: reverse_from_raster_to_worldpos(Vec3D::new(viewport_data.half_image_width + viewport_data.half_image_width * 0.25, viewport_data.half_image_height * 0.125, 1.0/20.0), viewport_data, Camera::empty())
                                }
                            ],
                            EntityMovementDuration::RealTime { duration: Duration::from_secs_f32(ECS_MID_TIME) }
                        )
                    ],
                    CoolGameEngineTID::entity_1(10)
                ),
                EntitySequence::new(
                    vec![
                        EntityMovement::new(
                            vec![
                                EntityMovementElement::StayAt { position: reverse_from_raster_to_worldpos(Vec3D::new(viewport_data.half_image_width + viewport_data.half_image_width * 0.75, viewport_data.half_image_height * 0.125, 1.0/20.0), viewport_data, Camera::empty()) },
                            ],
                            EntityMovementDuration::RealTime { duration: Duration::from_secs_f32(ECS_MID_TIME) }
                        )
                    ],
                    CoolGameEngineTID::entity_1(11)
                ),
                EntitySequence::new(
                    vec![
                        EntityMovement::new(
                            vec![
                                EntityMovementElement::StayAt { position: reverse_from_raster_to_worldpos(Vec3D::new(viewport_data.half_image_width + viewport_data.half_image_width * 0.25, viewport_data.half_image_height * 0.375, 1.0/20.0), viewport_data, Camera::empty()) },
                            ],
                            EntityMovementDuration::RealTime { duration: Duration::from_secs_f32(ECS_MID_TIME) }
                        )
                    ],
                    CoolGameEngineTID::entity_1(12)
                ),
                EntitySequence::new(
                    vec![
                        EntityMovement::new(
                            vec![
                                EntityMovementElement::StayAt { position: reverse_from_raster_to_worldpos(Vec3D::new(viewport_data.half_image_width + viewport_data.half_image_width * 0.75, viewport_data.half_image_height * 0.375, 1.0/20.0), viewport_data, Camera::empty()) },
                            ],
                            EntityMovementDuration::RealTime { duration: Duration::from_secs_f32(ECS_MID_TIME) }
                        )
                    ],
                    CoolGameEngineTID::entity_1(13)
                ),
                EntitySequence::new(
                    vec![
                        EntityMovement::new(
                            vec![
                                EntityMovementElement::StayAt { 
                                    position: reverse_from_raster_to_worldpos(Vec3D::new(viewport_data.half_image_width + viewport_data.half_image_width * 0.125, viewport_data.half_image_height + viewport_data.half_image_height * 0.125, 1.0/20.0), viewport_data, Camera::empty())
                                }
                            ],
                            EntityMovementDuration::RealTime { duration: Duration::from_secs_f32(ECS_MID_TIME) }
                        )
                    ],
                    CoolGameEngineTID::entity_1(20)
                ),
                EntitySequence::new(
                    vec![
                        EntityMovement::new(
                            vec![
                                EntityMovementElement::StayAt { position: reverse_from_raster_to_worldpos(Vec3D::new(viewport_data.half_image_width + viewport_data.half_image_width * 0.375, viewport_data.half_image_height + viewport_data.half_image_height * 0.125, 1.0/20.0), viewport_data, Camera::empty()) },
                            ],
                            EntityMovementDuration::RealTime { duration: Duration::from_secs_f32(ECS_MID_TIME) }
                        )
                    ],
                    CoolGameEngineTID::entity_1(21)
                ),
                EntitySequence::new(
                    vec![
                        EntityMovement::new(
                            vec![
                                EntityMovementElement::StayAt { position: reverse_from_raster_to_worldpos(Vec3D::new(viewport_data.half_image_width + viewport_data.half_image_width * 0.625, viewport_data.half_image_height + viewport_data.half_image_height * 0.125, 1.0/20.0), viewport_data, Camera::empty()) },
                            ],
                            EntityMovementDuration::RealTime { duration: Duration::from_secs_f32(ECS_MID_TIME) }
                        )
                    ],
                    CoolGameEngineTID::entity_1(22)
                ),
                EntitySequence::new(
                    vec![
                        EntityMovement::new(
                            vec![
                                EntityMovementElement::StayAt { position: reverse_from_raster_to_worldpos(Vec3D::new(viewport_data.half_image_width + viewport_data.half_image_width * 0.875, viewport_data.half_image_height + viewport_data.half_image_height * 0.125, 1.0/20.0), viewport_data, Camera::empty()) },
                            ],
                            EntityMovementDuration::RealTime { duration: Duration::from_secs_f32(ECS_MID_TIME) }
                        )
                    ],
                    CoolGameEngineTID::entity_1(23)
                )
            ],
            vec![
                EntitySequence::new(
                    vec![
                        EntityMovement::new(
                            vec![
                                EntityMovementElement::StayAt { 
                                    position: Vec3D::all_ones() * 1000.0
                                }
                            ],
                            EntityMovementDuration::RealTime { duration: Duration::from_secs_f32(ECS_END_TIME) }
                        )
                    ],
                    CoolGameEngineTID::entity_1(10)
                ),
                EntitySequence::new(
                    vec![
                        EntityMovement::new(
                            vec![
                                EntityMovementElement::StayAt { position: Vec3D::all_ones() * 1000.0}
                            ],
                            EntityMovementDuration::RealTime { duration: Duration::from_secs_f32(ECS_END_TIME) }
                        )
                    ],
                    CoolGameEngineTID::entity_1(11)
                ),
                EntitySequence::new(
                    vec![
                        EntityMovement::new(
                            vec![
                                EntityMovementElement::StayAt { position: Vec3D::all_ones() * 1000.0}
                            ],
                            EntityMovementDuration::RealTime { duration: Duration::from_secs_f32(ECS_END_TIME) }
                        )
                    ],
                    CoolGameEngineTID::entity_1(12)
                ),
                EntitySequence::new(
                    vec![
                        EntityMovement::new(
                            vec![
                                EntityMovementElement::StayAt { position: Vec3D::all_ones() * 1000.0}
                            ],
                            EntityMovementDuration::RealTime { duration: Duration::from_secs_f32(ECS_END_TIME) }
                        )
                    ],
                    CoolGameEngineTID::entity_1(13)
                ),
                EntitySequence::new(
                    vec![
                        EntityMovement::new(
                            vec![
                                EntityMovementElement::StayAt { position: Vec3D::all_ones() * 1000.0}
                            ],
                            EntityMovementDuration::RealTime { duration: Duration::from_secs_f32(ECS_END_TIME) }
                        )
                    ],
                    CoolGameEngineTID::entity_1(20)
                ),
                EntitySequence::new(
                    vec![
                        EntityMovement::new(
                            vec![
                                EntityMovementElement::StayAt { position: Vec3D::all_ones() * 1000.0}
                            ],
                            EntityMovementDuration::RealTime { duration: Duration::from_secs_f32(ECS_END_TIME) }
                        )
                    ],
                    CoolGameEngineTID::entity_1(21)
                ),
                EntitySequence::new(
                    vec![
                        EntityMovement::new(
                            vec![
                                EntityMovementElement::StayAt { position: Vec3D::all_ones() * 1000.0}
                            ],
                            EntityMovementDuration::RealTime { duration: Duration::from_secs_f32(ECS_END_TIME) }
                        )
                    ],
                    CoolGameEngineTID::entity_1(22)
                ),
                EntitySequence::new(
                    vec![
                        EntityMovement::new(
                            vec![
                                EntityMovementElement::StayAt { position: Vec3D::all_ones() * 1000.0}
                            ],
                            EntityMovementDuration::RealTime { duration: Duration::from_secs_f32(ECS_END_TIME) }
                        )
                    ],
                    CoolGameEngineTID::entity_1(23)
                ),
                EntitySequence::new(
                    vec![
                        EntityMovement::new(
                            vec![
                                EntityMovementElement::StayAt { position: reverse_from_raster_to_worldpos(Vec3D::new(viewport_data.half_image_width + viewport_data.half_image_width * 0.125, viewport_data.half_image_height * 0.125, 1.0/20.0), viewport_data, Camera::empty()) },
                            ],
                            EntityMovementDuration::RealTime { duration: Duration::from_secs_f32(ECS_END_TIME) }
                        )
                    ],
                    CoolGameEngineTID::entity_1(30)
                ),
                EntitySequence::new(
                    vec![
                        EntityMovement::new(
                            vec![
                                EntityMovementElement::StayAt { position: reverse_from_raster_to_worldpos(Vec3D::new(viewport_data.half_image_width + viewport_data.half_image_width * 0.375, viewport_data.half_image_height * 0.125, 1.0/20.0), viewport_data, Camera::empty()) },
                            ],
                            EntityMovementDuration::RealTime { duration: Duration::from_secs_f32(ECS_END_TIME) }
                        )
                    ],
                    CoolGameEngineTID::entity_1(31)
                ),
                EntitySequence::new(
                    vec![
                        EntityMovement::new(
                            vec![
                                EntityMovementElement::StayAt { position: reverse_from_raster_to_worldpos(Vec3D::new(viewport_data.half_image_width + viewport_data.half_image_width * 0.625, viewport_data.half_image_height * 0.125, 1.0/20.0), viewport_data, Camera::empty()) },
                            ],
                            EntityMovementDuration::RealTime { duration: Duration::from_secs_f32(ECS_END_TIME) }
                        )
                    ],
                    CoolGameEngineTID::entity_1(32)
                ),
                EntitySequence::new(
                    vec![
                        EntityMovement::new(
                            vec![
                                EntityMovementElement::StayAt { position: reverse_from_raster_to_worldpos(Vec3D::new(viewport_data.half_image_width + viewport_data.half_image_width * 0.875, viewport_data.half_image_height * 0.125, 1.0/20.0), viewport_data, Camera::empty()) },
                            ],
                            EntityMovementDuration::RealTime { duration: Duration::from_secs_f32(ECS_END_TIME) }
                        )
                    ],
                    CoolGameEngineTID::entity_1(33)
                )
            ],
            
        ]),
        CameraCutscene::new(vec![
            CameraSequence::new(vec![
                CameraMovement::new(
                    vec![
                        CameraMovementElement::StayPut
                        ],
                    CameraMovementDuration::RealTime { duration: Duration::from_secs_f32(5.0*TIME_TO_RASTER_TRANS_START) }
                )
            ])
        ]),
        ShaderCutscene::new(vec![
            ShaderSequence::new(vec![
                ShaderChange::new(vec![
                    ShaderChangeElement::FogDistanceChange { from: 10000.0, to: 10000.0 },
                    ShaderChangeElement::FogColorChange { from: (0.0,0.0,0.0), to: (0.0,0.0,0.0) },
                    ShaderChangeElement::ChangeSunPos { from: -Vec3D::all_ones(), to: -Vec3D::all_ones() },
                    ShaderChangeElement::Deactivate,
                ], ShaderChangeDuration::RealTime { duration: Duration::from_secs_f32(TIME_TO_RASTER_END_SECTION) })
            ]),
            ShaderSequence::new(vec![
                ShaderChange::new(vec![
                    ShaderChangeElement::FogDistanceChange { from: 10.0, to: 100.0 },
                    ShaderChangeElement::Activate,
                ], ShaderChangeDuration::RealTime { duration: Duration::from_secs_f32(RASTER_END_TIME) })
            ])
        ])
    )
}
