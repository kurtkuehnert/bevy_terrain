use gdal::errors::GdalError;
use std::num::ParseFloatError;
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum PreprocessError {
    #[error("unknown rasterband data type")]
    UnknownRasterbandDataType,
    #[error("transform operation failed")]
    TransformOperationFailed,
    #[error("The no data value is outside of the datatypes range.")]
    NoDataOutOfRange,
    #[error("GDAL error")]
    Gdal(#[from] GdalError),
    #[error("Parse error")]
    Parse(#[from] ParseFloatError),
}

pub type PreprocessResult<T> = Result<T, PreprocessError>;
