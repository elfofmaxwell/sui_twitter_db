use std::{collections::HashMap};
use std::error::Error;
use reqwest::{blocking::{Client}};
use serde_json::Value;


use crate::errors::{InvalidTweetField, InvalidUserField};
use crate::query_result::{self, UserDetail};
use crate::query_result::{FetchedTweet, BasicUserDetail, TweetType, BasicTweet};
use crate::{configuration};

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

    pub fn fetch(&self, conf: &configuration::Config) -> Result<UserDetail, Box<dyn Error>> {
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
        let response = request.send()?.text()?;
        let raw_user: Value = serde_json::from_str(&response)?;
        let mut user_detail = UserDetail {
            id: String::new(), 
            username: String::new(), 
            name: String::new(), 
            location: String::new(), 
            description: String::new(), 
        }; 
        if let Value::Array(user_list) = &raw_user["data"] {
            let user_entity = &user_list[0];
            user_detail.id = match &user_entity["id"] {
                Value::String(id) => id.clone(), 
                _ => {return Err(Box::new(InvalidUserField::new("id")));}
            }; 
            user_detail.username = match &user_entity["username"] {
                Value::String(username) => username.clone(), 
                _ => {return Err(Box::new(InvalidUserField::new("username")));}
            }; 
            user_detail.name = match &user_entity["name"] {
                Value::String(name) => name.clone(), 
                _ => {return Err(Box::new(InvalidUserField::new("name")));}
            }; 
            user_detail.location = match &user_entity["location"] {
                Value::String(location) => location.clone(), 
                _ => String::new()
            }; 
            user_detail.description = match &user_entity["description"] {
                Value::String(description) => description.clone(), 
                _ => String::new()
            }; 

        }
        Ok(user_detail)
    }

    pub fn fetch_basic(&self, conf: &configuration::Config) -> Result<BasicUserDetail, Box<dyn Error>> {
        let full_user_info = self.fetch(conf)?;
        Ok(BasicUserDetail {
                id: full_user_info.id.clone(), 
                name: full_user_info.name.clone(), 
                username: full_user_info.username.clone()
            })
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
            since_tweet_id: Some("1608055824878014464".to_string())
        }
    }

    pub fn fetch(&self, conf: &configuration::Config) -> Result<Vec<FetchedTweet>, Box<dyn Error>> {
        let client = Client::builder().build().expect("error in client builder");
        let query_url = format!("https://api.twitter.com/2/users/{}/tweets", &self.user_id);
        let mut request = client.get(&query_url).query(&[
            ("expansions".to_string(), "referenced_tweets.id.author_id".to_string()), 
            ("max_results".to_string(), "100".to_string()), 
            ("tweet.fields".to_string(), "referenced_tweets,entities,created_at".to_string()),
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

    
        let mut fetched_list: Vec<FetchedTweet> = Vec::new();
        let mut related_users: Vec<BasicUserDetail> = Vec::new(); 
        let mut related_tweets: Vec<BasicTweet> = Vec::new();
        let mut page_token: Option<String> = None;
        
        loop {
            let mut request_cloned = request.try_clone().expect("the request should be cloned");
            if let Some(page_token) = &page_token {
                request_cloned = request_cloned.query(&[("pagination_token".to_string(), page_token.clone())]);
            }
            let response = request_cloned.send()?.text()?;
            let response_parsed: serde_json::Value = serde_json::from_str(&response)?;

            let related_user_raw = &response_parsed["includes"]["users"];
            if let Value::Array(related_user_list) = related_user_raw {
                for user_entity_raw in related_user_list {
                    let username  = match &user_entity_raw["username"] {
                        Value::String(username) => username.to_owned(), 
                        _ => { return Err(Box::new(InvalidTweetField::new("includes.users.username"))); }
                    };
                    let user_id = match &user_entity_raw["id"] {
                        Value::String(id) => id.to_owned(), 
                        _ => { return Err(Box::new(InvalidTweetField::new("includes.users.id"))); }
                    };
                    let name = match &user_entity_raw["name"] {
                        Value::String(name) => name.to_owned(), 
                        _ => { return Err(Box::new(InvalidTweetField::new("includes.users.name"))); }
                    };

                    related_users.push(BasicUserDetail {
                        id: user_id, 
                        username: username, 
                        name: name
                    });
                }
            }

            let related_tweet_raw = &response_parsed["includes"]["tweets"];
            if let Value::Array(related_tweet_list) = related_tweet_raw {
                for tweet_raw in related_tweet_list {
                    let text = match &tweet_raw["text"] {
                        Value::String(text) => text.to_owned(), 
                        _ => { return Err(Box::new(InvalidTweetField::new("includes.tweets.text"))); }
                    };
                    let author_id = match &tweet_raw["author_id"] {
                        Value::String(author_id) => author_id.to_owned(), 
                        _ => { return Err(Box::new(InvalidTweetField::new("includes.tweets.author_id"))); }
                    };
                    let tweet_id = match &tweet_raw["id"] {
                        Value::String(tweet_id) => tweet_id.to_owned(), 
                        _ => { return Err(Box::new(InvalidTweetField::new("includes.tweets.id"))); }
                    }; 

                    related_tweets.push(BasicTweet {
                        author_id: author_id, 
                        id: tweet_id, 
                        text: text
                    });
                }
            }

            let data_list = &response_parsed["data"];
            match data_list {
                Value::Array(tweet_list) => {
                    for tweet_item_raw in tweet_list {
                        let mut tweet_item = FetchedTweet::new();

                        if let Value::String(tweet_id) = &tweet_item_raw["id"] {
                            tweet_item.id = tweet_id.to_owned();
                        } else {
                            return Err(Box::new(InvalidTweetField::new("id")));
                        }

                        if let Value::String(tweet_text) = &tweet_item_raw["text"] {
                            tweet_item.text = tweet_text.to_owned();
                        } else {
                            return Err(Box::new(InvalidTweetField::new("text")));
                        }

                        if let Value::String(created_at) = &tweet_item_raw["created_at"] {
                            tweet_item.created_at = created_at.to_owned();
                        } else {
                            return Err(Box::new(InvalidTweetField::new("created_at")));
                        }

                        if let Value::String(author_id) = &tweet_item_raw["author_id"] {
                            tweet_item.author_id = author_id.to_owned();
                        } else {
                            return Err(Box::new(InvalidTweetField::new("created_at")));
                        }

                        if let Value::String(ref_type) = &tweet_item_raw["referenced_tweets"][0]["type"] {
                            let related_tweet_id = match &tweet_item_raw["referenced_tweets"][0]["id"] {
                                Value::String(id) => id.to_owned(), 
                                _ => {
                                    return Err(Box::new(InvalidTweetField::new("referenced_tweets.id")));
                                }
                            };
                            
                            let related_tweed_detail = query_result::find_by_id(&related_tweet_id, &related_tweets).cloned().unwrap_or(BasicTweet { text: "".to_string(), id: related_tweet_id.clone(), author_id: "".to_string() });
                            let related_user_detail = query_result::find_by_id(&related_tweed_detail.author_id, &related_users).cloned().unwrap_or(BasicUserDetail { id: "".to_string(), username: "".to_string(), name: "".to_string() });

                            match ref_type.as_str() {
                                "replied_to" => {
                                    tweet_item.tweet_type = TweetType::Reply { 
                                        tweet: related_tweed_detail, 
                                        author: related_user_detail
                                    };
                                }
                                "quoted" => {
                                    tweet_item.tweet_type = TweetType::Retweet { 
                                        tweet: related_tweed_detail, 
                                        author: related_user_detail
                                    };
                                }
                                "retweeted" => {
                                    tweet_item.tweet_type = TweetType::Retweet { 
                                        tweet: related_tweed_detail, 
                                        author: related_user_detail
                                    };
                                }
                                _ => { return Err(Box::new(InvalidTweetField::new("referenced_tweets.type"))); }
                            }
                        }

                        if let Value::Array(hashtag_list) = &tweet_item_raw["entities"]["hashtags"] {
                            for hashtag_item in hashtag_list {
                                if let Value::String(hashtag) = &hashtag_item["tag"] {
                                    if let None = tweet_item.hashtags {
                                        tweet_item.hashtags = Some(Vec::new());
                                    }

                                    tweet_item.hashtags.as_mut().unwrap().push(hashtag.to_owned());
                                }
                            }
                        }

                        if let Value::Array(metion_list) = &tweet_item_raw["entities"]["mentions"] {
                            for mention_entity in metion_list {
                                if let Value::String(mentioned_username) = &mention_entity["username"] {
                                    let mentioned_id = match &mention_entity["id"] {
                                        Value::String(id) => id.to_owned(), 
                                        _ => {
                                            return Err(Box::new(InvalidTweetField::new("entities.mentions.id")));
                                        }
                                    };

                                    if let None = tweet_item.mentions {
                                        tweet_item.mentions = Some(Vec::new());
                                    }

                                    let basic_user_detail = query_result::find_by_id(&mentioned_id, &related_users).cloned().unwrap_or(BasicUserDetail {
                                        id: mentioned_id.clone(), 
                                        username: mentioned_username.clone(), 
                                        name: String::new()
                                    });

                                    tweet_item.mentions.as_mut().unwrap().push(BasicUserDetail {
                                        id: mentioned_id, 
                                        username: mentioned_username.to_owned(), 
                                        name: basic_user_detail.name
                                    });
                                }
                            }
                        }
                        fetched_list.push(tweet_item);
                    }
                }
                _ => {
                    if let Value::Number(n_result) = &response_parsed["meta"]["result_count"] {
                        if n_result.as_i64().expect("should return result num") == 0 {
                            break;
                        }
                    } else {
                        return Err(Box::new(InvalidTweetField::new("data")));
                    }
                }
            }

            page_token = match &response_parsed["meta"]["next_token"] {
                Value::String(token) => Some(token.clone()), 
                _ => { break; }
            };
        }
        
        Ok(fetched_list)
    }
}

