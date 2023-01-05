use rusqlite::Connection;

pub fn init_db(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        "BEGIN;
        DROP TABLE IF EXISTS user_profile;
        DROP TABLE IF EXISTS user_dict;
        DROP TABLE IF EXISTS tweet_dict;
        DROP TABLE IF EXISTS user_tweet;
        DROP TABLE IF EXISTS user_liked;
        DROP TABLE IF EXISTS user_following;
        DROP TABLE IF EXISTS user_unfollowed;
        DROP TABLE IF EXISTS hashtag_dict;
        DROP TABLE IF EXISTS mention_dict;
        
        CREATE TABLE user_profile (
            id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, 
            time TEXT, 
            user_id TEXT NOT NULL, 
            username TEXT NOT NULL, 
            name TEXT NOT NULL, 
            location TEXT, 
            description TEXT
        );

        CREATE TABLE user_tweet (
            id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, 
            tweet_id TEXT NOT NULL UNIQUE, 
            tweet_text TEXT NOT NULL,
            time TEXT NOT NULL,
            author_id TEXT NOT NULL, 
            tweet_type TEXT NOT NULL, 
            ref_tweet_id TEXT
        );

        CREATE TABLE user_liked (
            id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, 
            time TEXT, 
            user_id TEXT NOT NULL,
            author_id TEXT NOT NULL,
            ref_tweet_id TEXT NOT NULL
        ); 

        CREATE TABLE user_following (
            id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, 
            time TEXT, 
            user_id TEXT NOT NULL, 
            following_user_id TEXT NOT NULL, 
            action TEXT NOT NULL
        ); 

        CREATE TABLE user_current_following (
            id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, 
            time TEXT, 
            user_id TEXT NOT NULL, 
            following_user_id TEXT NOT NULL, 
            action TEXT NOT NULL
        );

        CREATE TABLE hashtag_dict (
            id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, 
            hashtag TEXT NOT NULL, 
            tweet_id TEXT NOT NULL
        ); 

        CREATE TABLE mention_dict (
            id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, 
            ref_user_id TEXT NOT NULL, 
            tweet_id TEXT NOT NULL
        );

        CREATE TABLE user_dict (
            id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, 
            user_id TEXT NOT NULL UNIQUE, 
            username TEXT NOT NULL, 
            name TEXT NOT NULL
        ); 

        CREATE TABLE tweet_dict (
            id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, 
            tweet_id TEXT NOT NULL UNIQUE, 
            author_id TEXT NOT NULL, 
            text TEXT NOT NULL
        );
        
        COMMIT;"
    )
}

#[cfg(test)]
mod test {
    use crate::{query_result::{FetchedUser, UserDetail}, configuration::TaskType};

    use super::*;

    #[test]
    fn test_init_db() {
        let conn = Connection::open("test_db.db").expect("Should open database");
        init_db(&conn).unwrap();

        let test_fetched_profile = FetchedUser{
            recorded_time: None, 
            user: UserDetail {
                id: "123456".to_string(), 
                username: "abcdefg".to_string(), 
                name: "zyxwvu".to_string(), 
                description: None, 
                location: None,
            }, 
        };

        test_fetched_profile.write_to_db(&conn, &TaskType::Initializing).unwrap();

        let test_fetched_profile_2 = FetchedUser {
            recorded_time: Some("1970-01-01T00:00.000Z".to_string()),
            user: UserDetail {
                location: Some("Tokyo".to_string()), 
                ..test_fetched_profile.user
            }
        };

        test_fetched_profile_2.write_to_db(&conn, &TaskType::Initializing).unwrap();

        test_fetched_profile_2.write_to_db(&conn, &TaskType::Monitoring).unwrap();
        
    }
}