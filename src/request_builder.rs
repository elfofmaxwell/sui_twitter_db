use std::collections::HashMap;
use std::error::Error;
use reqwest::{blocking::{Client, Response}};


use crate::{configuration, query_result::UserInfo};

pub trait RequestParams {
    fn to_hashmap(&self) -> HashMap<String, String>;
    fn get_base_url(&self) -> String;
    fn get_method(&self) -> RequestMethod;
}

pub enum RequestMethod {
    Get, 
    Post,
}

pub struct UserInfoFetcher {
    pub user_id: String, 
}

impl RequestParams for UserInfoFetcher {
    fn get_base_url(&self) -> String {
        "https://api.twitter.com/2/users/by".to_string()
    }

    fn get_method(&self) -> RequestMethod {
        RequestMethod::Get
    }

    fn to_hashmap(&self) -> HashMap<String, String> {
        HashMap::from([
            ("user.fields".to_string(), "description,location".to_string())
        ])
    }
}

impl UserInfoFetcher {
    pub fn new(user_id: &str) -> UserInfoFetcher {
        UserInfoFetcher { user_id: user_id.to_string() }
    }

    pub fn fetch(&self, conf: &configuration::Config) -> Result<UserInfo, Box<dyn Error>> {
        let client = Client::builder().build().expect("error in client builder");
        let request = 
            client
            .get(&self.get_base_url())
            .query(&[
                ("usernames".to_string(), self.user_id.clone()), 
                ("user.fields".to_string(), "description,location".to_string())]
            ).header(
                "Authorization", 
                format!("Bearer {}", &conf.bearer_token)
            );
        let response: UserInfo = request.send()?.json()?;
        Ok(response)
    }
}