use std::process;
use clap::Parser;
use sui_twitter_db::{configuration::{Config, Args}, request_builder::UserInfoFetcher};

fn main() {
    
    let args = Args::parse();

    let config = match Config::configure(&args.conf_path, args.verbose) {
        Ok(config) => config, 
        Err(e) => {
            eprintln!("{e}");
            process::exit(1);
        }
    };

    let user_info = UserInfoFetcher::new("YukinoMashiro"); 
    dbg!(user_info.fetch(&config).expect("Error in fetch!"));
}
