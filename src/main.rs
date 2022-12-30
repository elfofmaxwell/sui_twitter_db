use std::{process, fs};
use clap::Parser;
use sui_twitter_db::{configuration::{Config, Args}, request_builder::{UserInfoFetcher, TweetFetcher, LikeFetcher}};

fn main() {
    
    let args = Args::parse();

    let config = match Config::configure(&args.conf_path, args.verbose, &args.task_type) {
        Ok(config) => config, 
        Err(e) => {
            eprintln!("{e}");
            process::exit(1);
        }
    };

    for username in config.monitoring_username.iter() {
        let user_info = UserInfoFetcher::new(username); 
        let fetched_user = dbg!(user_info.fetch(&config).expect("Error in fetch user!"));
        //let tweet_query = TweetFetcher::new(&fetched_user.id, Some("1603963467396157440"));
        let like_query = LikeFetcher::new(&fetched_user.id, None);
        //let (query_result, _, _) = tweet_query.fetch(&config).expect("Error in query tweet!");
        let query_result = like_query.fetch(&config).expect("error in fetch like");
        //fs::write("./test_result.json", &query_result).expect("Unable to write to result file");
        println!("{:#?}", query_result);
    }
}
