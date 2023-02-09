use crate::{
    preprocess::{
        file_io::{format_node_path, load_image, load_or_create_node, save_image},
        UVec2Utils,
    },
    skip_none,
    terrain_data::{AttachmentConfig, AttachmentFormat},
};
use bevy::prelude::*;
use image::{DynamicImage, GenericImage, GenericImageView, Luma, LumaA, Pixel, Rgb, Rgba};
use itertools::{iproduct, izip};

pub(crate) trait AveragePixel: Copy + Clone + Pixel {
    fn average(a: Self, b: Self, c: Self, d: Self) -> Self;
}

impl AveragePixel for Rgb<u8> {
    fn average(a: Self, b: Self, c: Self, d: Self) -> Self {
        let mut value = Rgb([0; 3]);
        izip!(&mut value.0, &a.0, &b.0, &c.0, &d.0).for_each(|(out, &a, &b, &c, &d)| {
            *out = ((a as f32 + b as f32 + c as f32 + d as f32) / 4.0) as u8
        });
        value
    }
}

impl AveragePixel for Rgba<u8> {
    fn average(a: Self, b: Self, c: Self, d: Self) -> Self {
        let mut value = Rgba([0; 4]);
        izip!(&mut value.0, &a.0, &b.0, &c.0, &d.0).for_each(|(out, &a, &b, &c, &d)| {
            *out = ((a as f32 + b as f32 + c as f32 + d as f32) / 4.0) as u8
        });
        value
    }
}

impl AveragePixel for Luma<u16> {
    fn average(a: Self, b: Self, c: Self, d: Self) -> Self {
        let mut value = Luma([0; 1]);
        izip!(&mut value.0, &a.0, &b.0, &c.0, &d.0).for_each(|(out, &a, &b, &c, &d)| {
            *out = ((a as f32 + b as f32 + c as f32 + d as f32) / 4.0) as u16
        });
        value
    }
}

impl AveragePixel for LumaA<u16> {
    fn average(a: Self, b: Self, c: Self, d: Self) -> Self {
        let mut value = LumaA([0; 2]);
        izip!(&mut value.0, &a.0, &b.0, &c.0, &d.0).for_each(|(out, &a, &b, &c, &d)| {
            *out = ((a as f32 + b as f32 + c as f32 + d as f32) / 4.0) as u16
        });
        value
    }
}

type Filter = fn(&mut DynamicImage, &DynamicImage, &AttachmentConfig, UVec2);

pub(crate) fn imageops_linear<I, J>(
    parent_image: &mut I,
    child_image: &J,
    child_size: u32,
    node_x: u32,
    node_y: u32,
    border_size: u32,
) where
    I: GenericImage,
    J: GenericImageView<Pixel = I::Pixel>,
    <I as GenericImageView>::Pixel: AveragePixel,
{
    for (x, y) in iproduct!(0..child_size, 0..child_size) {
        let mut values = [child_image.get_pixel(0, 0); 4];
        for (i, value) in values.iter_mut().enumerate() {
            *value = child_image.get_pixel(
                (x << 1) + border_size + (i as u32 >> 1),
                (y << 1) + border_size + (i as u32 & 1),
            )
        }

        let value = AveragePixel::average(values[0], values[1], values[2], values[3]);

        parent_image.put_pixel(node_x + x, node_y + y, value);
    }
}

pub(crate) fn linear(
    parent_image: &mut DynamicImage,
    child_image: &DynamicImage,
    attachment: &AttachmentConfig,
    offset: UVec2,
) {
    let child_size = attachment.center_size >> 1;
    let node_x = offset.x * child_size + attachment.border_size;
    let node_y = offset.y * child_size + attachment.border_size;

    match attachment.format {
        AttachmentFormat::Rgb8 => {
            imageops_linear(
                parent_image.as_mut_rgb8().unwrap(),
                child_image.as_rgb8().unwrap(),
                child_size,
                node_x,
                node_y,
                attachment.border_size,
            );
        }
        AttachmentFormat::Rgba8 => {
            imageops_linear(
                parent_image.as_mut_rgba8().unwrap(),
                child_image.as_rgba8().unwrap(),
                child_size,
                node_x,
                node_y,
                attachment.border_size,
            );
        }
        AttachmentFormat::R16 => {
            imageops_linear(
                parent_image.as_mut_luma16().unwrap(),
                child_image.as_luma16().unwrap(),
                child_size,
                node_x,
                node_y,
                attachment.border_size,
            );
        }
        AttachmentFormat::Rg16 => {
            imageops_linear(
                parent_image.as_mut_luma_alpha16().unwrap(),
                child_image.as_luma_alpha16().unwrap(),
                child_size,
                node_x,
                node_y,
                attachment.border_size,
            );
        }
    }
}

pub(crate) fn minmax(
    parent_image: &mut DynamicImage,
    child_image: &DynamicImage,
    attachment: &AttachmentConfig,
    offset: UVec2,
) {
    let parent_image = parent_image.as_mut_luma_alpha16().unwrap();
    let child_image = child_image.as_luma_alpha16().unwrap();

    let child_size = attachment.center_size >> 1;

    let node_x = offset.x * child_size + attachment.border_size;
    let node_y = offset.y * child_size + attachment.border_size;

    for (x, y) in iproduct!(0..child_size, 0..child_size) {
        let mut min = u16::MAX;
        let mut max = u16::MIN;

        for (cx, cy) in iproduct!(0..2, 0..2) {
            let value = child_image
                .get_pixel(
                    (x << 1) + cx + attachment.border_size,
                    (y << 1) + cy + attachment.border_size,
                )
                .0;
            min = min.min(value[0]);
            max = max.max(value[1]);
        }

        let value = LumaA([min, max]);
        parent_image.put_pixel(node_x + x, node_y + y, value);
    }
}

pub(crate) fn down_sample_layer(
    filter: Filter,
    directory: &str,
    attachment: &AttachmentConfig,
    lod: u32,
    first: UVec2,
    last: UVec2,
) {
    first.product(last).for_each(|(x, y)| {
        let node_path = format_node_path(directory, lod, x, y);
        let mut node_image = load_or_create_node(&node_path, attachment);

        for (cx, cy) in iproduct!(0..2, 0..2) {
            let child_path = format_node_path(directory, lod - 1, (x << 1) + cx, (y << 1) + cy);
            let child_image = skip_none!(load_image(&child_path, attachment.file_format));
            // Todo: if a child node is not available, we should fill the gap in the parent one
            // maybe this should not even be possible

            filter(
                &mut node_image,
                &child_image,
                attachment,
                UVec2::new(cx, cy),
            );
        }

        save_image(&node_path, &node_image, attachment);
    });
}
