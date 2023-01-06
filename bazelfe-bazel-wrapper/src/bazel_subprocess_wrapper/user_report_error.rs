use std::error;

use std::fmt;

#[derive(Debug, Clone)]
pub struct UserReportError(pub String);

impl fmt::Display for UserReportError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Error with human error message of: {}", self.0)
    }
}

impl error::Error for UserReportError {}
