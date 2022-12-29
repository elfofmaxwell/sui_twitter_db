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

#[derive(Debug)]
pub struct InvalidUserField {
    field: String
}

impl Error for InvalidUserField {}

impl fmt::Display for InvalidUserField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "User Object: Required field {} is invalid!", &self.field)
    }
}

impl InvalidUserField {
    pub fn new(field: &str) -> InvalidUserField {
        InvalidUserField {
            field: field.to_string()
        }
    }
}

#[derive(Debug)]
pub struct InvalidUserList;

impl Error for InvalidUserList {}

impl fmt::Display for InvalidUserList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "The user list is invalid!")
    }
}

impl InvalidUserList {
    pub fn new() -> InvalidUserList {
        InvalidUserList
    }
}