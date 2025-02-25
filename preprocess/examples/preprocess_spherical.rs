use bevy_terrain::prelude::*;
use bevy_terrain_preprocess::prelude::*;
use gdal::raster::GdalDataType;
use std::env::set_var;

fn main() {
    unsafe {
        if true {
            set_var("RAYON_NUM_THREADS", "0");
            // set_var("GDAL_NUM_THREADS", "ALL_CPUS");
        } else {
            set_var("RAYON_NUM_THREADS", "1");
            // set_var("GDAL_NUM_THREADS", "1");
        }
    }

    let hartenstein_dtm_path = "/Volumes/ExternalSSD/saxony_data/hartenstein_dtm";
    let hartenstein_dop_path = "/Volumes/ExternalSSD/saxony_data/hartenstein_dop";
    // let hartenstein_dop_path =
    //     "/Volumes/ExternalSSD/saxony_data/hartenstein_dop/33332_5610_2_sn.tif";
    // let hartenstein_dtm_path = "/Volumes/ExternalSSD/saxony_data/hartenstein_dtm/33336_5618.tif";
    // let src_path = "/Volumes/ExternalSSD/test/test.tif";

    // let earth_path = "/Volumes/ExternalSSD/gebco_2024/gebco_small.tif";
    // let earth_path = "/Volumes/ExternalSSD/gebco_2024/gebco_medium.tif";
    let earth_path = "/Volumes/ExternalSSD/gebco_2024/gebco_large.tif";
    // let earth_path = "/Volumes/ExternalSSD/gebco_2024/gebco_huge.tif";
    // let earth_path = "/Volumes/ExternalSSD/gebco_2024/gebco_original.tif";

    // let args = Cli {
    //     src_path: vec!["assets/source_data/gebco_earth.tif".into()],
    //     terrain_path: "assets/terrains/earth".into(),
    //     temp_path: None,
    //     overwrite: true,
    //     no_data: PreprocessNoData::Source,
    //     data_type: PreprocessDataType::DataType(GdalDataType::Float32),
    //     fill_radius: 16.0,
    //     create_mask: true,
    //     lod_count: None,
    //     attachment_label: AttachmentLabel::Height,
    //     texture_size: 512,
    //     border_size: 2,
    //     mip_level_count: 1,
    //     format: AttachmentFormat::RF32,
    // };

    // let args = Cli {
    //     src_path: vec!["assets/source_data/LOS.tiff".into()],
    //     terrain_path: "assets/terrains/los".into(),
    //     temp_path: None,
    //     overwrite: true,
    //     no_data: PreprocessNoData::Source,
    //     data_type: PreprocessDataType::DataType(GdalDataType::Float32),
    //     fill_radius: 16.0,
    //     create_mask: true,
    //     lod_count: None,
    //     attachment_label: AttachmentLabel::Height,
    //     texture_size: 512,
    //     border_size: 2,
    //     mip_level_count: 1,
    //     format: AttachmentFormat::RF32,
    // };

    let args = Cli {
        src_path: vec!["/Volumes/ExternalSSD/swiss_data/swiss_large.tif".into()],
        terrain_path: "assets/terrains/swiss".into(),
        temp_path: None,
        overwrite: true,
        no_data: PreprocessNoData::NoData(10000.0),
        data_type: PreprocessDataType::DataType(GdalDataType::Float32),
        fill_radius: 32.0,
        create_mask: true,
        lod_count: None,
        attachment_label: AttachmentLabel::Height,
        texture_size: 512,
        border_size: 2,
        mip_level_count: 1,
        format: AttachmentFormat::RF32,
    };

    // let args = Cli {
    //     src_path: vec![earth_path.into()],
    //     terrain_path: "/Volumes/ExternalSSD/tiles/earth".into(),
    //     temp_path: None,
    //     overwrite: true,
    //     no_data: PreprocessNoData::Source,
    //     data_type: PreprocessDataType::DataType(GdalDataType::Float32),
    //     fill_radius: 16.0,
    //     create_mask: true,
    //
    //     lod_count: None,
    //
    //     attachment_label: AttachmentLabel::Height,
    //     texture_size: 512,
    //     border_size: 2,
    //     mip_level_count: 1,
    //     format: AttachmentFormat::RF32,
    // };

    // let args = Cli {
    //     src_path: vec![hartenstein_dtm_path.into()],
    //     terrain_path: "/Volumes/ExternalSSD/tiles/hartenstein".into(),
    //     temp_path: None,
    //     overwrite: true,
    //     no_data: PreprocessNoData::Source,
    //     data_type: PreprocessDataType::DataType(GdalDataType::Float32),
    //     fill_radius: 16.0,
    //     create_mask: true,
    //     lod_count: None,
    //     attachment_label: AttachmentLabel::Height,
    //     texture_size: 512,
    //     border_size: 2,
    //     mip_level_count: 1,
    //     format: AttachmentFormat::RF32,
    // };

    // let args = Cli {
    //     src_path: vec![hartenstein_dop_path.into()],
    //     terrain_path: "/Volumes/ExternalSSD/tiles/hartenstein".into(),
    //     temp_path: None,
    //     overwrite: true,
    //     no_data: PreprocessNoData::NoData(0.0),
    //     data_type: PreprocessDataType::DataType(GdalDataType::UInt8),
    //     fill_radius: 16.0,
    //     create_mask: false,
    //     lod_count: Some(15),
    //     attachment_label: AttachmentLabel::Custom("albedo".to_string()),
    //     texture_size: 512,
    //     border_size: 2,
    //     mip_level_count: 1,
    //     format: AttachmentFormat::RgbU8,
    // };

    // let args = Cli {
    //     src_path: vec!["/Volumes/ExternalSSD/scope_data/LOS-99-00-01_LonLat_200m_argeo_warped_EPSG32631_negated.tiff".into()],
    //     terrain_path: "/Volumes/ExternalSSD/tiles/scope".into(),
    //     temp_path: None,
    //     overwrite: true,
    //     no_data: PreprocessNoData::Source,
    //     data_type: PreprocessDataType::DataType(GdalDataType::Float32),
    //     fill_radius: 16.0,
    //     create_mask: true,
    //     lod_count: None,
    //     attachment_label: AttachmentLabel::Height,
    //     texture_size: 512,
    //     border_size: 2,
    //     mip_level_count: 1,
    //     format: AttachmentFormat::RF32,
    // };

    let (src_dataset, mut context) = PreprocessContext::from_cli(args).unwrap();

    preprocess(src_dataset, &mut context);
}
