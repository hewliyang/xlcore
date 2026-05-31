use xlcore_types::{ApiError, ApiErrorCode};

pub(crate) fn load_err_to_api(value: xlcore_io::XlsxLoadError) -> ApiError {
    let mut err = ApiError::new(ApiErrorCode::Other, value.to_string());
    if let xlcore_io::XlsxLoadError::Schema { part, .. } = value {
        err.part = Some(part);
    }
    err
}

pub(crate) fn sdk_err_to_api(value: ooxmlsdk::common::SdkError) -> ApiError {
    ApiError::new(ApiErrorCode::Other, value.to_string())
}

pub(crate) fn anyhow_err_to_api(value: anyhow::Error) -> ApiError {
    ApiError::new(ApiErrorCode::Other, value.to_string())
}

pub(crate) fn zip_err(err: impl std::fmt::Display) -> ApiError {
    ApiError::new(ApiErrorCode::OoxmlWriteError, err.to_string())
}
