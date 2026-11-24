//! Computer vision mode uses the same HTTP control/API server as site mode.
//! The UI page is not part of the CV client workflow; API routes remain available.

pub(crate) fn run_cv() -> Result<(), Box<dyn std::error::Error + 'static>> {
    crate::http_api::run_api_without_page()
}
