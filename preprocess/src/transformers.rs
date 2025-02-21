use crate::{
    gdal_extension::{GDALCustomTransformer, GDALTransformerInfo, Transformer},
    result::{PreprocessError, PreprocessResult},
};
use bevy_terrain::math::Coordinate;
use gdal::{Dataset, GeoTransform, GeoTransformEx, errors::GdalError, spatial_ref::SpatialRef};
use gdal_sys::{
    GDALCreateReprojectionTransformerEx, GDALDestroyReprojectionTransformer,
    GDALReprojectionTransform,
};
use glam::{DVec2, DVec3};
use itertools::izip;
use std::ffi::c_void;
use std::ptr;

impl Transformer for GeoTransform {
    fn transform(
        &mut self,
        dst_to_src: bool,
        x: &mut [f64],
        y: &mut [f64],
        _: &mut [f64],
        _: &mut [bool],
    ) -> PreprocessResult<()> {
        let transform = if dst_to_src {
            self
        } else {
            &mut self.invert()?
        };

        for (x, y) in x.iter_mut().zip(y.iter_mut()) {
            (*x, *y) = transform.apply(*x, *y);
        }

        Ok(())
    }
}

pub struct ReprojectionTransformer {
    ptr: *mut c_void,
    counter: u32,
}

impl ReprojectionTransformer {
    fn new(src_spatial_ref: &SpatialRef, dst_spatial_ref: &SpatialRef) -> PreprocessResult<Self> {
        let ptr = unsafe {
            GDALCreateReprojectionTransformerEx(
                src_spatial_ref.to_c_hsrs(),
                dst_spatial_ref.to_c_hsrs(),
                ptr::null(),
            )
        };
        if ptr.is_null() {
            return Err(GdalError::NullPointer {
                method_name: "GDALCreateReprojectionTransformerEx",
                msg: "Creating the transformer failed".to_string(),
            }
            .into());
        }

        Ok(Self { ptr, counter: 0 })
    }
}

impl Drop for ReprojectionTransformer {
    fn drop(&mut self) {
        unsafe { GDALDestroyReprojectionTransformer(self.ptr) }
    }
}

impl Transformer for ReprojectionTransformer {
    fn transform(
        &mut self,
        dst_to_src: bool,
        x: &mut [f64],
        y: &mut [f64],
        z: &mut [f64],
        success: &mut [bool],
    ) -> PreprocessResult<()> {
        let mut success_int = vec![0; x.len()];

        self.counter += 1;
        //dbg!(self.counter);

        //dbg!(thread::current().id());

        let return_value = unsafe {
            GDALReprojectionTransform(
                self.ptr,
                dst_to_src.into(),
                x.len().try_into().unwrap(),
                x.as_mut_ptr(),
                y.as_mut_ptr(),
                z.as_mut_ptr(),
                success_int.as_mut_ptr(),
            )
        };

        if return_value == 0 {
            return Err(PreprocessError::TransformOperationFailed);
        }

        for (success_bool, &success_int) in success.iter_mut().zip(success_int.iter()) {
            *success_bool = *success_bool && success_int != 0;
        }

        Ok(())
    }
}

struct CubeTransformer {
    face: u32,
}

impl CubeTransformer {
    fn new(face: u32) -> Self {
        Self { face }
    }
}

impl Transformer for CubeTransformer {
    fn transform(
        &mut self,
        dst_to_src: bool,
        lon_or_u: &mut [f64],
        lat_or_v: &mut [f64],
        _: &mut [f64],
        success: &mut [bool],
    ) -> PreprocessResult<()> {
        // Todo: convert to and from spherical to ellipsoidal lat/lon
        // Todo: check unit <--> lat/lon

        if dst_to_src {
            for (lon_or_u, lat_or_v, success) in
                izip!(lon_or_u.iter_mut(), lat_or_v.iter_mut(), success.iter_mut())
            {
                let coordinate = Coordinate::new(self.face, DVec2::new(*lon_or_u, *lat_or_v));
                let unit_position = coordinate.unit_position(true);

                let lon = unit_position.z.atan2(-unit_position.x);
                let lat = unit_position.y.asin();

                *success = *success && !lat.is_nan();
                *lon_or_u = lon.to_degrees();
                *lat_or_v = lat.to_degrees();
            }
        } else {
            for (lon_or_u, lat_or_v, success) in
                izip!(lon_or_u.iter_mut(), lat_or_v.iter_mut(), success.iter_mut())
            {
                let lon = lon_or_u.to_radians();
                let lat = lat_or_v.to_radians();

                let unit_position =
                    DVec3::new(-lat.cos() * lon.cos(), lat.sin(), lat.cos() * lon.sin());

                let coordinate = Coordinate::from_unit_position(unit_position, true);

                *success = *success
                    && (unit_position.length() - 1.0).abs() < 0.00001
                    && coordinate.face == self.face;
                *lon_or_u = coordinate.uv.x;
                *lat_or_v = coordinate.uv.y;
            }
        }
        Ok(())
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn clone_custom_transformer(arg: *mut c_void, _: f64, _: f64) -> *mut c_void {
    // this assumes, that the transformer is thread safe and stateless
    // Otherwise, a new custom transformer should be created.
    // However I have no idea, how it should be allocated and deallocated.

    // Since we do not implement the destroy transformer function, cleanup should be handled
    // when the custom transformer is dropped.
    // Also all fields of the custom transformer should be thread safe.
    // However, we wrap the Reprojection transformer, which should in theory be cloned.
    // It does not have a create similar function, but instead has to be serialized to XML.
    // Using GDALDeserializeTransformer, a copy of this transformer can then be instantiated.
    // Then, this new pointer has to be stored in a list inside of this custom transformer.
    // Finally, in the drop method, all of these transformers have to be deallocated.
    // When accessing the transformer, it should be looked up inside the list, based on the thread id.

    arg
}

#[repr(C)]
pub struct CustomTransformer {
    src_inverse_geo_transform: GeoTransform,
    dst_geo_transform: Option<GeoTransform>,
    lon_lat_transformer: ReprojectionTransformer,
    cube_transformer: CubeTransformer,
}

impl CustomTransformer {
    pub fn new(
        src: &Dataset,
        face: u32,
        dst_geo_transform: Option<GeoTransform>,
    ) -> PreprocessResult<GDALCustomTransformer> {
        Ok(GDALCustomTransformer {
            info: GDALTransformerInfo::new(clone_custom_transformer),
            inner: Box::new(Self {
                src_inverse_geo_transform: src.geo_transform()?.invert()?,
                dst_geo_transform,
                lon_lat_transformer: ReprojectionTransformer::new(
                    &src.spatial_ref()?,
                    &SpatialRef::from_proj4("+proj=lonlat +ellps=WGS84 +datum=WGS84")?,
                )?,
                cube_transformer: CubeTransformer::new(face),
            }),
        })
    }
}

impl Transformer for CustomTransformer {
    fn transform(
        &mut self,
        dst_to_src: bool,
        x: &mut [f64],
        y: &mut [f64],
        z: &mut [f64],
        success: &mut [bool],
    ) -> PreprocessResult<()> {
        // gdal suggest requires a bidirectional transformer from src pixel space, to destination uv space
        // gdal warp requires a unidirectional transformer from destination pixel space, to src pixel space

        // for some strange reason success is not correctly initialized
        for success in success.iter_mut() {
            *success = true;
        }

        if dst_to_src {
            if let Some(mut geo_transform) = self.dst_geo_transform {
                geo_transform.transform(dst_to_src, x, y, z, success)?;
            }

            self.cube_transformer
                .transform(dst_to_src, x, y, z, success)?;
            self.lon_lat_transformer
                .transform(dst_to_src, x, y, z, success)?;
            self.src_inverse_geo_transform
                .transform(dst_to_src, x, y, z, success)?;
        } else {
            // this only runs during the suggest phase
            // here we output uv coordinates directly, without applying a geo transform (we want to compute this)
            self.src_inverse_geo_transform
                .transform(dst_to_src, x, y, z, success)?;
            self.lon_lat_transformer
                .transform(dst_to_src, x, y, z, success)?;
            self.cube_transformer
                .transform(dst_to_src, x, y, z, success)?;
        }

        Ok(())
    }
}
