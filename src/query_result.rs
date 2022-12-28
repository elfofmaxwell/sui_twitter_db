use std::{collections::HashMap};

use serde::{Serialize, Deserialize};

pub trait IdMarked {
    fn get_id(&self) -> &String;
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UserDetail {
    pub id: String, 
    pub name: String, 
    pub username: String, 
    pub location: String, 
    pub description: String
}

impl IdMarked for UserDetail {
    fn get_id(&self) -> &String {
        &self.id
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UserInfo {
    pub data: Vec<UserDetail>,
}

#[derive(Debug)]
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

#[derive(Debug)]
pub struct BasicTweet {
    pub text: String, 
    pub id: String
}

impl IdMarked for BasicTweet {
    fn get_id(&self) -> &String {
        &self.id
    }
}

#[derive(Debug)]
pub enum TweetType {
    tweet, 
    reply {
        tweet: BasicTweet, 
        author: BasicUserDetail
    }, 
    retweet {
       tweet: BasicTweet, 
       author: BasicUserDetail
    }
}

#[derive(Debug)]
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
            tweet_type: TweetType::tweet, 
            hashtags: None, 
            mentions: None,
        }
    }
}

pub fn find_by_id<'a, T: IdMarked>(id: &str, dictionary: &'a Vec<T>) -> Option<&'a T> {
    dictionary.iter().find(|item| item.get_id()==id)
}

#[cfg(test)]
mod tests {
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
}