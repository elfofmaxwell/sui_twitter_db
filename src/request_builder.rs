use std::{collections::HashMap, fmt::format};
use std::error::Error;
use reqwest::{blocking::{Client, Response}};


use crate::query_result::TweetInfo;
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
    user_id: String, 
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


pub struct TweetFetcher {
    user_id: String, 
    since_tweet_id: Option<String>, 
}

impl TweetFetcher {
    pub fn new(user_id: &str) -> TweetFetcher {
        TweetFetcher { 
            user_id: user_id.to_string(), 
            since_tweet_id: None
        }
    }

    pub fn fetch(&self, conf: &configuration::Config) -> Result<TweetInfo, Box<dyn Error>> {
        let client = Client::builder().build().expect("error in client builder");
        let query_url = format!("https://api.twitter.com/2/users/{}/tweets", &self.user_id);
        let mut request = client.get(&query_url).query(&[
            ("expansions".to_string(), "referenced_tweets.id.author_id".to_string()), 
            ("max_results".to_string(), "100".to_string()), 
            ("tweet.fields".to_string(), "referenced_tweets,entities".to_string()),
            ("user.fields".to_string(), "id,name,username".to_string())
        ]).header("Authorization", format!("Bearer {}", &conf.bearer_token));

        match &self.since_tweet_id {
            Some(since_twitter_id) => {
                request = request.query(&[
                    ("since_id", since_twitter_id)
                ]);
            }
            None => ()
        }

        let response: TweetInfo = request.send()?.json()?;
        Ok(response)
    }
}