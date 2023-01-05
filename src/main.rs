use std::{process, thread, time, sync::Arc};
use clap::Parser;
use rusqlite::Connection;
use sui_twitter_db::{configuration::{Config, Args, TaskType}, request_builder::{UserInfoFetcher, TweetFetcher, LikeFetcher, FollowingFetcher}, db, query_result::{FetchedUser, FetchedTweet, LikedTweet, FollowingUser}};

fn main() {
    env_logger::init();


    let args = Args::parse();

    let config = match Config::configure(&args.conf_path, args.verbose, &args.task_type) {
        Ok(config) => config, 
        Err(e) => {
            eprintln!("{e}");
            process::exit(1);
        }
    };

    
    for username in config.monitoring_username.iter() {
        match config.task_type {
            TaskType::Initializing => {
                println!("====> Initializing <====");
                println!("====> Depending on the user's condition");
                println!("====> This might take some time");

                let conn = Connection::open(&config.db_path).expect("Unable to open the database");

                db::init_db(&conn).expect("Unable to initialize database");

                let user_profile_fetcher = UserInfoFetcher::new(username);
                let fetched_profile = user_profile_fetcher.fetch(&config).expect("Failed to fetch user profile");
                fetched_profile.write_to_db(&conn, &TaskType::Initializing).expect("Failed to write user profile to database");

                let tweet_fetcher = TweetFetcher::new(&fetched_profile.user.id, None);
                let (tweets, ref_tweets, ref_users) = tweet_fetcher.fetch(&config).expect("Failed to fetch user tweets");
                for ref_user in ref_users.into_iter() {
                    ref_user.write_to_db(&conn).expect("Failed to write referenced users to database");
                }
                for ref_tweet in ref_tweets.into_iter() {
                    ref_tweet.write_to_db(&conn).expect("Failed to write referenced tweet to database");
                }
                for tweet in tweets.into_iter() {
                    tweet.write_to_db(&conn).expect("Failed to write fetched tweet to database");
                }

                let like_fetcher = LikeFetcher::new(&fetched_profile.user.id, None);
                let (liked_tweet_records, liked_tweets, liked_users) = like_fetcher.fetch(&config).expect("Failed to fetch liked twitter");
                for liked_user in liked_users.into_iter() {
                    liked_user.write_to_db(&conn).expect("Failed to write liked users to database");
                }
                for liked_tweet in liked_tweets.into_iter() {
                    liked_tweet.write_to_db(&conn).expect("Failed to write liked tweet to database");
                }
                for liked_tweet_record in liked_tweet_records.into_iter() {
                    liked_tweet_record.write_to_db(&conn).expect("Failed to write liked tweet record to database");
                }

                let following_fetcher = FollowingFetcher::new(&fetched_profile.user.id, None);
                let (following_records, followed_users) = following_fetcher.fetch(&config, &conn).expect("Failed to fetch following users");
                for followed_user in followed_users.into_iter() {
                    followed_user.write_to_db(&conn).expect("Failed to write followed users to database");
                }
                for following_record in following_records.into_iter() {
                    following_record.write_to_db(&conn).expect("Failed to write following records to database");
                }
            }

            TaskType::Monitoring => {
                let arc_username = Arc::new(username.clone());
                let profile_username = Arc::clone(&arc_username);
                let tweet_username = Arc::clone(&arc_username);
                let like_username = Arc::clone(&arc_username);
                let following_username = Arc::clone(&arc_username);
                let conn = Connection::open(&config.db_path).expect("Unable to open the database");
                let user_profile = FetchedUser::get_records(&conn, &arc_username, None, 0).unwrap().into_iter().next().unwrap();
                drop(conn);
                let arc_user_id = Arc::new(user_profile.user.id.clone());
                let tweet_user_id = Arc::clone(&arc_user_id);
                let like_user_id = Arc::clone(&arc_user_id);
                let following_user_id = Arc::clone(&arc_user_id);

                let user_profile_config = config.clone();
                let user_tweet_config = config.clone();
                let user_following_config = config.clone();
                let user_like_config = config.clone();

                let user_profile_handler = thread::spawn(move || {

                    let conn = Connection::open(&user_profile_config.db_path).expect("Unable to open the database");
                    
                    loop {
                        let user_profile_fetcher = UserInfoFetcher::new(&profile_username);
                        let fetched_profile = user_profile_fetcher.fetch(&user_profile_config).expect("Failed to fetch user profile");
                        log::info!(
                            "{}: get user profile => user-id: {}, user_name: {}, name: {}", 
                            &profile_username, &fetched_profile.user.id, &fetched_profile.user.username, &fetched_profile.user.name
                        );
                        fetched_profile.write_to_db(&conn, &TaskType::Monitoring).expect("Failed to write user profile to database");
                        thread::sleep(time::Duration::from_secs(120));
                    }

                });

                let user_tweet_handler = thread::spawn(move || {
                    let conn = Connection::open(&user_tweet_config.db_path).expect("Unable to open the database");

                    loop {
                        let latest_tweet_id = FetchedTweet::newest_id(&conn, &tweet_user_id).expect("should get latest id record");
                        let tweet_fetcher = TweetFetcher::new(&tweet_user_id, latest_tweet_id.as_deref());
                        let (tweets, ref_tweets, ref_users) = tweet_fetcher.fetch(&user_tweet_config).expect("Failed to fetch user tweets");
                        for ref_user in ref_users.into_iter() {
                            ref_user.write_to_db(&conn).expect("Failed to write referenced users to database");
                        }
                        for ref_tweet in ref_tweets.into_iter() {
                            ref_tweet.write_to_db(&conn).expect("Failed to write referenced tweet to database");
                        }
                        for tweet in tweets.into_iter() {
                            log::info!(
                                "{}: get new tweet => text: {}, type: {:?}, created at: {}", 
                                &tweet_username, &tweet.text, &tweet.tweet_type, &tweet.created_at
                            );
                            tweet.write_to_db(&conn).expect("Failed to write fetched tweet to database");
                        }
                        thread::sleep(time::Duration::from_secs(60));
                    }

                });

                let user_liked_handler = thread::spawn(move || {
                    let conn = Connection::open(&user_like_config.db_path).expect("Unable to open the database");

                    loop {
                        let latest_like_id = LikedTweet::newest_id(&conn, &like_user_id).expect("should get liked id record");
                        let like_fetcher = LikeFetcher::new(&like_user_id, latest_like_id.as_deref());
                        let (liked_tweet_records, liked_tweets, liked_users) = like_fetcher.fetch(&user_like_config).expect("Failed to fetch liked twitter");
                        for liked_user in liked_users.into_iter() {
                            liked_user.write_to_db(&conn).expect("Failed to write liked users to database");
                        }
                        for liked_tweet in liked_tweets.into_iter() {
                            liked_tweet.write_to_db(&conn).expect("Failed to write liked tweet to database");
                        }
                        for liked_tweet_record in liked_tweet_records.into_iter() {
                            log::info!(
                                "{}: get new liked: text: {}, author: {}", 
                                &like_username, &liked_tweet_record.tweet.text, &liked_tweet_record.author.username
                            );
                            liked_tweet_record.write_to_db(&conn).expect("Failed to write liked tweet record to database");
                        }
                        thread::sleep(time::Duration::from_secs(60));
                    }
                });

                let user_follow_handler = thread::spawn(move || {
                    let conn = Connection::open(&user_following_config.db_path).expect("Unable to open the database");

                    loop {
                        let latest_follow_id = FollowingUser::get_newest_ids(&conn, &following_user_id).expect("should get liked id record");
                        let following_fetcher = FollowingFetcher::new(&following_user_id, Some(latest_follow_id));
                        let (following_records, followed_users) = following_fetcher.fetch(&user_following_config, &conn).expect("Failed to fetch following users");
                        for followed_user in followed_users.into_iter() {
                            followed_user.write_to_db(&conn).expect("Failed to write followed users to database");
                        }
                        for following_record in following_records.into_iter() {
                            log::info!(
                                "{}: get new following action => username: {}, action: {:?}", 
                                &following_username, &following_record.followed_user.username, &following_record.action
                            );
                            following_record.write_to_db(&conn).expect("Failed to write following records to database");
                        }
                        thread::sleep(time::Duration::from_secs(180));
                    }
                });

                user_profile_handler.join().unwrap();
                user_tweet_handler.join().unwrap();
                user_liked_handler.join().unwrap();
                user_follow_handler.join().unwrap();
            }
        }
    }
}
