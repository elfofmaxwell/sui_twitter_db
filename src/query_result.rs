use std::{collections::HashMap};

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

#[derive(Serialize, Deserialize, Debug)]
pub struct RefUserDetail {
    pub id: String, 
    pub name: String, 
    pub username: String, 
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TweetHashTag {
    start: i8, 
    end: i8, 
    tag: String
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TweetMention {
    start: i8, 
    end: i8, 
    tag: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TweetEntity {
    annotations: Vec<HashMap<String, String>>, 
    cashtags: Vec<HashMap<String, String>>, 
    hashtags: Vec<TweetHashTag>, 
    mentions: Vec<TweetMention>, 
    urls: Vec<HashMap<String, String>>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TweetExpandFields {
    users: Vec<RefUserDetail>, 
    tweets: Vec<TweetDetail>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TweetDetail {
    id: String, 
    edit_history_tweet_ids: Vec<String>, 
    text: String,
    created_at: String, 
    referenced_tweets: Vec<HashMap<String, String>>, 
    entities: TweetEntity
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TweetInfo {
    data: Vec<TweetDetail>, 
    includes: TweetExpandFields,
    meta: HashMap<String, String>
}