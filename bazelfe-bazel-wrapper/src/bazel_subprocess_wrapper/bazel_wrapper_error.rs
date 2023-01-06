use thiserror::Error;

#[derive(Error, Debug)]
pub enum BazelWrapperError {
    #[error("Reporting user error: `{0}`")]
    UserErrorReport(super::UserReportError),
    #[error("Unclassified or otherwise unknown error occured: `{0:?}`")]
    Unknown(Box<dyn std::error::Error>),
}
