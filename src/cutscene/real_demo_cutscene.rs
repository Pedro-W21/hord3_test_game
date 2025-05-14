use std::{f32::consts::PI, time::Duration};

use hord3::{defaults::default_rendering::vectorinator_binned::rendering_spaces::ViewportData, horde::geometry::{rotation::Orientation, vec3d::{Vec3D, Vec3Df}}};

use crate::gui_elements::centered_title::get_centered_title;

use super::{camera_movement::{CameraMovement, CameraMovementDuration, CameraMovementElement, CameraSequence}, cutscene_gui::{UIMovement, UIMovementDuration, UIMovementElement, UISequence}, CameraCutscene, EntityCutscene, FullCutscene, GUICutscene, ShaderCutscene};



pub fn get_real_demo_cutscene(viewport_data:&ViewportData) -> FullCutscene {
    FullCutscene::new(
        GUICutscene::new(
            vec![
            ]
        ),
        EntityCutscene::new(vec![
            // PRE COOL
            
        ]),
        CameraCutscene::new(vec![
            CameraSequence::new(vec![
                CameraMovement::new(
                    vec![
                        CameraMovementElement::MoveFromToLinear { from: Vec3Df::new(200.0, 0.0, 80.0), to: Vec3Df::new(-10.0, 0.0, 40.0) },
                        CameraMovementElement::RotateFromToLinear { from: Orientation::new(PI + PI/2.0, 0.0, PI/2.0 + PI/6.0), to: Orientation::new(PI + PI/2.0, 0.0, PI/2.0 + PI/6.0) }
                        ],
                    CameraMovementDuration::RealTime { duration: Duration::from_secs_f32(22.0) }
                )
            ])
        ]),
        ShaderCutscene::new(vec![
        ])
    )
}