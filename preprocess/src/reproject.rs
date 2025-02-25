use crate::{
    dataset::{FaceInfo, PreprocessContext, create_empty_dataset},
    gdal_extension::{GDALCustomTransformer, ProgressCallback, SuggestedWarpOutput, warp},
    result::PreprocessResult,
    transformers::CustomTransformer,
};
use bevy_terrain::prelude::AttachmentLabel;
use gdal::{Dataset, GeoTransform, GeoTransformEx, raster::GdalType};
use glam::{DVec2, IVec2, U64Vec2};
use itertools::Itertools;
use std::collections::HashMap;

pub struct Transform<'a> {
    pub transformer: GDALCustomTransformer,
    pub face: u32,
    pub lod: u32,
    pub size: U64Vec2,
    pub geo_transform: GeoTransform,
    pub uv_start: DVec2,
    pub uv_end: DVec2,
    pub pixel_start: IVec2,
    pub pixel_end: IVec2,
    pub progress_callback: Option<Box<ProgressCallback<'a>>>,
}

pub fn reproject<T: Copy + GdalType>(
    src_dataset: Dataset,
    context: &mut PreprocessContext,
    progress_callback: Option<&ProgressCallback>,
) -> PreprocessResult<HashMap<u32, FaceInfo>> {
    if let Some(progress_callback) = progress_callback {
        progress_callback(0.0);
    }

    let mut transforms = compute_transforms(&src_dataset, context, progress_callback)?;

    let faces = transforms
        .iter_mut()
        .map(|transform| {
            let dst_path = context.temp_dir.join(format!("face{}.tif", transform.face));
            let dst_dataset = create_empty_dataset::<T>(
                &dst_path,
                transform.size,
                Some(transform.geo_transform),
                &context,
            )?;

            warp(
                &src_dataset,
                &dst_dataset,
                &context,
                &mut transform.transformer,
                transform.progress_callback.as_deref(),
            )?;

            if matches!(context.attachment_label, AttachmentLabel::Height) {
                let min_max = dst_dataset
                    .rasterband(1)
                    .unwrap()
                    .compute_raster_min_max(true)
                    .unwrap();

                context.min_height = context.min_height.min(min_max.min as f32);
                context.max_height = context.max_height.max(min_max.max as f32);
            }

            Ok((
                transform.face,
                FaceInfo {
                    lod: transform.lod,
                    pixel_start: transform.pixel_start,
                    pixel_end: transform.pixel_end,
                    path: dst_path,
                },
            ))
        })
        .collect::<PreprocessResult<HashMap<_, _>>>()?;

    Ok(faces)
}

pub fn compute_transforms<'a>(
    src_dataset: &Dataset,
    context: &mut PreprocessContext,
    progress_callback: Option<&'a ProgressCallback>,
) -> PreprocessResult<Vec<Transform<'a>>> {
    let mut transforms = Vec::with_capacity(6);

    let mut total_area = 0.0;

    for face in 0..6 {
        let mut transformer = CustomTransformer::new(src_dataset, face, None)?;

        let Some(SuggestedWarpOutput {
            size,
            mut geo_transform,
        }) = SuggestedWarpOutput::compute(src_dataset, &mut transformer)?
        else {
            continue;
        };

        // flip y axis
        geo_transform[3] = geo_transform[3] + geo_transform[5] * size.y as f64;
        geo_transform[5] = -geo_transform[5];

        let uv_start = DVec2::from(geo_transform.apply(0.0, 0.0)).max(DVec2::ZERO);
        let uv_end = DVec2::from(geo_transform.apply(size.x as f64, size.y as f64)).min(DVec2::ONE);

        total_area += (uv_end - uv_start).element_product();

        transforms.push(Transform {
            face,
            size,
            uv_start,
            uv_end,
            lod: 0,
            pixel_start: IVec2::ZERO,
            pixel_end: IVec2::ZERO,
            transformer,
            geo_transform,
            progress_callback: None,
        });
    }

    let max_lod = if let Some(lod_count) = context.lod_count {
        lod_count - 1
    } else {
        let mut max_lod = 0;

        for transform in &mut transforms {
            // GDAL uses a heuristic to compute the output dimensions in pixels by setting
            // the same number of pixels on the diagonal on both the input and output
            // projections. Since we have up to six different output images, this
            // heuristic must be modified a bit. Since the S2 projection with a
            // quadratic mapping is quite area-uniform, we divide the total GDAL based
            // output image into the six output images by their area proportion of the total
            // output.

            let uv_size = transform.uv_end - transform.uv_start;

            let correction = uv_size.element_product().sqrt() / total_area.sqrt();
            let size = (transform.size.as_dvec2() * correction).round();

            max_lod = max_lod.max(
                (size / context.attachment.center_size() as f64 / uv_size)
                    .max_element()
                    .log2()
                    .ceil() as u32,
            );
        }

        context.lod_count = Some(max_lod + 1);

        max_lod
    };

    let pixel_size = 1.0 / ((1 << max_lod) * context.attachment.center_size()) as f64;

    for transform in &mut transforms {
        let pixel_start = (transform.uv_start / pixel_size).floor();
        let pixel_end = (transform.uv_end / pixel_size).ceil();

        // println!(
        //     "Snapping to the quadtree pixel grid caused the size of the reprojected dataset to be adjusted from {} to {}. This is an up-scaling of {:.2}%.",
        //     transform.size,
        //     size,
        //     (size.as_dvec2() / transform.size.as_dvec2()).element_product() * 100.0 - 100.0
        // );

        transform.lod = max_lod;
        transform.size = (pixel_end - pixel_start).as_u64vec2();
        transform.geo_transform = GeoTransform::from([
            pixel_start.x * pixel_size,
            pixel_size,
            0.0,
            pixel_start.y * pixel_size,
            0.0,
            pixel_size,
        ]);
        transform.pixel_start = pixel_start.as_ivec2();
        transform.pixel_end = pixel_end.as_ivec2();
        transform.transformer =
            CustomTransformer::new(src_dataset, transform.face, Some(transform.geo_transform))?;
    }

    let work_portions = transforms
        .iter()
        .map(|transform| transform.size.element_product())
        .collect_vec();
    let total_work = work_portions.iter().sum::<u64>() as f64;
    let callback_intervals = work_portions
        .iter()
        .scan(0, |work_done, &work_portion| {
            *work_done += work_portion;
            Some((
                (*work_done - work_portion) as f64 / total_work,
                work_portion as f64 / total_work,
            ))
        })
        .collect_vec();

    for (transform, (offset, scale)) in transforms.iter_mut().zip(callback_intervals) {
        transform.progress_callback = progress_callback.map(|progress_callback| {
            Box::new(move |completion: f64| {
                progress_callback(completion.clamp(0.0, 1.0).mul_add(scale, offset))
            }) as Box<ProgressCallback>
        })
    }

    Ok(transforms)
}
