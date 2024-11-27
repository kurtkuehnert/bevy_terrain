use crate::big_space::{GridTransform, GridTransformItem, ReferenceFrames};
use crate::debug::DebugCameraController;
use crate::{render::TilingPrepassNode, shaders::PICKING_SHADER, util::StaticBuffer};
use bevy::color::palettes::basic;
use bevy::core_pipeline::core_3d::graph::{Core3d, Node3d};
use bevy::input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel};
use bevy::math::{DQuat, DVec2, DVec3};
use bevy::render::graph::CameraDriverLabel;
use bevy::render::render_graph::{RenderGraph, RenderGraphApp, RenderLabel, ViewNodeRunner};
use bevy::render::render_resource::binding_types::{
    storage_buffer, texture_2d_multisampled, texture_depth_2d,
};
use bevy::render::view::ExtractedView;
use bevy::render::RenderApp;
use bevy::{
    ecs::query::QueryItem,
    prelude::*,
    render::{
        camera::ExtractedCamera,
        render_graph::{self, NodeRunError, RenderGraphContext},
        render_resource::{
            binding_types::{sampler, storage_buffer_sized, texture_2d},
            *,
        },
        renderer::{RenderContext, RenderDevice, RenderQueue},
        view::ViewDepthTexture,
    },
    window::PrimaryWindow,
};
use std::ops::{Add, DerefMut, Sub};
use std::sync::{Arc, Mutex};
use std::{num::NonZeroU64, ops::Deref};
use wgpu::util::DownloadBuffer;

pub fn get_mouse_position(window: Query<&Window, With<PrimaryWindow>>) {}

#[derive(Clone, Copy, Debug)]
pub struct PanData {
    anchor_position: DVec3,
    camera_transform: Transform,
    world_from_clip: Mat4,
}

#[derive(Clone, Debug, Component)]
pub struct OrbitalCameraController {
    pub enabled: bool,
    pub zoom: f64,
    pub zoom_speed: f64,
    pub rotation: DVec2,
    pub pan_data: Option<PanData>,
    pub rotation_speed: f64,
    pub rotation_anchor: Option<DVec3>,
    pub time_to_reach_target: f64,
}

impl Default for OrbitalCameraController {
    fn default() -> Self {
        Self {
            enabled: false,
            zoom: 0.0,
            zoom_speed: 0.01,
            pan_data: None,
            rotation: DVec2::ZERO,
            rotation_speed: 0.1,
            rotation_anchor: None,
            time_to_reach_target: 1.0,
        }
    }
}

fn ray_ellipsoid_intersection(
    camera_position: DVec3,
    ray_direction: DVec3,
    terrain_origin: DVec3,
    major_axes: f64,
    minor_axes: f64,
) -> Option<DVec3> {
    let a2 = major_axes * major_axes;
    let b2 = minor_axes * minor_axes;

    let cam = camera_position - terrain_origin;
    let dir = ray_direction;
    let a = (dir.x * dir.x / a2) + (dir.y * dir.y / b2) + (dir.z * dir.z / a2);
    let b = 2.0 * ((cam.x * dir.x / a2) + (cam.y * dir.y / b2) + (cam.z * dir.z / a2));
    let c = (cam.x * cam.x / a2) + (cam.y * cam.y / b2) + (cam.z * cam.z / a2) - 1.0;

    let discriminant = b * b - 4.0 * a * c;

    if discriminant < 0.0 {
        return None; // No intersection
    }

    // Compute the roots of the quadratic equation
    let sqrt_discriminant = discriminant.sqrt();
    let t1 = (-b - sqrt_discriminant) / (2.0 * a);
    let t2 = (-b + sqrt_discriminant) / (2.0 * a);

    // Find the smallest positive root (t)
    [t1, t2]
        .into_iter()
        .filter(|&t| t >= 0.0)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .map(|t| camera_position + ray_direction * t)
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
    window: Query<&Window, With<PrimaryWindow>>,
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

    let window_size = window.single().size().as_dvec2();
    let terrain_origin = DVec3::ZERO;
    let camera_position = frame.grid_position_double(&cell, &transform);
    let cursor_position = readback_data.world_position.as_dvec3();
    let cursor_coords = readback_data.cursor_coords.as_dvec2();

    controller.zoom = mouse_scroll
        .read()
        .map(|event| event.y as f64)
        .fold(controller.zoom, f64::sub);

    if mouse_buttons.pressed(MouseButton::Left) {
        controller.pan_data = controller.pan_data.or(Some(PanData {
            anchor_position: cursor_position,
            camera_transform: *transform,
            world_from_clip: readback_data.world_from_clip,
        }));
    } else {
        controller.pan_data = None;
    }

    if mouse_buttons.pressed(MouseButton::Middle) {
        controller.rotation_anchor = controller.rotation_anchor.or(Some(cursor_position));

        controller.rotation = mouse_move
            .read()
            .map(|event| event.delta.as_dvec2())
            .fold(controller.rotation, DVec2::sub);
    } else {
        controller.rotation_anchor = None;
    }

    let delta = time.delta().as_secs_f64();
    let mut new_camera_position = camera_position;

    if let Some(pan_data) = controller.pan_data {
        gizmos.sphere(
            pan_data.anchor_position.as_vec3(),
            default(),
            10000.0,
            basic::YELLOW,
        );

        let initial_camera_position = pan_data.camera_transform.translation.as_dvec3();
        let initial_camera_rotation = pan_data.camera_transform.rotation;
        let initial_terrain_camera = initial_camera_position - terrain_origin;

        let coords = DVec2::new(cursor_coords.x, 1.0 - cursor_coords.y);
        let ndc_coords = (coords * 2.0 - 1.0).extend(0.0001);

        let camera_cursor_direction = (pan_data
            .world_from_clip
            .project_point3(ndc_coords.as_vec3())
            .as_dvec3()
            - initial_camera_position)
            .normalize();

        // let radius = (pan_data.anchor_position - terrain_origin).length();

        // compute ray ellipsoid intersection
        // Todo: actually it should be a ray sphere intersection with radius of length of initial cursor position
        let Some(cursor_hit_position) = ray_ellipsoid_intersection(
            initial_camera_position,
            camera_cursor_direction,
            terrain_origin,
            6371000.0,
            6371000.0,
        ) else {
            return;
        };

        // based of the panning anchor position and the cursor hit position compute the new camera transform
        // the world origin should stay at the center of the screen
        let original_dir = (pan_data.anchor_position - terrain_origin).normalize();
        let current_dir = (cursor_hit_position - terrain_origin).normalize();

        // the world should be rotated by this amount, so that the panning anchor ends up under the cursor
        let world_rotation = DQuat::from_rotation_arc(original_dir, current_dir);

        let camera_rotation = world_rotation.inverse();

        transform.translation =
            (terrain_origin + camera_rotation * initial_terrain_camera).as_vec3();
        transform.rotation = camera_rotation.as_quat() * initial_camera_rotation;

        // transform.translation = transform.translation + Vec3::new(0.0, 100.0, 0.0);
    }

    if let Some(rotation_anchor) = controller.rotation_anchor {
        gizmos.sphere(rotation_anchor.as_vec3(), default(), 10000.0, basic::BLUE);

        let rotation_axis_x = (rotation_anchor - terrain_origin).normalize(); // terrain normal
        let rotation_axis_y = transform.right().as_dvec3(); // camera right direction

        let mut angle = delta * controller.rotation * controller.rotation_speed;

        // Todo: fix this
        let right = transform.right().as_dvec3();
        let up = transform.up().as_dvec3();
        let normal = rotation_axis_x.cross(right).normalize();
        let current_angle = std::f64::consts::FRAC_PI_2 - up.angle_between(normal);
        angle.y = (current_angle + angle.y).clamp(0.0, std::f64::consts::FRAC_PI_2) - current_angle;

        let rotation_x = DQuat::from_axis_angle(rotation_axis_x, angle.x);
        let rotation_y = DQuat::from_axis_angle(rotation_axis_y, angle.y);
        let rotation = rotation_x * rotation_y;

        let target_camera_position =
            rotation_anchor + rotation * (camera_position - rotation_anchor);

        controller.rotation = DVec2::ZERO;

        new_camera_position = target_camera_position;
        transform.rotation = rotation.as_quat() * transform.rotation;
    }

    if controller.zoom.abs() > 0.0 && cursor_position.is_finite() {
        let cursor_camera = camera_position - cursor_position;
        let terrain_cursor = cursor_position - terrain_origin;
        let terrain_camera = camera_position - terrain_origin;

        let distance_to_cursor = cursor_camera.length();
        let target_distance_to_cursor =
            distance_to_cursor.powf(1.0 + controller.zoom * controller.zoom_speed);
        let target_camera_position =
            cursor_position + cursor_camera.normalize() * target_distance_to_cursor;

        // we have to rotate the camera towards the normal at the cursor
        // let angle = terrain_cursor.angle_between(terrain_camera);
        // let target_angle = 0.0;
        // let new_angle = angle - angle * delta / 1.0;
        // let rotation_axis = terrain_cursor.cross(terrain_camera).normalize();

        // new_camera_position = camera_position.lerp(
        //     target_camera_position,
        //     (delta / controller.time_to_reach_target).min(1.0),
        // );
        //
        // // compute "used" amount of scroll
        // let new_distance_to_cursor = (new_camera_position - cursor_position).length();
        // let scroll_used =
        //     (new_distance_to_cursor.log(distance_to_cursor) - 1.0) / controller.zoom_speed;
        //
        // controller.zoom -= scroll_used;
        //
        // transform.translation = new_camera_position.as_vec3();

        transform.translation = target_camera_position.as_vec3();
        controller.zoom = 0.0;

        // let target_rotation =
        //     DQuat::from_rotation_arc(terrain_camera.normalize(), terrain_cursor.normalize());
        //
        // //dbg!(terrain_camera.angle_between(terrain_cursor));
        //
        // let rotation =
        //     DQuat::IDENTITY.slerp(target_rotation, (10.0 * scroll_used.abs() * delta).min(1.0));
        //
        // // dbg!(rotation);
        //
        // transform.translation = (terrain_origin + rotation * (new_camera_position - terrain_origin)).as_vec3();
        // transform.rotation = rotation.as_quat() * transform.rotation;
    }
}

pub fn test(
    mut gizmos: Gizmos,
    mut readback: ResMut<PickingReadback>,
    window: Query<&Window, With<PrimaryWindow>>,
) {
    let PickingReadback { readback, data } = readback.deref_mut();

    let mut picking_data = readback.lock().unwrap();
    *data = picking_data.clone();

    gizmos.sphere(data.world_position, default(), 10000.0, basic::RED);

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

        app.add_systems(
            Update,
            (get_mouse_position, test, orbital_camera_controller),
        )
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
