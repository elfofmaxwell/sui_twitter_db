use std::{fs, error::Error};
use clap::{Parser, ValueEnum};
use serde::{Serialize, Deserialize};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum TaskType {
    Initializing, 
    Monitoring
}

/// `FileConfig` structure saving configurations from config file
#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct FileConfig {
    bearer_token: String,
    monitoring_username: Vec<String>, 
    db_path: String,
}

/// `Config` structure saving configurations required for running
#[derive(PartialEq, Debug, Clone)]
pub struct Config {
    pub conf_path: String, 
    pub bearer_token: String,
    pub monitoring_username: Vec<String>, 
    pub db_path: String,
    pub verbose: bool, 
    pub task_type: TaskType
}

impl Config {
    /// `configure`: construct `Config` with provided `verbose` and from specified configuration yaml. 
    /// # Arguments
    /// * `conf_path`: the path to the yaml configuration path. 
    /// * `verbose`: the level of details printed
    /// # Returns
    /// the result of `Config`. 
    /// # Errors
    /// * [`std::io::Error`]
    /// * `serde_yaml::Error`
    /// * `sui_twitter_monitor::OptionError`: invalid option entry(ies)
    pub fn configure(
        conf_path: &str, 
        verbose: bool, 
        task_type: &TaskType,
    ) -> Result<Config, Box<dyn Error>> {
        let conf_yaml_str = fs::read_to_string(conf_path)?;
        let conf_file_options: FileConfig = serde_yaml::from_str(&conf_yaml_str)?;
        Ok(Config {
            conf_path: String::from(conf_path), 
            bearer_token: conf_file_options.bearer_token,
            monitoring_username: conf_file_options.monitoring_username, 
            db_path: conf_file_options.db_path,
            verbose: verbose,
            task_type: task_type.clone()
        })
    }
}

/// Following a twitter user's activities
#[derive(Debug, Parser)]
pub struct Args {
    /// Path of the config file
    #[arg(short, long, default_value = "./config.yaml", value_name = "FILE_PATH")]
    pub conf_path: String, 

    /// Verbose level
    #[arg(short, long)]
    pub verbose: bool,

    #[arg(value_enum)]
    pub task_type: TaskType
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conf_constructor () {
        let conf_path = "test_conf.yaml";
        let verbose = true; 

        let target_conf = Config {
            conf_path: String::from(conf_path), 
            bearer_token: String::from("aaaabbbb"),
            db_path: String::from("./sui.db"), 
            monitoring_username: vec![String::from("@suisei"), String::from("@miko")],
            verbose: verbose,
            task_type: TaskType::Monitoring
        };

        assert_eq!(target_conf, Config::configure("test_conf.yaml", verbose, &TaskType::Monitoring).unwrap());
    }
}