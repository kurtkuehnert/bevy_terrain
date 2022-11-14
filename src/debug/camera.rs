use bevy::{input::mouse::MouseMotion, prelude::*};
use dolly::prelude::*;

// Todo: unify dolly glam and bevy glam

/// A fly camera used to navigate and debug the terrain.
///
/// It is controlled using the arrow keys, and the mouse.
#[derive(Component)]
pub struct DebugCamera {
    pub rig: CameraRig<RightHanded>,
    pub active: bool,
    pub translation_speed: f32,
    pub rotation_speed: f32,
    pub acceleration: f32,
}

impl Default for DebugCamera {
    fn default() -> Self {
        Self {
            rig: CameraRig::builder()
                .with(Position::new(dolly::glam::Vec3::new(0.0, 100.0, 0.0)))
                .with(YawPitch {
                    yaw_degrees: -135.0,
                    pitch_degrees: 0.0,
                })
                .with(Smooth::new_position_rotation(3.0, 1.5))
                .build(),
            active: false,
            translation_speed: 100.0,
            rotation_speed: 8.0,
            acceleration: 1.03,
        }
    }
}

impl DebugCamera {
    pub fn new(position: Vec3, yaw_degrees: f32, pitch_degrees: f32) -> Self {
        Self {
            rig: CameraRig::builder()
                .with(Position::new(dolly::glam::Vec3::new(
                    position.x, position.y, position.z,
                )))
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
        }
    }
}

pub(crate) fn debug_camera_control(
    time: Res<Time>,
    mut motion_events: EventReader<MouseMotion>,
    keys: Res<Input<KeyCode>>,
    mut camera_rig_query: Query<(&mut Transform, &mut DebugCamera)>,
) {
    let delta_time = time.delta_seconds();

    if let Some((_, mut camera)) = camera_rig_query
        .iter_mut()
        .find(|(_, camera)| camera.active)
    {
        let mut speed_factor = 1.0;
        let mut rotation_delta = Vec2::ZERO;
        let mut translation_delta = dolly::glam::Vec3::ZERO;

        for motion in motion_events.iter() {
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
            speed_factor = 1.0 / camera.acceleration;
        }
        if keys.pressed(KeyCode::End) {
            speed_factor = camera.acceleration / 1.0;
        }

        camera.translation_speed *= speed_factor;

        if translation_delta != dolly::glam::Vec3::ZERO {
            translation_delta = translation_delta.normalize();
        }

        let euler = camera
            .rig
            .final_transform
            .rotation
            .to_euler(dolly::glam::EulerRot::YXZ);
        translation_delta =
            dolly::glam::Quat::from_euler(dolly::glam::EulerRot::YXZ, euler.0, 0.0, 0.0)
                * translation_delta;

        translation_delta = translation_delta * camera.translation_speed * delta_time;
        rotation_delta = rotation_delta * camera.rotation_speed * delta_time;

        camera
            .rig
            .driver_mut::<YawPitch>()
            .rotate_yaw_pitch(rotation_delta.x, rotation_delta.y);
        camera
            .rig
            .driver_mut::<Position>()
            .translate(translation_delta);
    } else {
        for _ in motion_events.iter() {}
    }

    for (mut transform, mut camera) in &mut camera_rig_query {
        let (translation, rotation) = camera.rig.update(delta_time).into_position_rotation();
        transform.translation = Vec3::new(translation.x, translation.y, translation.z);
        transform.rotation = Quat::from_array(rotation.to_array());
    }
}
