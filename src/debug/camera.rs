use bevy::{input::mouse::MouseMotion, prelude::*};
use dolly::prelude::*;

#[derive(Component)]
pub struct DebugRig {
    pub rig: CameraRig<RightHanded>,
    pub active: bool,
    pub translation_speed: f32,
    pub rotation_speed: f32,
    pub acceleration: f32,
}

/// A fly camera used to navigate and debug the terrain.
///
/// It is controlled using the arrow keys, and the mouse.
#[derive(Bundle)]
pub struct DebugCamera {
    pub camera: Camera3dBundle,
    pub rig: DebugRig,
}

impl Default for DebugCamera {
    fn default() -> Self {
        Self {
            camera: default(),
            rig: DebugRig {
                rig: CameraRig::builder()
                    .with(Position::new(Vec3::new(-150.0, 0.0, 0.0)))
                    .with(YawPitch {
                        yaw_degrees: -90.0,
                        pitch_degrees: 0.0,
                    })
                    .with(Smooth::new_position_rotation(3.0, 1.5))
                    .build(),
                active: false,
                translation_speed: 100.0,
                rotation_speed: 8.0,
                acceleration: 1.03,
            },
        }
    }
}

impl DebugCamera {
    pub fn new(position: Vec3, yaw_degrees: f32, pitch_degrees: f32) -> Self {
        Self {
            camera: default(),
            rig: DebugRig {
                rig: CameraRig::builder()
                    .with(Position::new(position))
                    .with(YawPitch {
                        yaw_degrees,
                        pitch_degrees,
                    })
                    .with(Smooth::new_position_rotation(3.0, 1.5))
                    .build(),
                active: false,
                translation_speed: 100.0,
                rotation_speed: 8.0,
                acceleration: 1.03,
            },
        }
    }
}

pub(crate) fn debug_camera_control(
    time: Res<Time>,
    mut motion_events: EventReader<MouseMotion>,
    keys: Res<Input<KeyCode>>,
    mut debug_rig_query: Query<(&mut Transform, &mut DebugRig)>,
) {
    let delta_time = time.delta_seconds();

    if let Some((_, mut rig)) = debug_rig_query.iter_mut().find(|(_, camera)| camera.active) {
        let mut speed_factor = 1.0;
        let mut rotation_delta = Vec2::ZERO;
        let mut translation_delta = Vec3::ZERO;

        for motion in motion_events.read() {
            rotation_delta += -motion.delta;
        }

        if keys.pressed(KeyCode::Left) {
            translation_delta.x -= 1.0;
        }
        if keys.pressed(KeyCode::Right) {
            translation_delta.x += 1.0;
        }
        if keys.pressed(KeyCode::PageUp) {
            translation_delta.y += 1.0;
        }
        if keys.pressed(KeyCode::PageDown) {
            translation_delta.y -= 1.0;
        }
        if keys.pressed(KeyCode::Up) {
            translation_delta.z -= 1.0;
        }
        if keys.pressed(KeyCode::Down) {
            translation_delta.z += 1.0;
        }
        if keys.pressed(KeyCode::Home) {
            speed_factor = 1.0 / rig.acceleration;
        }
        if keys.pressed(KeyCode::End) {
            speed_factor = rig.acceleration / 1.0;
        }

        rig.translation_speed *= speed_factor;

        if translation_delta != Vec3::ZERO {
            translation_delta = translation_delta.normalize();
        }

        let euler = rig.rig.final_transform.rotation.to_euler(EulerRot::YXZ);
        translation_delta = Quat::from_euler(EulerRot::YXZ, euler.0, 0.0, 0.0) * translation_delta;

        translation_delta = translation_delta * rig.translation_speed * delta_time;
        rotation_delta = rotation_delta * rig.rotation_speed * delta_time;

        rig.rig
            .driver_mut::<YawPitch>()
            .rotate_yaw_pitch(rotation_delta.x, rotation_delta.y);
        rig.rig
            .driver_mut::<Position>()
            .translate(translation_delta);
    } else {
        for _ in motion_events.read() {}
    }

    for (mut transform, mut rig) in &mut debug_rig_query {
        if keys.just_pressed(KeyCode::T) {
            rig.active = !rig.active;
        }

        let (translation, rotation) = rig.rig.update(delta_time).into_position_rotation();
        transform.translation = Vec3::new(translation.x, translation.y, translation.z);
        transform.rotation = Quat::from_array(rotation.to_array());
    }
}
