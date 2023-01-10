use std::collections::HashSet;
use std::error::Error;
use std::{thread, time};
use reqwest::{blocking::{Client}};
use rusqlite::Connection;
use serde_json::Value;


use crate::configuration::TaskType;
use crate::errors::*;
use crate::query_result::{self, UserDetail, LikedTweet, FollowingUser, FetchedUser, FollowingAction};
use crate::query_result::{FetchedTweet, BasicUserDetail, TweetType, BasicTweet};
use crate::{configuration};

pub enum RequestMethod {
    Get, 
    Post,
}

pub struct UserInfoFetcher {
    username: String, 
}

impl UserInfoFetcher {
    pub fn new(username: &str) -> UserInfoFetcher {
        UserInfoFetcher { username: username.to_string() }
    }

    pub fn fetch(&self, conf: &configuration::Config) -> Result<FetchedUser, Box<dyn Error>> {
        let client = Client::builder().build().expect("error in client builder");
        let request = 
            client
            .get("https://api.twitter.com/2/users/by")
            .query(&[
                ("usernames".to_string(), self.username.clone()), 
                ("user.fields".to_string(), "description,location".to_string())]
            ).header(
                "Authorization", 
                format!("Bearer {}", &conf.bearer_token)
            );
        let response = request.send()?.text()?;
        log::debug!("User detail fetched!");
        let raw_user: Value = serde_json::from_str(&response)?;
        let mut user_detail = UserDetail {
            id: String::new(), 
            username: String::new(), 
            name: String::new(), 
            location: None, 
            description: None, 
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
                Value::String(location) => Some(location.clone()), 
                _ => None
            }; 
            user_detail.description = match &user_entity["description"] {
                Value::String(description) => Some(description.clone()), 
                _ => None
            }; 

        }

        let mut fetched_user = FetchedUser::record(&conf.task_type);
        fetched_user.user = user_detail;
        Ok(fetched_user)
    }

    pub fn fetch_basic(&self, conf: &configuration::Config) -> Result<BasicUserDetail, Box<dyn Error>> {
        let full_user_info = self.fetch(conf)?;
        Ok(BasicUserDetail {
                id: full_user_info.user.id.clone(), 
                name: full_user_info.user.name.clone(), 
                username: full_user_info.user.username.clone()
            })
    }
}


pub struct TweetFetcher {
    user_id: String, 
    since_tweet_id: Option<String>, 
}

impl TweetFetcher {
    pub fn new(user_id: &str, since_tweet_id: Option<&str>) -> TweetFetcher {
        TweetFetcher { 
            user_id: user_id.to_string(), 
            since_tweet_id: match since_tweet_id {
                Some(tweet_id) => Some(tweet_id.to_string()), 
                None => None
            }
        }
    }

    pub fn fetch(&self, conf: &configuration::Config) -> Result<(
        Vec<FetchedTweet>, 
        Vec<BasicTweet>, 
        Vec<BasicUserDetail>
    ), Box<dyn Error>> {
        let client = Client::builder().build().expect("error in client builder");
        let query_url = format!("https://api.twitter.com/2/users/{}/tweets", &self.user_id);
        let mut request = client.get(&query_url).query(&[
            ("expansions".to_string(), "referenced_tweets.id.author_id".to_string()), 
            ("max_results".to_string(), match conf.task_type {
                TaskType::Initializing => "100".to_string(), 
                TaskType::Monitoring => "5".to_string()
            }), 
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
            if let Some(next_token) = &page_token {
                request_cloned = request_cloned.query(&[("pagination_token".to_string(), next_token.clone())]);
            }
            let response = request_cloned.send()?.text()?;
            log::debug!("Tweet fetched!");
            let response_parsed: serde_json::Value = serde_json::from_str(&response)?;

            let data_list = &response_parsed["data"];
            match data_list {
                Value::Array(tweet_list) => {
                    let mut related_user_in_page = collect_include_users(&response_parsed["includes"]["users"])?;
                    related_users.append(&mut related_user_in_page);
        
                    let related_tweet_raw = &response_parsed["includes"]["tweets"];
                    if let Value::Array(related_tweet_list) = related_tweet_raw {
                        for tweet_raw in related_tweet_list {
                            let related_tweet_item = parse_related_tweet(tweet_raw)?;
                            related_tweets.push(related_tweet_item);
                        }
                    }

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
                            
                            let related_tweed_detail = query_result::find_by_id(
                                &related_tweet_id, 
                                &related_tweets
                            ).cloned().unwrap_or(
                                    BasicTweet { 
                                        text: String::from("Unavailable tweet"), 
                                        id: related_tweet_id.clone(), 
                                        author_id: "".to_string(), 
                                        hashtags: None
                                    }
                                );
                                
                            let related_user_detail = query_result::find_by_id(
                                &related_tweed_detail.author_id, 
                                &related_users).cloned().unwrap_or(
                                    BasicUserDetail { 
                                        id: (&related_tweed_detail.author_id).to_string(), 
                                        username: "".to_string(), 
                                        name: "".to_string() 
                                    }
                                );

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

                                    let basic_user_detail = query_result::find_by_id(
                                        &mentioned_id, 
                                        &related_users
                                    ).cloned().unwrap_or(BasicUserDetail {
                                        id: mentioned_id.clone(), 
                                        username: mentioned_username.clone(), 
                                        name: String::from("Unavailable account")
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
                        log::error!("Get response: {}", &response);
                        return Err(Box::new(InvalidTweetField::new("data")));
                    }
                }
            }

            page_token = match &response_parsed["meta"]["next_token"] {
                Value::String(token) => Some(token.clone()), 
                _ => { break; }
            };
        }
        
        fetched_list.reverse();
        Ok((fetched_list, related_tweets, related_users))
    }


}


pub struct LikeFetcher {
    user_id: String, 
    latest_recorded_id: Option<String>
}

impl LikeFetcher {
    pub fn new(user_id: &str, latest_recorded_id: Option<&str>) -> LikeFetcher {
        LikeFetcher { 
            user_id: user_id.to_string(), 
            latest_recorded_id: match latest_recorded_id {
                Some(latest_recorded_id) => Some(latest_recorded_id.to_string()), 
                None => None 
            }
        }
    }

    pub fn fetch(&self, conf:&configuration::Config) -> Result<(Vec<LikedTweet>, Vec<BasicTweet>, Vec<BasicUserDetail>), Box<dyn Error>> {
        let mut latest_recorded_id: Option<String> = None;
        if let TaskType::Monitoring = conf.task_type {
            latest_recorded_id = self.latest_recorded_id.clone();
        }

        let client = Client::builder().build().expect("error in client builder");
        let query_url = format!("https://api.twitter.com/2/users/{}/liked_tweets", &self.user_id);
        let request = client.get(&query_url).query(&[
            ("expansions".to_string(), "author_id".to_string()), 
            ("max_results".to_string(), match conf.task_type {
                TaskType::Initializing => "100".to_string(), 
                TaskType::Monitoring => "10".to_string()
            }), 
            ("tweet.fields".to_string(), "id,text,entities".to_string()),
            ("user.fields".to_string(), "id,name,username".to_string())
        ]).header("Authorization", format!("Bearer {}", &conf.bearer_token));

        let mut fetched_list: Vec<LikedTweet> = Vec::new();
        let mut related_users: Vec<BasicUserDetail> = Vec::new(); 
        let mut related_tweets: Vec<BasicTweet> = Vec::new();
        let mut page_token: Option<String> = None;
        
        'over_pages: loop {
            let mut request_cloned = request.try_clone().expect("Should be able to clone request");
            if let Some(next_token) = &page_token {
                request_cloned = request_cloned.query(&[("pagination_token".to_string(), next_token.clone())]);
            }
            let response = request_cloned.send()?.text()?;
            log::debug!("Liked fetched!");
            let response_parsed: serde_json::Value = serde_json::from_str(&response)?;
            
            let data_list = &response_parsed["data"];
            
            match data_list {
                Value::Array(liked_list) => {
                    let mut related_user_in_page = collect_include_users(&response_parsed["includes"]["users"])?;
                    related_users.append(&mut related_user_in_page);
                    for liked_tweet_raw in liked_list {
                        let mut liked_tweet_item = LikedTweet::record(&conf.task_type, &self.user_id);
                        let related_tweet_item = parse_related_tweet(liked_tweet_raw)?;
                        if let Some(latest_id) = &latest_recorded_id {
                            if latest_id.as_str() == related_tweet_item.id.as_str() {
                                break 'over_pages;
                            }
                        }
                        let basic_user_info = query_result::find_by_id(&related_tweet_item.author_id, &related_users).cloned().unwrap_or(BasicUserDetail {
                            id: related_tweet_item.author_id.clone(), 
                            username: String::from("unavailable_account"), 
                            name: String::from("Unavailable account")
                        });
    
                        liked_tweet_item.tweet = related_tweet_item.clone();
                        liked_tweet_item.author = basic_user_info.clone();
    
                        related_tweets.push(related_tweet_item); 
                        fetched_list.push(liked_tweet_item);
                    }
                }
                _ => {
                    if let Value::Number(n_result) = &response_parsed["meta"]["result_count"] {
                        if n_result.as_i64().expect("should return result num") == 0 {
                            break 'over_pages;
                        }
                    } else {
                        log::error!("Get response: {}", &response);
                        return Err(Box::new(InvalidTweetField::new("data")));
                    }
                }
            }

            page_token = match &response_parsed["meta"]["next_token"] {
                Value::String(token) => {
                    thread::sleep(time::Duration::from_secs(13));
                    Some(token.clone())
                }, 
                _ => { break 'over_pages; }
            };

        }

        fetched_list.reverse();
        Ok((fetched_list, related_tweets, related_users))
    }
}


pub struct FollowingFetcher{
    user_id: String, 
    following_ids: Option<Vec<String>>
}

impl FollowingFetcher {
    pub fn new(user_id: &str, following_ids: Option<Vec<String>>) -> FollowingFetcher{
        FollowingFetcher {
            user_id: user_id.to_string(), 
            following_ids: following_ids,
        }
    }

    pub fn fetch(&self, conf: &configuration::Config, conn: &Connection) -> Result<(Vec<FollowingUser>, Vec<BasicUserDetail>), Box<dyn Error>> {
        let mut latest_records= None;
        if let TaskType::Monitoring = conf.task_type {
            latest_records = (&self.following_ids).as_ref();
        }
        let mut latest_record_id: Option<&str> = None;
        if let Some(latest_record_list) = latest_records {
            let old_following_num = latest_record_list.len();
            if old_following_num > 0 {
                latest_record_id = Some(&latest_record_list[0].as_str());
            }
        }
        

        let client = Client::builder().build().expect("error in client builder");
        let query_url = format!("https://api.twitter.com/2/users/{}/following", &self.user_id);
        let request = client.get(&query_url).query(&[
            ("max_results".to_string(), match conf.task_type {
                TaskType::Initializing => "1000".to_string(), 
                TaskType::Monitoring => "2".to_string()
            }), 
            ("user.fields".to_string(), "id,name,username".to_string())
        ]).header("Authorization", format!("Bearer {}", &conf.bearer_token));

        let mut fetched_list: Vec<FollowingUser> = Vec::new();
        let mut related_users: Vec<BasicUserDetail> = Vec::new(); 
        let mut current_following_ids: Vec<String> = Vec::new();
        let mut page_token: Option<String> = None;
        
        let mut existing_following = false;
        let mut req_num = 0;
        'over_pages: loop {
            let mut request_cloned = request.try_clone().expect("Should be able to clone request");
            if let Some(next_token) = &page_token {
                request_cloned = request_cloned.query(&[("pagination_token".to_string(), next_token.clone())]);
            }
            let response = request_cloned.send()?.text()?;
            log::debug!("Following fetched!");
            let response_parsed: serde_json::Value = serde_json::from_str(&response)?;
            
            let data_list = &response_parsed["data"];

            match data_list {
                Value::Array(following_list) => {
                    for following_user_raw in following_list {
                        let mut following_entity = FollowingUser::record(&conf.task_type, &self.user_id);

                        let following_id = match &following_user_raw["id"] {
                            Value::String(id) => id.to_string(), 
                            _ => { return Err(Box::new(InvalidUserField::new("id"))); }
                        };
                        current_following_ids.push(following_id.clone());

                        if !existing_following  {
                            if let Some(latest_id) = latest_record_id {
                                if latest_id == following_id.as_str() {
                                    existing_following = true;
                                }
                            }
                        }
                    

                        if !existing_following {
                            let following_username = match &following_user_raw["username"] {
                                Value::String(username) => username.to_string(), 
                                _ => { return Err(Box::new(InvalidUserField::new("username"))); }
                            };
    
                            let following_name = match &following_user_raw["name"] {
                                Value::String(name) => name.to_string(), 
                                _ => { return Err(Box::new(InvalidUserField::new("name"))); }
                            };
    
                            let following_user_detail = BasicUserDetail {
                                id: following_id, 
                                username: following_username, 
                                name: following_name,
                            };
    
                            following_entity.followed_user = following_user_detail.clone();
    
                            fetched_list.push(following_entity);
                            related_users.push(following_user_detail);
                        }
                    }
                }
                _ => {
                    if let Value::Number(n_result) = &response_parsed["meta"]["result_count"] {
                        if n_result.as_i64().expect("should return result num") == 0 {
                            break 'over_pages;
                        }
                    } else {
                        log::error!("Get response: {}", &response);
                        return Err(Box::new(InvalidUserField::new("data")));
                    }
                }
            }

            page_token = match &response_parsed["meta"]["next_token"] {
                Value::String(token) => {
                    req_num += 1;
                    if req_num % 2 == 0 {
                        thread::sleep(time::Duration::from_secs(120));
                    }
                    Some(token.clone())
                }, 
                _ => { break 'over_pages; }
            };
        }

        let current_following_set: HashSet<&String> = current_following_ids.iter().collect();
        let mut prev_following_set: HashSet<&String> = HashSet::new();
        if let Some(prev_following) = latest_records {
            prev_following_set = prev_following.into_iter().collect();
        }
        for unfollowed_id in prev_following_set.difference(&current_following_set) {
            let mut unfollowed_entity = FollowingUser::record(&conf.task_type, &self.user_id);
            unfollowed_entity.action = FollowingAction::Unfollow;
            unfollowed_entity.followed_user = BasicUserDetail::get_record(conn, unfollowed_id)?;
            fetched_list.push(unfollowed_entity);
        }


        fetched_list.reverse();
        Ok((fetched_list, related_users))
    }
}


fn collect_include_users(include_users_raw: &Value) -> Result<Vec<BasicUserDetail>, Box<dyn Error>> {
    let mut related_users: Vec<BasicUserDetail> = Vec::new();
    if let Value::Array(related_user_list) = include_users_raw {
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
    } else {
        return Err(Box::new(InvalidUserList::new()));
    }

    Ok(related_users)
}


fn parse_related_tweet(single_tweet_raw: &Value) -> Result<BasicTweet, Box<dyn Error>> {
    let mut related_tweet_item = BasicTweet {
        author_id: String::new(), 
        id: String::new(), 
        text: String::new(), 
        hashtags: None
    };
    match &single_tweet_raw["text"] {
        Value::String(text) => { related_tweet_item.text = text.to_owned(); }
        _ => { return Err(Box::new(InvalidTweetField::new("includes.tweets.text"))); }
    };
    match &single_tweet_raw["author_id"] {
        Value::String(author_id) => { related_tweet_item.author_id = author_id.to_owned(); }
        _ => { return Err(Box::new(InvalidTweetField::new("includes.tweets.author_id"))); }
    };
    match &single_tweet_raw["id"] {
        Value::String(tweet_id) => { related_tweet_item.id = tweet_id.to_owned(); }
        _ => { return Err(Box::new(InvalidTweetField::new("includes.tweets.id"))); }
    }; 
    if let Value::Array(hashtag_list) = &single_tweet_raw["entities"]["hashtags"] {
        for hashtag_item in hashtag_list {
            if let Value::String(hashtag) = &hashtag_item["tag"] {
                if let None = related_tweet_item.hashtags {
                    related_tweet_item.hashtags = Some(Vec::new());
                }

                related_tweet_item.hashtags.as_mut().unwrap().push(hashtag.to_owned());
            }
        }
    }
    Ok(related_tweet_item)
}