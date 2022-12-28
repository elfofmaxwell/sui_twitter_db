use std::{error::Error, fmt};
#[derive(Debug)]
pub struct InvalidTweetField {
    field: String
}

impl Error for InvalidTweetField {}

impl fmt::Display for InvalidTweetField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Tweet Object: Required field {} is invalid!", &self.field)
    }
}

impl InvalidTweetField {
    pub fn new(field: &str) -> InvalidTweetField {
        InvalidTweetField {
            field: field.to_string()
        }
    }
}