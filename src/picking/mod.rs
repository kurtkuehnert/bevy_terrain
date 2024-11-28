use crate::big_space::{GridTransform, GridTransformItem, ReferenceFrames};
use crate::{shaders::PICKING_SHADER, util::StaticBuffer};
use bevy::color::palettes::basic;
use bevy::core_pipeline::core_3d::graph::{Core3d, Node3d};
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::math::{DMat3, DQuat, DVec2, DVec3};
use bevy::render::render_graph::{RenderGraphApp, RenderLabel, ViewNodeRunner};
use bevy::render::render_resource::binding_types::{storage_buffer, texture_depth_2d};
use bevy::render::view::ExtractedView;
use bevy::render::RenderApp;
use bevy::window::CursorGrabMode;
use bevy::{
    ecs::query::QueryItem,
    prelude::*,
    render::{
        render_graph::{self, NodeRunError, RenderGraphContext},
        render_resource::{binding_types::sampler, *},
        renderer::{RenderContext, RenderDevice, RenderQueue},
        view::ViewDepthTexture,
    },
    window::PrimaryWindow,
};
use std::ops::{Add, AddAssign, Deref};
use std::ops::{DerefMut, Sub};
use std::sync::{Arc, Mutex};
use wgpu::util::DownloadBuffer;

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
}

#[derive(Clone, Debug, Component)]
pub struct OrbitalCameraController {
    enabled: bool,
    cursor_coords: Vec2,
    anchor_position: DVec3,
    camera_position: DVec3,
    camera_rotation: DQuat,
    pan_data: Option<PanData>,
    zoom_data: Option<ZoomData>,
    rotation_data: Option<RotationData>,
    time_to_reach_target: f32,
}

impl Default for OrbitalCameraController {
    fn default() -> Self {
        Self {
            enabled: true,
            zoom_data: None,
            pan_data: None,
            rotation_data: None,
            time_to_reach_target: 0.1,
            cursor_coords: Vec2::ZERO,
            anchor_position: Default::default(),
            camera_position: Default::default(),
            camera_rotation: Default::default(),
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
    mut mouse_move: EventReader<MouseMotion>,
    mut mouse_scroll: EventReader<MouseWheel>,
    mut camera: Query<(Entity, GridTransform, &mut OrbitalCameraController)>,
    readback: Res<PickingReadback>,
    mut window: Query<&mut Window, With<PrimaryWindow>>,
) {
    let (
        camera,
        GridTransformItem {
            mut transform,
            mut cell,
        },
        mut controller,
    ) = camera.single_mut();

    let frame = frames.parent_frame(camera).unwrap();

    keyboard
        .just_pressed(KeyCode::KeyR)
        .then(|| controller.enabled = !controller.enabled);

    if !controller.enabled {
        return;
    }

    let readback_data = &readback.data;

    let terrain_origin = DVec3::ZERO;
    let camera_rotation = transform.rotation.as_dquat();
    let camera_position = frame.grid_position_double(&cell, &transform);
    let cursor_position = readback_data
        .world_position
        .is_finite()
        .then(|| readback_data.world_position.as_dvec3());
    let cursor_coords = readback_data.cursor_coords;

    let smoothing = (time.delta_seconds() / controller.time_to_reach_target).min(1.0);

    let mut window = window.single_mut();

    let mut update_cursor_coords = true;

    if mouse_buttons.pressed(MouseButton::Left) && cursor_position.is_some() {
        if controller.pan_data.is_none() {
            controller.anchor_position = cursor_position.unwrap();
            controller.camera_position = camera_position;
            controller.camera_rotation = camera_rotation;
            controller.pan_data = Some(PanData {
                world_from_clip: readback_data.world_from_clip,
                pan_coords: cursor_coords,
            });
        }

        let pan_coords = &mut controller.pan_data.as_mut().unwrap().pan_coords;
        *pan_coords = pan_coords.lerp(cursor_coords, smoothing);
    } else {
        controller.pan_data = None;
    }

    if mouse_buttons.pressed(MouseButton::Middle) {
        if controller.rotation_data.is_none() && cursor_position.is_some() {
            controller.anchor_position = cursor_position.unwrap();
            controller.camera_position = camera_position;
            controller.camera_rotation = camera_rotation;
            controller.rotation_data = Some(RotationData {
                target_rotation: DVec2::ZERO,
                rotation: DVec2::ZERO,
            });
        } else {
            update_cursor_coords = false;
        }

        let rotation_speed = 0.01;

        if let Some(rotation_data) = controller.rotation_data.as_mut() {
            rotation_data.target_rotation += mouse_move
                .read()
                .map(|event| -event.delta.as_dvec2() * rotation_speed)
                .sum::<DVec2>();

            rotation_data.rotation = rotation_data
                .rotation
                .lerp(rotation_data.target_rotation, smoothing as f64);
        }
    } else {
        controller.rotation_data = None;
    }

    if mouse_buttons.pressed(MouseButton::Right) {
        if controller.zoom_data.is_none() && cursor_position.is_some() {
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

        if let Some(zoom_data) = controller.zoom_data.as_mut() {
            zoom_data.target_zoom += mouse_move
                .read()
                .map(|event| -event.delta.element_sum() as f64 * zoom_speed)
                .sum::<f64>();

            zoom_data.zoom = zoom_data.zoom.lerp(zoom_data.target_zoom, smoothing as f64);
        }
    } else {
        controller.zoom_data = None;
    }

    // Todo: add support for scroll wheel zoom

    if update_cursor_coords {
        if window.cursor.grab_mode == CursorGrabMode::Locked {
            window.cursor.grab_mode = CursorGrabMode::None;
            let window_size = window.size();
            window.set_cursor_position(Some(controller.cursor_coords * window_size));
        }

        controller.cursor_coords = cursor_coords;
    } else {
        window.cursor.grab_mode = CursorGrabMode::Locked;
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

        let initial_tilt = (controller.anchor_position - terrain_origin)
            .angle_between(controller.camera_position - controller.anchor_position);

        let delta_heading = rotation_data.rotation.x;
        let delta_tilt = rotation_data
            .rotation
            .y
            .clamp(-initial_tilt, std::f64::consts::FRAC_PI_2 - initial_tilt);

        // Todo: fix tilt clamping

        let rotation_heading = DQuat::from_axis_angle(heading_axis, delta_heading);
        let rotation_tilt = DQuat::from_axis_angle(tilt_axis, delta_tilt);
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
        new_camera_rotation =
            DQuat::from_rotation_arc(initial_direction, new_direction) * controller.camera_rotation;
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
        default(),
        new_camera_position.distance(anchor_position) as f32 / anchor_size,
        basic::GREEN,
    );

    transform.translation = new_camera_position.as_vec3();
    transform.rotation = new_camera_rotation.as_quat();
}

pub fn test(
    mut gizmos: Gizmos,
    mut readback: ResMut<PickingReadback>,
    window: Query<&Window, With<PrimaryWindow>>,
) {
    let PickingReadback { readback, data } = readback.deref_mut();

    let mut picking_data = readback.lock().unwrap();
    *data = picking_data.clone();

    // gizmos.sphere(data.world_position, default(), 10000.0, basic::RED);

    let window = window.single();

    if let Some(position) = window.cursor_position() {
        picking_data.cursor_coords = position / window.size();
    }
}

#[derive(Resource, Default, Clone)]
pub struct PickingReadback {
    readback: Arc<Mutex<PickingData>>,
    data: PickingData,
}

#[derive(Default, Debug, Clone, ShaderType)]
struct PickingData {
    cursor_coords: Vec2,
    depth: f32,
    world_position: Vec3,
    world_from_clip: Mat4,
}

#[derive(Resource)]
pub struct PickingPipeline {
    picking_layout: BindGroupLayout,
    id: CachedComputePipelineId,
}

impl FromWorld for PickingPipeline {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();
        let pipeline_cache = world.resource::<PipelineCache>();

        let picking_layout = device.create_bind_group_layout(
            None,
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    storage_buffer::<PickingData>(false),
                    sampler(SamplerBindingType::Filtering),
                    texture_depth_2d(),
                    // texture_2d_multisampled(TextureSampleType::Float { filterable: false }),
                ),
            ),
        );

        let id = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: None,
            layout: vec![picking_layout.clone()],
            push_constant_ranges: Vec::new(),
            shader: world.load_asset(PICKING_SHADER),
            shader_defs: vec![],
            entry_point: "pick".into(),
        });

        Self { picking_layout, id }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct PickingLabel;

#[derive(Default)]
pub struct PickingNode;

impl render_graph::ViewNode for PickingNode {
    type ViewQuery = (&'static ExtractedView, &'static ViewDepthTexture);
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (view, depth): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let device = world.resource::<RenderDevice>();
        let queue = world.resource::<RenderQueue>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let picking_pipeline = world.resource::<PickingPipeline>();
        let picking_readback = world.resource::<PickingReadback>().clone();

        let picking_data = picking_readback.readback.lock().unwrap().clone();

        let cursor_coords = Vec2::new(
            picking_data.cursor_coords.x,
            1.0 - picking_data.cursor_coords.y,
        );
        let world_from_clip = view.world_from_view.compute_matrix() * view.clip_from_view.inverse();

        let picking_buffer = StaticBuffer::<PickingData>::create(
            None,
            device,
            &picking_data,
            BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
        );

        let sampler = device.create_sampler(&SamplerDescriptor {
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..default()
        });
        let depth_view = depth.texture.create_view(&Default::default());

        let bind_group = device.create_bind_group(
            None,
            &picking_pipeline.picking_layout,
            &BindGroupEntries::sequential((&picking_buffer, &sampler, &depth_view)),
        );

        // let depth_copy_buffer = StaticBuffer::<()>::empty_sized(
        //     None,
        //     device,
        //     64,
        //     BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
        // );
        //
        // let origin = Origin3d { x: 0, y: 0, z: 0 };
        // let copy_size = Extent3d {
        //     width: 1,
        //     height: 1,
        //     depth_or_array_layers: 1,
        // };

        // render_context.command_encoder().copy_texture_to_buffer(
        //     ImageCopyTexture {
        //         texture: &depth.texture,
        //         mip_level: 0,
        //         origin,
        //         aspect: Default::default(),
        //     },
        //     ImageCopyBuffer {
        //         buffer: &depth_copy_buffer,
        //         layout: ImageDataLayout {
        //             offset: 0,
        //             bytes_per_row: Some(4 * size_of::<f32>() as u32),
        //             rows_per_image: Some(4),
        //         },
        //     },
        //     copy_size,
        // );

        let Some(pipeline) = pipeline_cache.get_compute_pipeline(picking_pipeline.id) else {
            return Ok(());
        };

        let mut command_encoder =
            device.create_command_encoder(&CommandEncoderDescriptor::default());

        let mut compute_pass =
            command_encoder.begin_compute_pass(&ComputePassDescriptor::default());

        compute_pass.set_bind_group(0, &bind_group, &[]);
        compute_pass.set_pipeline(pipeline);
        compute_pass.dispatch_workgroups(1, 1, 1);

        drop(compute_pass);

        let command_buffer = command_encoder.finish();

        queue.submit(Some(command_buffer));

        DownloadBuffer::read_buffer(
            device.wgpu_device(),
            &queue,
            &picking_buffer.slice(..),
            move |result| {
                let buffer = result.expect("Reading buffer failed!");
                let storage_buffer = encase::StorageBuffer::new(buffer.deref());

                let mut data = PickingData::default();
                storage_buffer.read(&mut data).unwrap();

                let ndc_coords = (cursor_coords * 2.0 - 1.0).extend(data.depth);

                data.world_position = world_from_clip.project_point3(ndc_coords);
                data.world_from_clip = world_from_clip;

                *picking_readback.readback.lock().unwrap() = data;
            },
        );

        Ok(())
    }
}

pub struct PickingPlugin;

impl Plugin for PickingPlugin {
    fn build(&self, app: &mut App) {
        let picking_readback = PickingReadback::default();

        app.add_systems(Update, (test, orbital_camera_controller))
            .insert_resource(picking_readback.clone());

        app.sub_app_mut(RenderApp)
            .insert_resource(picking_readback)
            .add_render_graph_node::<ViewNodeRunner<PickingNode>>(Core3d, PickingLabel)
            .add_render_graph_edge(Core3d, Node3d::EndMainPass, PickingLabel);
    }
    fn finish(&self, app: &mut App) {
        app.sub_app_mut(RenderApp)
            .init_resource::<PickingPipeline>();
    }
}
