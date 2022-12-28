use std::{process, fs};
use clap::Parser;
use sui_twitter_db::{configuration::{Config, Args}, request_builder::{UserInfoFetcher, TweetFetcher}};

fn main() {
    
    let args = Args::parse();

    let config = match Config::configure(&args.conf_path, args.verbose) {
        Ok(config) => config, 
        Err(e) => {
            eprintln!("{e}");
            process::exit(1);
        }
    };

    for username in config.monitoring_username.iter() {
        let user_info = UserInfoFetcher::new(username); 
        let fetched_user = dbg!(user_info.fetch(&config).expect("Error in fetch user!"));
        let tweet_query = TweetFetcher::new(&fetched_user.data[0].id);
        let query_result = tweet_query.fetch(&config).expect("Error in query tweet!");
        //fs::write("./test_result.json", &query_result).expect("Unable to write to result file");
        dbg!(query_result);
    }
}
