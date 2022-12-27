use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct UserDetail {
    pub id: String, 
    pub name: String, 
    pub username: String, 
    pub location: String, 
    pub description: String
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UserInfo {
    pub data: Vec<UserDetail>,
}