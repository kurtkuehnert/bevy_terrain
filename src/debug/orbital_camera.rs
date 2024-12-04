use crate::big_space::{GridTransform, GridTransformItem, ReferenceFrames};
use crate::picking::PickingReadback;
use crate::prelude::TerrainViewComponents;
use bevy::{
    color::palettes::basic,
    input::{mouse::AccumulatedMouseMotion, ButtonInput},
    math::{DQuat, DVec2, DVec3, Mat4, Vec2},
    prelude::*,
    window::{CursorGrabMode, PrimaryWindow},
};

fn ray_sphere_intersection(
    ray_origin: DVec3,
    ray_direction: DVec3,
    sphere_origin: DVec3,
    radius: f64,
) -> Option<DVec3> {
    let oc = ray_origin - sphere_origin;
    let b = 2.0 * oc.dot(ray_direction);
    let c = oc.dot(oc) - radius * radius;

    let sqrt_discriminant = (b * b - 4.0 * c).sqrt();

    if sqrt_discriminant.is_nan() {
        return None; // No intersection
    }

    // Compute the roots of the quadratic equation
    let t1 = (-b - sqrt_discriminant) / 2.0;
    let t2 = (-b + sqrt_discriminant) / 2.0;

    [t1, t2]
        .into_iter()
        .filter(|&t| t >= 0.0)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .map(|t| ray_origin + ray_direction * t)
}

#[derive(Clone, Copy, Debug)]
pub struct PanData {
    pan_coords: Vec2,
    world_from_clip: Mat4,
}

#[derive(Clone, Copy, Debug)]
pub struct ZoomData {
    target_zoom: f64,
    zoom: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct RotationData {
    target_rotation: DVec2,
    rotation: DVec2,
    initial_tilt: f64,
}

#[derive(Clone, Debug, Component)]
pub struct OrbitalCameraController {
    enabled: bool,
    controlling_terrain: Option<Entity>,
    picking_priority: Vec<(Entity, Entity)>,
    cursor_coords: Vec2,
    anchor_position: DVec3,
    camera_position: DVec3,
    camera_rotation: DQuat,
    pan_data: Option<PanData>,
    zoom_data: Option<ZoomData>,
    rotation_data: Option<RotationData>,
    time_to_reach_target: f64,
}

impl OrbitalCameraController {
    pub fn new(picking_priority: impl Into<Vec<(Entity, Entity)>>) -> Self {
        Self {
            enabled: true,
            controlling_terrain: None,
            zoom_data: None,
            pan_data: None,
            rotation_data: None,
            time_to_reach_target: 0.1,
            cursor_coords: Vec2::ZERO,
            anchor_position: Default::default(),
            camera_position: Default::default(),
            camera_rotation: Default::default(),
            picking_priority: picking_priority.into(),
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn orbital_camera_controller(
    mut gizmos: Gizmos,
    frames: ReferenceFrames,
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_move: Res<AccumulatedMouseMotion>,
    picking_readbacks: Res<TerrainViewComponents<PickingReadback>>,
    mut camera: Query<(Entity, GridTransform, &mut OrbitalCameraController)>,
    mut window: Query<&mut Window, With<PrimaryWindow>>,
) {
    let (
        camera,
        GridTransformItem {
            mut transform,
            cell,
        },
        mut controller,
    ) = camera.single_mut();

    keyboard
        .just_pressed(KeyCode::KeyR)
        .then(|| controller.enabled = !controller.enabled);

    if !controller.enabled {
        return;
    }

    let frame = frames.parent_frame(camera).unwrap();

    for (terrain, view) in controller.picking_priority.clone() {
        match controller.controlling_terrain {
            Some(controlling_terrain) if controlling_terrain != terrain => continue,
            _ => {}
        };

        let readback_data = &picking_readbacks.get(&(terrain, view)).unwrap().data;

        let terrain_origin = DVec3::ZERO;
        let camera_rotation = transform.rotation.as_dquat();
        let camera_position = frame.grid_position_double(&cell, &transform);
        let cursor_position = readback_data
            .world_position
            .is_finite()
            .then(|| readback_data.world_position.as_dvec3());
        let cursor_coords = readback_data.cursor_coords;

        let smoothing = (time.delta_secs_f64() / controller.time_to_reach_target).min(1.0);

        let mut window = window.single_mut();

        let mut update_cursor_coords = true;

        if mouse_buttons.pressed(MouseButton::Left) {
            if controller.pan_data.is_none() && cursor_position.is_some() {
                controller.controlling_terrain = Some(terrain);
                controller.anchor_position = cursor_position.unwrap();
                controller.camera_position = camera_position;
                controller.camera_rotation = camera_rotation;
                controller.pan_data = Some(PanData {
                    world_from_clip: readback_data.world_from_clip,
                    pan_coords: cursor_coords,
                });
            }

            if let Some(data) = &mut controller.pan_data {
                data.pan_coords = data.pan_coords.lerp(cursor_coords, smoothing as f32);
            }
        } else {
            controller.controlling_terrain = None;
            controller.pan_data = None;
        }

        if mouse_buttons.pressed(MouseButton::Middle) {
            if controller.rotation_data.is_none() && cursor_position.is_some() {
                controller.controlling_terrain = Some(terrain);
                controller.anchor_position = cursor_position.unwrap();
                controller.camera_position = camera_position;
                controller.camera_rotation = camera_rotation;
                controller.rotation_data = Some(RotationData {
                    target_rotation: DVec2::ZERO,
                    rotation: DVec2::ZERO,
                    initial_tilt: (controller.anchor_position - terrain_origin)
                        .angle_between(controller.camera_position - controller.anchor_position),
                });
            } else {
                update_cursor_coords = false;
            }

            let rotation_speed = 0.005;

            if let Some(data) = controller.rotation_data.as_mut() {
                // Todo: fix tilt clamping
                data.target_rotation -= mouse_move.delta.as_dvec2() * rotation_speed;
                data.target_rotation.y = data.target_rotation.y.clamp(
                    -data.initial_tilt,
                    std::f64::consts::FRAC_PI_2 - data.initial_tilt,
                );

                data.rotation = data.rotation.lerp(data.target_rotation, smoothing);
            }
        } else {
            controller.controlling_terrain = None;
            controller.rotation_data = None;
        }

        if mouse_buttons.pressed(MouseButton::Right) {
            if controller.zoom_data.is_none() && cursor_position.is_some() {
                controller.controlling_terrain = Some(terrain);
                controller.anchor_position = cursor_position.unwrap();
                controller.camera_position = camera_position;
                controller.camera_rotation = camera_rotation;

                let zoom = (cursor_position.unwrap() - camera_position).length().log2();

                controller.zoom_data = Some(ZoomData {
                    target_zoom: zoom,
                    zoom,
                });
            } else {
                update_cursor_coords = false;
            }

            let zoom_speed = 0.01;

            if let Some(data) = controller.zoom_data.as_mut() {
                data.target_zoom -= mouse_move.delta.element_sum() as f64 * zoom_speed;
                data.zoom = data.zoom.lerp(data.target_zoom, smoothing);
            }
        } else {
            controller.controlling_terrain = None;
            controller.zoom_data = None;
        }

        // Todo: add support for scroll wheel zoom

        if update_cursor_coords {
            if window.cursor_options.grab_mode == CursorGrabMode::Locked {
                window.cursor_options.grab_mode = CursorGrabMode::None;
                let window_size = window.size();
                window.set_cursor_position(Some(controller.cursor_coords * window_size));
            }

            controller.cursor_coords = cursor_coords;
        } else {
            window.cursor_options.grab_mode = CursorGrabMode::Locked;
        }

        let anchor_size = 200.0;

        let mut new_camera_position = camera_position;
        let mut new_camera_rotation = camera_rotation;

        if let Some(pan_data) = controller.pan_data {
            // Invariants:
            // The anchor world position remains at the screen space position of the cursor.
            // The terrain is just rotated, but not translated relative to the camera.

            let new_cursor_coords =
                Vec2::new(pan_data.pan_coords.x, 1.0 - pan_data.pan_coords.y).as_dvec2();
            let ndc_coords = (new_cursor_coords * 2.0 - 1.0).extend(0.0001); // Todo: using f64 we should be able to set this to 1.0 for the near plane

            let camera_cursor_direction = (pan_data
                .world_from_clip
                .project_point3(ndc_coords.as_vec3())
                .as_dvec3()
                - controller.camera_position)
                .normalize();

            let radius = (controller.anchor_position - terrain_origin).length();

            // compute ray sphere intersection, where the sphere has a radius of the length of the anchor position
            // this way the anchor point should line up correctly with the cursor
            let Some(new_cursor_position) = ray_sphere_intersection(
                controller.camera_position,
                camera_cursor_direction,
                terrain_origin,
                radius,
            ) else {
                controller.pan_data = None;
                return;
            };

            // based of the anchor position and the cursor hit position compute the new camera transform
            // the world origin should stay at the center of the screen
            let initial_direction = (controller.anchor_position - terrain_origin).normalize();
            let new_direction = (new_cursor_position - terrain_origin).normalize();

            // the camera should be rotated by this amount, so that the panning anchor ends up under the cursor
            let rotation = DQuat::from_rotation_arc(new_direction, initial_direction);

            new_camera_position =
                terrain_origin + rotation * (controller.camera_position - terrain_origin);
            new_camera_rotation = rotation * controller.camera_rotation;
        }

        if let Some(rotation_data) = controller.rotation_data {
            // Invariants:
            // The cursor world position stays at the same screen-space location.
            // The distance between anchor and camera remains constant.

            let heading_axis = (controller.anchor_position - terrain_origin).normalize(); // terrain normal
            let tilt_axis = controller.camera_rotation * DVec3::X; // camera right direction

            let rotation_heading = DQuat::from_axis_angle(heading_axis, rotation_data.rotation.x);
            let rotation_tilt = DQuat::from_axis_angle(tilt_axis, rotation_data.rotation.y);
            let rotation = rotation_heading * rotation_tilt;

            new_camera_position = controller.anchor_position
                + rotation * (controller.camera_position - controller.anchor_position);
            new_camera_rotation = rotation * controller.camera_rotation;
        }

        if let Some(zoom_data) = controller.zoom_data {
            // Invariants:
            // The terrain origin stays at the screen center.
            // The cursor world position stays at the same screen-space location.

            let anchor_terrain = controller.anchor_position - terrain_origin;
            let camera_terrain = terrain_origin - controller.camera_position;
            let camera_anchor = controller.anchor_position - controller.camera_position;

            // compute the side lengths and the angles of the triangle anchor - terrain origin - new camera
            let a = anchor_terrain.length();
            let b = 2.0_f64.powf(zoom_data.zoom);

            let alpha = camera_terrain.angle_between(camera_anchor);
            let beta = (b / a * alpha.sin()).asin();
            let gamma = std::f64::consts::PI - alpha - beta;

            let c = f64::sqrt(a * a + b * b - 2.0 * a * b * gamma.cos());

            if beta.is_nan() {
                controller.zoom_data = None;
                return;
            }

            // rotation from the anchor direction towards the initial camera direction
            let rotation =
                DQuat::from_axis_angle(camera_terrain.cross(camera_anchor).normalize(), beta);

            let camera_position = terrain_origin + rotation * (c * anchor_terrain.normalize());

            let initial_direction = camera_terrain.normalize();
            let new_direction = (terrain_origin - camera_position).normalize();

            new_camera_position = camera_position;
            new_camera_rotation = DQuat::from_rotation_arc(initial_direction, new_direction)
                * controller.camera_rotation;
        }

        let anchor_position = if controller.pan_data.is_none()
            && controller.rotation_data.is_none()
            && controller.zoom_data.is_none()
        {
            cursor_position.unwrap_or(DVec3::NAN)
        } else {
            controller.anchor_position
        };

        gizmos.sphere(
            anchor_position.as_vec3(),
            new_camera_position.distance(anchor_position) as f32 / anchor_size,
            basic::GREEN,
        );

        transform.translation = new_camera_position.as_vec3();
        transform.rotation = new_camera_rotation.as_quat();
    }
}
