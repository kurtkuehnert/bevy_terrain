use crate::big_space::{FloatingOrigin, GridCell, GridTransform, RootReferenceFrame};

use bevy::{input::mouse::MouseMotion, math::DVec3, prelude::*};

#[derive(Bundle)]
pub struct DebugCameraBundle {
    pub camera: Camera3dBundle,
    pub controller: DebugCameraController,
    pub cell: GridCell,
    pub origin: FloatingOrigin,
}

impl Default for DebugCameraBundle {
    fn default() -> Self {
        Self {
            camera: default(),
            controller: default(),
            cell: default(),
            origin: FloatingOrigin,
        }
    }
}

impl DebugCameraBundle {
    pub fn new(position: DVec3, speed: f64, frame: &RootReferenceFrame) -> Self {
        let (cell, translation) = frame.translation_to_grid(position);

        Self {
            camera: Camera3dBundle {
                transform: Transform::from_translation(translation).looking_to(Vec3::X, Vec3::Y),
                projection: PerspectiveProjection {
                    near: 0.001,
                    ..default()
                }
                .into(),
                ..default()
            },
            cell,
            controller: DebugCameraController {
                translation_speed: speed,
                ..default()
            },
            ..default()
        }
    }
}

#[derive(Clone, Debug, Reflect, Component)]
pub struct DebugCameraController {
    pub enabled: bool,
    /// Smoothness of translation, from `0.0` to `1.0`.
    pub translational_smoothness: f64,
    /// Smoothness of rotation, from `0.0` to `1.0`.
    pub rotational_smoothness: f32,
    pub translation_speed: f64,
    pub rotation_speed: f32,
    pub acceleration_speed: f64,
    pub translation_velocity: DVec3,
    pub rotation_velocity: Vec2,
}

impl Default for DebugCameraController {
    fn default() -> Self {
        Self {
            enabled: false,
            translational_smoothness: 0.9,
            rotational_smoothness: 0.8,
            translation_speed: 10e1,
            rotation_speed: 1e-1,
            acceleration_speed: 4.0,
            translation_velocity: Default::default(),
            rotation_velocity: Default::default(),
        }
    }
}

pub fn camera_controller(
    frame: Res<RootReferenceFrame>,
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut mouse_move: EventReader<MouseMotion>,
    mut camera: Query<(GridTransform, &mut DebugCameraController)>,
) {
    let (mut position, mut controller) = camera.single_mut();

    keyboard
        .just_pressed(KeyCode::KeyT)
        .then(|| controller.enabled = !controller.enabled);

    if !controller.enabled {
        return;
    }

    let mut translation_direction = DVec3::ZERO; // x: left/right, y: up/down, z: forward/backward
    let rotation_direction = mouse_move.read().map(|m| -m.delta).sum::<Vec2>(); // x: yaw, y: pitch, z: roll
    let mut acceleration = 0.0;

    keyboard
        .pressed(KeyCode::ArrowLeft)
        .then(|| translation_direction.x -= 1.0);
    keyboard
        .pressed(KeyCode::ArrowRight)
        .then(|| translation_direction.x += 1.0);
    keyboard
        .pressed(KeyCode::PageUp)
        .then(|| translation_direction.y += 1.0);
    keyboard
        .pressed(KeyCode::PageDown)
        .then(|| translation_direction.y -= 1.0);
    keyboard
        .pressed(KeyCode::ArrowUp)
        .then(|| translation_direction.z -= 1.0);
    keyboard
        .pressed(KeyCode::ArrowDown)
        .then(|| translation_direction.z += 1.0);
    keyboard.pressed(KeyCode::Home).then(|| acceleration -= 1.0);
    keyboard.pressed(KeyCode::End).then(|| acceleration += 1.0);

    translation_direction = position.transform.rotation.as_dquat() * translation_direction;

    let dt = time.delta_seconds_f64();
    let lerp_translation = 1.0 - controller.translational_smoothness.clamp(0.0, 0.999);
    let lerp_rotation = 1.0 - controller.rotational_smoothness.clamp(0.0, 0.999);

    let translation_velocity_target = translation_direction * controller.translation_speed * dt;
    let rotation_velocity_target = rotation_direction * controller.rotation_speed * dt as f32;

    controller.translation_velocity = controller
        .translation_velocity
        .lerp(translation_velocity_target, lerp_translation);
    controller.rotation_velocity = controller
        .rotation_velocity
        .lerp(rotation_velocity_target, lerp_rotation);
    controller.translation_speed *= 1.0 + acceleration * controller.acceleration_speed * dt;

    let (cell_delta, translation_delta) =
        frame.translation_to_grid(controller.translation_velocity);

    let (yaw, pitch, _) = position.transform.rotation.to_euler(EulerRot::YXZ);
    let new_yaw = (yaw + controller.rotation_velocity.x) % std::f32::consts::TAU;
    let new_pitch = (pitch + controller.rotation_velocity.y)
        .clamp(-std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2);

    *position.cell += cell_delta;
    position.transform.translation += translation_delta;
    position.transform.rotation = Quat::from_euler(EulerRot::YXZ, new_yaw, new_pitch, 0.0);
}
