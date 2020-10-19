extern crate nom;

mod parser_helpers;

use nom::error::ErrorKind as NomErrorKind;
#[derive(Debug)]
pub enum Error {
    NomIncomplete(),
    NomError(String, NomErrorKind),
    NomFailure(String, NomErrorKind),
    UnexpectedRemainingData(String),
}

impl<'a> From<nom::Err<(&'a str, NomErrorKind)>> for Error {
    fn from(e: nom::Err<(&'a str, NomErrorKind)>) -> Error {
        match e {
            nom::Err::Incomplete(_) => Error::NomIncomplete(),
            nom::Err::Error((remaining, kind)) => Error::NomError(remaining.to_string(), kind),
            nom::Err::Failure((remaining, kind)) => Error::NomFailure(remaining.to_string(), kind),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, PartialEq)]
pub enum SelectorType {
    SelectorList(Vec<(String, Option<String>)>),
    WildcardSelector,
    NoSelector,
}
#[derive(Debug, PartialEq)]
pub struct Import {
    pub line_number: u32,
    pub prefix_section: String,
    pub suffix: SelectorType,
}

#[derive(Debug, PartialEq)]
pub struct ParsedFile {
    pub package_name: Option<String>,
    pub imports: Vec<Import>,
}

pub mod java;
pub mod scala;
