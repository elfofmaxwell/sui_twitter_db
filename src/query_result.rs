use std::error::Error;

use rusqlite::{Connection, named_params, OptionalExtension, params};
use serde::{Serialize, Deserialize};
use chrono::prelude::*;

use crate::configuration::TaskType;

pub trait IdMarked {
    fn get_id(&self) -> &String;
}

#[derive(Debug, PartialEq)]
pub struct FetchedUser {
    pub recorded_time: Option<String>, 
    pub user: UserDetail,
}

impl FetchedUser {
    pub fn record(task_type: &TaskType) -> FetchedUser {
        let mut time_string: Option<String> = None;
        if let TaskType::Monitoring = task_type {
            let current_time_vec: Vec<String> = Utc::now().format("%+").to_string().chars().enumerate().map(|(idx, x)| {
                if idx <  23 {
                    x.to_string()
                } else if idx == 23 {
                    'Z'.to_string()
                } else {
                    "".to_string()
                }
            } ).collect();
            time_string = Some(current_time_vec.join(""));
        } 

        FetchedUser { 
            recorded_time: time_string, 
            user: UserDetail { 
                id: String::new(), 
                username: String::new(), 
                name: String::new(), 
                location: None, 
                description: None,
            } 
        }
    }

    pub fn write_to_db(&self, conn: &Connection, task_type: &TaskType) -> Result<(), Box<dyn Error>> {
        let mut stmt = conn.prepare(
            "INSERT INTO user_profile 
            (time, user_id, username, name, location, description)
            VALUES (:time, :user_id, :username, :name, :location, :description)"
        )?;

        if let TaskType::Monitoring = task_type {
            let latest_records = FetchedUser::get_records(conn, &self.user.username, Some(1), 0)?;
            let latest_record = latest_records.get(0).expect("Should have 1 user record");

            println!("latest condition {:#?}", &latest_record);
            println!("new record {:#?}", self);
            if dbg!(latest_record.user == self.user) {
                return Ok(());
            }
        }

        stmt.execute(named_params! {
            ":time": &self.recorded_time,
            ":user_id": &self.user.id, 
            ":name": &self.user.name, 
            ":username": &self.user.username, 
            ":location": &self.user.location, 
            ":description": &self.user.description,
        })?;

        Ok(())
    }

    pub fn get_records(conn: &Connection, username: &str, max_results: Option<u16>, offset: u16) -> Result<Vec<FetchedUser>, rusqlite::Error> {
        let user_constructor = |row: &rusqlite::Row| -> rusqlite::Result<FetchedUser> {
            Ok(FetchedUser {
                recorded_time: row.get(1)?, 
                user: UserDetail { 
                    id: row.get(2)?, 
                    username: row.get(3)?, 
                    name: row.get(4)?, 
                    location: row.get(5)?, 
                    description: row.get(6)? 
                }
            })
        };

        let mut user_vec: Vec<FetchedUser> = Vec::new();
        match max_results {
            Some(max_val) => {
                let mut user_profile_stmt = conn.prepare("SELECT * FROM user_profile WHERE username = ? ORDER BY id DESC LIMIT ? OFFSET ?")?;
                let query_result = user_profile_stmt.query_map(params![username, max_val, offset], user_constructor)?;
                for fetched_user in query_result {
                    user_vec.push(fetched_user?);
                }
            }
            None => {
                let mut user_profile_stmt = conn.prepare("SELECT * FROM user_profile WHERE username = ? ORDER BY id DESC")?;
                let query_result = user_profile_stmt.query_map(params![username], user_constructor)?;
                for fetched_user in query_result {
                    user_vec.push(fetched_user?);
                }
            }
        }
        
        Ok(user_vec)
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct UserDetail {
    pub id: String, 
    pub name: String, 
    pub username: String, 
    pub location: Option<String>, 
    pub description: Option<String>
}

impl IdMarked for UserDetail {
    fn get_id(&self) -> &String {
        &self.id
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BasicUserDetail {
    pub id: String, 
    pub username: String, 
    pub name: String
}

impl IdMarked for BasicUserDetail {
    fn get_id(&self) -> &String {
        &self.id
    }
}

impl BasicUserDetail {
    pub fn write_to_db(&self, conn: &Connection) -> Result<(), Box<dyn Error>> {
        let mut user_dict_stmt = conn.prepare(
            "INSERT INTO user_dict 
            (user_id, username, name) 
            VALUES (:user_id, :username, :name) 
            ON CONFLICT (user_id) DO UPDATE 
            SET username = :new_username, name = :new_name"
        )?;

        user_dict_stmt.execute(
            named_params! {
                ":user_id": &self.id, 
                ":username": &self.username, 
                ":name": &self.name, 
                ":new_username": &self.username, 
                ":new_name": &self.name
            }
        )?;
        
        Ok(())
    }

    pub fn get_record(conn: &Connection, user_id: &str) -> Result<BasicUserDetail, Box<dyn Error>> {
        let user: BasicUserDetail = conn.query_row(
            "SELECT * FROM user_dict WHERE user_id = ?", [user_id, ], |row| {
                Ok(BasicUserDetail {
                    id: row.get(1)?, 
                    username: row.get(2)?, 
                    name: row.get(3)?
                })
            }
        )?;
        Ok(user)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BasicTweet {
    pub text: String, 
    pub id: String, 
    pub author_id: String,
    pub hashtags: Option<Vec<String>>, 
}

impl IdMarked for BasicTweet {
    fn get_id(&self) -> &String {
        &self.id
    }
}

impl BasicTweet {
    pub fn write_to_db(&self, conn: &Connection) -> Result<(), Box<dyn Error>> {
        let check_exist: rusqlite::Result<Option<String>> = conn.query_row(
            "SELECT id FROM tweet_dict WHERE tweet_id = ?", 
            [&self.id], 
            |row| {
                row.get(0)
            }
        ).optional();

        match check_exist? {
            Some(_) => { return Ok(()); }
            None => {
                let mut tweet_dict_stmt = conn.prepare(
                    "INSERT INTO tweet_dict 
                    (tweet_id, author_id, text) 
                    VALUES (:tweet_id, :author_id, :text) 
                    ON CONFLICT (tweet_id) DO UPDATE 
                    SET author_id = :new_author_id, text = :new_text"
                )?;
        
                tweet_dict_stmt.execute(
                    named_params! {
                        ":tweet_id": &self.id, 
                        ":author_id": &self.author_id, 
                        ":text": &self.text, 
                        ":new_author_id": &self.author_id,
                        ":new_text": &self.text
                    }
                )?;

                let mut hashtag_dict_stmt = conn.prepare(
                    "INSERT INTO hashtag_dict 
                    (hashtag, tweet_id) 
                    VALUES (:hashtag, :tweet_id)"
                )?;

                if let Some(hashtag_vec) = &self.hashtags {
                    for hashtag in hashtag_vec {
                        hashtag_dict_stmt.execute(
                            named_params! {
                                ":hashtag": hashtag, 
                                ":tweet_id": &self.id
                            }
                        )?;
                    }
                }

            }
        }

        Ok(())
    }

    pub fn get_record(conn: &Connection, tweet_id: &str) -> Result<BasicTweet, Box<dyn Error>> {
        let mut tweet: BasicTweet = conn.query_row(
            "SELECT * FROM tweet_dict WHERE tweet_id = ?", 
            [tweet_id, ], 
            |row| {
                Ok(BasicTweet {
                    id: row.get(1)?, 
                    author_id: row.get(2)?, 
                    text: row.get(3)?, 
                    hashtags: None
                })
            }
        )?;

        let mut hashtag_stmt = conn.prepare("SELECT * FROM hashtag_dict WHERE tweet_id = :tweet_id")?;
        let hashtag_iter = hashtag_stmt.query_map(named_params! {":tweet_id": &tweet.id}, |row| {
                row.get(1)
        })?;
        let mut hashtag_vec: Vec<String> = Vec::new();
        for hashtag in hashtag_iter {
            hashtag_vec.push(hashtag?);
        }
        if hashtag_vec.len() > 0 {
            tweet.hashtags = Some(hashtag_vec);
        }

        Ok(tweet)
    }
    
}
#[derive(Debug, PartialEq)]
pub enum TweetType {
    Tweet, 
    Reply {
        tweet: BasicTweet, 
        author: BasicUserDetail
    }, 
    Retweet {
       tweet: BasicTweet, 
       author: BasicUserDetail
    }
}

#[derive(Debug, PartialEq)]
pub struct FetchedTweet {
    pub id: String, 
    pub text: String, 
    pub created_at: String,
    pub author_id: String, 
    pub tweet_type: TweetType,
    pub hashtags: Option<Vec<String>>, 
    pub mentions: Option<Vec<BasicUserDetail>>,
}

impl FetchedTweet {
    pub fn new() -> FetchedTweet {
        FetchedTweet{
            id: String::new(), 
            text: String::new(), 
            created_at: String::new(), 
            author_id: String::new(), 
            tweet_type: TweetType::Tweet, 
            hashtags: None, 
            mentions: None,
        }
    }

    pub fn get_records(conn: &Connection, author_id: &str, max_results: Option<u16>, offset: u16) -> Result<Vec<FetchedTweet>, Box<dyn Error>> {

        let query_mapper = |row: &rusqlite::Row| -> rusqlite::Result<(FetchedTweet, String, Option<String>)> {
            let mut latest_tweet = FetchedTweet::new();
                latest_tweet.id = row.get(1)?; 
                latest_tweet.text = row.get(2)?;
                latest_tweet.created_at = row.get(3)?;
                latest_tweet.author_id = row.get(4)?;
                let tweet_type_str: String = row.get(5)?;
                let ref_tweet_id: Option<String> = row.get(6)?;
                Ok((latest_tweet, tweet_type_str, ref_tweet_id))
        };

        let mut user_tweet_query_results: Vec<(FetchedTweet, String, Option<String>)> = Vec::new();
        match max_results {
            Some(max_val) => {
                let mut user_tweet_stmt = conn.prepare("SELECT * FROM user_tweet WHERE author_id = ? ORDER BY id DESC LIMIT ? OFFSET ?")?;
                let query_results = user_tweet_stmt.query_map(params![author_id, max_val, offset], query_mapper)?;
                for query_result in query_results {
                    user_tweet_query_results.push(query_result?);
                }
            }
            None => {
                let mut user_tweet_stmt = conn.prepare("SELECT * FROM user_tweet WHERE author_id = ? ORDER BY id DESC")?;
                let query_results = user_tweet_stmt.query_map(params![author_id], query_mapper)?;
                for query_result in query_results {
                    user_tweet_query_results.push(query_result?);
                }
            }
        }

        let mut fetched_tweet_list: Vec<FetchedTweet> = Vec::new();
        for (mut latest_tweet, tweet_type_str, ref_tweet_id) in user_tweet_query_results.into_iter() {

            let mut hashtag_query = conn.prepare("SELECT * FROM hashtag_dict WHERE tweet_id = ?")?;
            let mut mention_query = conn.prepare("SELECT * FROM mention_dict WHERE tweet_id = ?")?;
            let mut user_query = conn.prepare("SELECT * FROM user_dict WHERE user_id = ?")?;
            let latest_hashtag = hashtag_query.query_map([&latest_tweet.id], |row| {
                row.get(1)
            })?;
            let latest_mention = mention_query.query_map([&latest_tweet.id], |row| {
                row.get(1)
            })?;
    
            let mut hashtag_vec: Vec<String> = Vec::new();
            let mut mention_vec: Vec<String> = Vec::new();
            for hashtag in latest_hashtag {
                hashtag_vec.push(hashtag?);
            }
            if hashtag_vec.len() > 0 {
                latest_tweet.hashtags = Some(hashtag_vec);
            }
    
            for mentioned_id in latest_mention {
                mention_vec.push(mentioned_id?);
            }
            if mention_vec.len() > 0 {
                latest_tweet.mentions = Some(Vec::new());
                for mentioned_id in &mention_vec {
                    let mentioned_user = user_query.query_map([mentioned_id], |row| {
                        Ok(BasicUserDetail {
                            id: row.get(1)?, 
                            username: row.get(2)?, 
                            name: row.get(3)?
                        })
                    })?.next().expect("Should find the user").expect("should be able to parse the user");
                    latest_tweet.mentions.as_mut().unwrap().push(mentioned_user);
                }
            }
    
            if &tweet_type_str == "tweet" {
                latest_tweet.tweet_type = TweetType::Tweet;
            } else if (&tweet_type_str == "retweet") || (&tweet_type_str == "reply") {
                let mut ref_tweet: BasicTweet = conn.query_row("SELECT * FROM tweet_dict WHERE tweet_id = ?", [&ref_tweet_id.expect("should have ref id")], |row| {
                    let ref_tweet = BasicTweet {
                        id: row.get(1)?, 
                        author_id: row.get(2)?,
                        text: row.get(3)?,
                        hashtags: None,
                    };
                    Ok(ref_tweet)
                })?;
    
                let ref_hashtag = hashtag_query.query_map([&ref_tweet.id], |row| {
                    row.get(1)
                })?;
    
                let mut ref_hashtag_vec: Vec<String> = Vec::new();
                for hashtag in ref_hashtag {
                    ref_hashtag_vec.push(hashtag?);
                }
                if ref_hashtag_vec.len() > 0 {
                    ref_tweet.hashtags = Some(ref_hashtag_vec);
                }
                
                let ref_user: BasicUserDetail = user_query.query_map([&ref_tweet.author_id], |row| {
                    Ok(BasicUserDetail {
                        id: row.get(1)?, 
                        username: row.get(2)?, 
                        name: row.get(3)?
                    })
                })?.next().expect("Should find the user").expect("should be able to parse the user");
    
                if &tweet_type_str == "retweet" {
                    latest_tweet.tweet_type = TweetType::Retweet { tweet: ref_tweet, author: ref_user };
                } else {
                    latest_tweet.tweet_type = TweetType::Reply { tweet: ref_tweet, author: ref_user };
                }
            } else {
                panic!("Unacceptable tweet type in table user_tweet!");
            }

            fetched_tweet_list.push(latest_tweet);
        }
        

        Ok(fetched_tweet_list)
    } 

    pub fn newest_id(conn: &Connection, author_id: &str) -> Result<Option<String>, Box<dyn Error>> {
        let newest_id: Option<String> = conn.query_row(
            "SELECT tweet_id FROM user_tweet WHERE author_id = ? ORDER BY id DESC LIMIT 1", 
            [author_id, ], 
            |row| {
                row.get(0)
            }
        ).optional()?;
        Ok(newest_id)
    }

    pub fn write_to_db(&self, conn: &Connection) -> Result<(), Box<dyn Error>> {
        let mut user_tweet_stmt = conn.prepare(
            "INSERT INTO user_tweet
            (tweet_id, tweet_text, time, author_id, tweet_type, ref_tweet_id) 
            VALUES (:tweet_id, :tweet_text, :time, :author_id, :tweet_type, :ref_tweet_id)"
        )?;
        let mut hashtag_dict_stmt = conn.prepare(
            "INSERT INTO hashtag_dict 
            (hashtag, tweet_id) 
            VALUES (:hashtag, :tweet_id)"
        )?;
        let mut mention_dict_stmt = conn.prepare(
            "INSERT INTO mention_dict 
            (ref_user_id, tweet_id) 
            VALUES (:ref_user_id, :tweet_id)"
        )?;

        let (tweet_type, ref_tweet_id) = match &self.tweet_type {
            TweetType::Tweet => ("tweet", None), 
            TweetType::Reply { tweet, author: _ } => ("reply", Some(tweet.id.as_str())),
            TweetType::Retweet { tweet, author: _ } => ("retweet", Some(tweet.id.as_str()))
        };

        user_tweet_stmt.execute(
            named_params! {
                ":tweet_id": &self.id, 
                ":tweet_text": &self.text,
                ":time": &self.created_at, 
                ":author_id": &self.author_id, 
                ":tweet_type": tweet_type, 
                ":ref_tweet_id": ref_tweet_id
            }
        )?;

        if let Some(hashtag_vec) = &self.hashtags {
            for hashtag_str in hashtag_vec {
                hashtag_dict_stmt.execute(
                    named_params! {
                        ":hashtag": hashtag_str, 
                        ":tweet_id": &self.id
                    }
                )?;
            }
        }

        if let Some(mention_vec) = &self.mentions {
            for mentioned_id in mention_vec {
                mention_dict_stmt.execute(
                    named_params! {
                        ":ref_user_id": &mentioned_id.id, 
                        ":tweet_id": &self.id
                    }
                )?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, PartialEq)]
pub struct LikedTweet {
    pub recorded_time: Option<String>, 
    pub user_id: String,
    pub tweet: BasicTweet, 
    pub author: BasicUserDetail
}

impl LikedTweet {
    pub fn record(task_type: &TaskType, user_id: &str) -> LikedTweet {
        let mut time_string: Option<String> = None;
        if let TaskType::Monitoring = task_type {
            let current_time_vec: Vec<String> = Utc::now().format("%+").to_string().chars().enumerate().map(|(idx, x)| {
                if idx <  23 {
                    x.to_string()
                } else if idx == 23 {
                    'Z'.to_string()
                } else {
                    "".to_string()
                }
            } ).collect();
            time_string = Some(current_time_vec.join(""));
        } 

        LikedTweet { 
            recorded_time: time_string, 
            user_id: user_id.to_string(),
            tweet: BasicTweet {
                text: String::new(), 
                id: String::new(), 
                author_id: String::new(),
                hashtags: None
            }, 
            author: BasicUserDetail { 
                id: String::new(), 
                username: String::new(), 
                name: String::new() 
            } 
        }
    }

    pub fn newest_id(conn: &Connection, user_id: &str) -> Result<Option<String>, Box<dyn Error>> {
        let newest_id: Option<String> = conn.query_row(
            "SELECT tweet_id FROM user_liked WHERE user_id = ? ORDER BY id DESC LIMIT 1", 
            [user_id, ], 
            |row| {
                row.get(4)
            }
        ).optional()?;
        Ok(newest_id)
    }

    pub fn get_records(conn: &Connection, user_id: &str, max_results: Option<u16>, offset: u16) -> Result<Vec<LikedTweet>, Box<dyn Error>> {
        let result_mapper = |row: &rusqlite::Row| -> rusqlite::Result<(Option<String>, String, String)> {
            Ok((row.get(1)?, row.get(3)?, row.get(4)?))
        };

        let mut queried_liked_vec: Vec<(Option<String>, String, String)> = Vec::new();
        match max_results {
            Some(max_val) => {
                let mut user_liked_stmt = conn.prepare("SELECT * FROM user_liked WHERE user_id = ? ORDER BY id DESC LIMIT ? OFFSET ?")?;
                let query_results = user_liked_stmt.query_map(
                    params![user_id, max_val, offset], 
                    result_mapper
                )?;
                for query_result in query_results {
                    queried_liked_vec.push(query_result?);
                }
            }
            None => {
                let mut user_liked_stmt = conn.prepare("SELECT * FROM user_liked WHERE user_id = ? ORDER BY id DESC")?;
                let query_results = user_liked_stmt.query_map(
                    params![user_id], 
                    result_mapper
                )?;
                for query_result in query_results {
                    queried_liked_vec.push(query_result?);
                }
            }
        }

        let mut liked_tweet_vec: Vec<LikedTweet> = Vec::new();
        for (recorded_time, author_id, ref_tweet_id) in queried_liked_vec.into_iter() {
            let author_detail: BasicUserDetail = BasicUserDetail::get_record(conn, &author_id)?;
            let tweet_detail: BasicTweet = BasicTweet::get_record(conn, &ref_tweet_id)?;
            liked_tweet_vec.push(
                LikedTweet { 
                    recorded_time: recorded_time, 
                    user_id: user_id.to_string(), 
                    tweet: tweet_detail, 
                    author: author_detail 
                }
            );
        }

        Ok(liked_tweet_vec)
    }

    pub fn write_to_db(&self, conn: &Connection) -> Result<(), Box<dyn Error>> {
        let mut user_liked_stmt = conn.prepare(
            "INSERT INTO user_liked
            (time, user_id, author_id, ref_tweet_id)
            VALUES (:time, :user_id, :author_id, :ref_tweet_id)"
        )?;
        user_liked_stmt.execute(
            named_params! {
                ":time": &self.recorded_time, 
                ":user_id": &self.user_id, 
                ":author_id": &self.author.id, 
                ":ref_tweet_id": &self.tweet.id
            }
        )?;

        Ok(())
    }
}

#[derive(Debug, PartialEq)]
pub enum FollowingAction {
    Follow, 
    Unfollow
}

#[derive(Debug, PartialEq)]
pub struct FollowingUser {
    pub recorded_time: Option<String>, 
    pub user_id: String,
    pub followed_user: BasicUserDetail, 
    pub action: FollowingAction
}

impl FollowingUser {
    pub fn record(task_type: &TaskType, user_id: &str) -> FollowingUser {
        let mut time_string: Option<String> = None;
        if let TaskType::Monitoring = task_type {
            let current_time_vec: Vec<String> = Utc::now().format("%+").to_string().chars().enumerate().map(|(idx, x)| {
                if idx <  23 {
                    x.to_string()
                } else if idx == 23 {
                    'Z'.to_string()
                } else {
                    "".to_string()
                }
            } ).collect();
            time_string = Some(current_time_vec.join(""));
        } 

        FollowingUser { 
            recorded_time: time_string, 
            user_id: user_id.to_string(),
            followed_user: BasicUserDetail { 
                id: String::new(), 
                username: String::new(), 
                name: String::new() 
            }, 
            action: FollowingAction::Follow
        }
    }

    pub fn newest_id(conn: &Connection, user_id: &str) -> Result<Option<String>, Box<dyn Error>> {
        let newest_id: Option<String> = conn.query_row(
            "SELECT tweet_id FROM user_following WHERE user_id = ? and action = 'follow' ORDER BY id DESC LIMIT 1", 
            [user_id, ], 
            |row| {
                row.get(3)
            }
        ).optional()?;
        Ok(newest_id)
    }

    pub fn get_records(conn: &Connection,  user_id: &str, max_results: Option<u16>, offset: u16) -> Result<Vec<FollowingUser>, Box<dyn Error>> {
        let query_map = |row: &rusqlite::Row| -> rusqlite::Result<(Option<String>, String, String)> {
            Ok((row.get(1)?, row.get(3)?, row.get(4)?))
        };

        let mut queried_following_vec: Vec<(Option<String>, String, String)> = Vec::new();
        match max_results {
            Some(max_val) => {
                let mut user_following_stmt = conn.prepare("SELECT * FROM user_following WHERE user_id = ? ORDER BY id DESC LIMIT ? OFFSET ?")?;
                let query_results = user_following_stmt.query_map(params![user_id, max_val, offset], query_map)?;
                for query_result in query_results {
                    queried_following_vec.push(query_result?);
                }
            }
            None => {
                let mut user_following_stmt = conn.prepare("SELECT * FROM user_following WHERE user_id = ? ORDER BY id DESC")?;
                let query_results = user_following_stmt.query_map(params![user_id], query_map)?;
                for query_result in query_results {
                    queried_following_vec.push(query_result?);
                }
            }
        }

        let mut following_vec: Vec<FollowingUser> = Vec::new();
        for (recorded_time, followed_id, action_str) in queried_following_vec.into_iter() {
            let following_action = if action_str.as_str() == "follow" {
                FollowingAction::Follow
            } else if action_str.as_str() == "unfollow" {
                FollowingAction::Unfollow
            } else {
                panic!("Unacceptable following type string");
            };

            let followed_user = BasicUserDetail::get_record(conn, &followed_id)?;
            following_vec.push(
                FollowingUser {
                    recorded_time: recorded_time, 
                    user_id: user_id.to_string(), 
                    followed_user: followed_user, 
                    action: following_action
                }
            );
        }
        Ok(following_vec)
    }

    pub fn write_to_db(&self, conn: &Connection) -> Result<(), Box<dyn Error>> {
        let mut user_following_stmt = conn.prepare(
            "INSERT INTO user_following
            (time, user_id, following_user_id, action)
            VALUES (:time, :user_id, :following_user_id, :action)"
        )?;

        let action_str = match &self.action {
            FollowingAction::Follow => "follow", 
            FollowingAction::Unfollow => "unfollow"
        };

        user_following_stmt.execute(
            named_params! {
                ":time": &self.recorded_time, 
                ":user_id": &self.user_id, 
                ":following_user_id": &self.followed_user.id, 
                ":action": action_str
            }
        )?;
        Ok(())
    }
}

pub fn find_by_id<'a, T: IdMarked>(id: &str, dictionary: &'a Vec<T>) -> Option<&'a T> {
    dictionary.iter().find(|item| item.get_id()==id)
}

#[cfg(test)]
mod tests {
    use crate::db::init_db;

    use super::*;
    #[test]
    fn test_find_by_id() {
        let user_dict = vec![
            BasicUserDetail {
                id: "1234".to_string(), 
                name: "abcd".to_string(), 
                username: "efgh".to_string()
            }, 
            BasicUserDetail {
                id: "0987".to_string(), 
                name: "zyxw".to_string(), 
                username: "vuts".to_string()
            }
        ];

        let target = "0987";
        let found_user = find_by_id(target, &user_dict);
        assert_eq!(&found_user.unwrap().name, "zyxw");
    }

    #[test]
    fn test_db_write_and_get() {
        let conn = Connection::open_in_memory().unwrap();

        init_db(&conn).unwrap();
        let hsmtss_profile_0 = FetchedUser {
            recorded_time: None, 
            user: UserDetail { 
                id: "0".to_string(), 
                name: "Hoshimachi Suisei".to_string(), 
                username: "hoshimatisuisei".to_string(), 
                location: Some("Tokyo".to_string()), 
                description: Some("Inui Toko Daisuki!".to_string())
            }
        };

        let hsmtss_profile_1 = FetchedUser {
            recorded_time: Some("2022-12-31T00:00:00Z".to_string()), 
            user: UserDetail { 
                id: "0".to_string(), 
                name: "Hoshimachi Suisei".to_string(), 
                username: "hoshimatisuisei".to_string(), 
                location: Some("Komoro".to_string()), 
                description: Some("Inui Toko Daisuki!".to_string())
            }
        };

        let inui_toko_profile_0 = BasicUserDetail {
            id: "1".to_string(), 
            username: "inui_toko".to_string(), 
            name: "Inui Toko".to_string()
        };

        let inui_toko_profile_1 = BasicUserDetail {
            id: "1".to_string(), 
            username: "inui_toko".to_string(), 
            name: "Inui Toko!".to_string()
        };

        let hsmt_tweet_0 = FetchedTweet {
            id: "001".to_string(), 
            author_id: "0".to_string(), 
            text: "Toko-chan chuu #inui_toko_daisuki @inui_toko".to_string(), 
            created_at: "2022-01-01T00:00:00Z".to_string(), 
            tweet_type: TweetType::Tweet, 
            mentions: Some(vec![inui_toko_profile_0.clone()]), 
            hashtags: Some(vec!["inui_toko_daisuki".to_string()])
        };

        let tkymtw_profile = BasicUserDetail {
            id: "2".to_string(), 
            username: "tokoyami_towa".to_string(), 
            name: "Tokoyami Towa".to_string()
        };

        let tkymtw_tweet = BasicTweet {
            id: "002".to_string(), 
            author_id: "2".to_string(), 
            text: "Suisei gomi! #hoshimachi".to_string(),
            hashtags: Some(vec!["hoshimachi".to_string()])
        };

        let hsmt_follow_0 = FollowingUser {
            recorded_time: None, 
            user_id: "0".to_string(), 
            followed_user: inui_toko_profile_0.clone(), 
            action: FollowingAction::Follow
        };

        let hsmt_follow_1 = FollowingUser {
            recorded_time: Some("2021-01-01T00:00:00Z".to_string()), 
            user_id: "0".to_string(), 
            followed_user: tkymtw_profile.clone(), 
            action: FollowingAction::Follow
        };

        let hsmt_like_0 = LikedTweet {
            recorded_time: None, 
            user_id: "0".to_string(), 
            tweet: tkymtw_tweet.clone(),
            author: tkymtw_profile.clone()
        };

        let hsmt_tweet_1 = FetchedTweet {
            id: "003".to_string(), 
            author_id: "0".to_string(), 
            text: "a... @inui_toko".to_string(), 
            created_at: "2022-01-01T00:00:00Z".to_string(), 
            tweet_type: TweetType::Reply { tweet: tkymtw_tweet.clone(), author: tkymtw_profile.clone() }, 
            mentions: Some(vec![tkymtw_profile.clone(), inui_toko_profile_1.clone()]), 
            hashtags: None
        };

        hsmtss_profile_0.write_to_db(&conn, &TaskType::Initializing).unwrap();
        let gotten_hsmtss_profile_0 = FetchedUser::get_records(&conn, "hoshimatisuisei", Some(1), 0).unwrap().into_iter().next().unwrap();
        assert_eq!(gotten_hsmtss_profile_0, hsmtss_profile_0);

        inui_toko_profile_0.write_to_db(&conn).unwrap();
        hsmt_tweet_0.write_to_db(&conn).unwrap();
        let gotten_hsmtss_tweet_0 = FetchedTweet::get_records(&conn, "0", Some(1), 0).unwrap().into_iter().next().unwrap();
        assert_eq!(gotten_hsmtss_tweet_0, hsmt_tweet_0);

        tkymtw_profile.write_to_db(&conn).unwrap();
        tkymtw_tweet.write_to_db(&conn).unwrap();
        hsmt_like_0.write_to_db(&conn).unwrap();
        let gotten_hsmtss_like_0 = LikedTweet::get_records(&conn, "0", Some(1), 0).unwrap().into_iter().next().unwrap();
        assert_eq!(gotten_hsmtss_like_0, hsmt_like_0);

        hsmt_follow_0.write_to_db(&conn).unwrap();
        let gotten_hsmtss_follow_0 = FollowingUser::get_records(&conn, "0", Some(1), 0).unwrap().into_iter().next().unwrap();
        assert_eq!(gotten_hsmtss_follow_0, hsmt_follow_0);

        hsmtss_profile_1.write_to_db(&conn, &TaskType::Monitoring).unwrap();
        let gotten_hsmtss_profile_1 = FetchedUser::get_records(&conn, "hoshimatisuisei", Some(1), 0).unwrap().into_iter().next().unwrap();
        assert_eq!(gotten_hsmtss_profile_1, hsmtss_profile_1);

        hsmt_follow_1.write_to_db(&conn).unwrap();
        let gotten_hsmtss_follow_1 = FollowingUser::get_records(&conn, "0", Some(1), 0).unwrap().into_iter().next().unwrap();
        assert_eq!(gotten_hsmtss_follow_1, hsmt_follow_1);

        inui_toko_profile_1.write_to_db(&conn).unwrap();
        hsmt_tweet_1.write_to_db(&conn).unwrap();
        let gotten_hsmtss_tweet_1 = FetchedTweet::get_records(&conn, "0", Some(1), 0).unwrap().into_iter().next().unwrap();
        assert_eq!(gotten_hsmtss_tweet_1, hsmt_tweet_1);


    }
}