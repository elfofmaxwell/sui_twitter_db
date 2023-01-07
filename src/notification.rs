use reqwest::blocking::Client;
use rusqlite::Connection;

use crate::{query_result::ToTelegramMsg, configuration::Config};

use std::error::Error;

pub fn send_tg(fetched: &impl ToTelegramMsg, conf: &Config, conn: &Connection) -> Result<(), Box<dyn Error>> {
    let target_url = format!("https://api.telegram.org/bot{}/sendMessage", &conf.bot_token);
    let client = Client::builder().build().expect("error in client builder");
    let tg_msgs = fetched.tg_msg(conf, conn)?;
    for tg_msg in tg_msgs.into_iter() {
        let tg_msg = tg_msg;
        let request = client.post(&target_url).json(&tg_msg);
        log::info!("TG message sent: {}", request.send()?.text()?);
    }
    Ok(())
}

pub fn tg_escape(text: &str) -> String {
    text.chars().map(|c|
        match c {
            '_' => "\\_".to_string(), 
            '*' => "\\*".to_string(),
            '`' => "\\`".to_string(), 
            _ => c.to_string()
    }).collect()
}