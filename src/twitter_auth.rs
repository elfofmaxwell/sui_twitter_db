use std::{time, error::Error, collections::HashMap};
use percent_encoding::AsciiSet;
use rand::{Rng};
use hmac::{Hmac, Mac};
use sha1::Sha1;

use crate::{configuration, request_builder::{RequestParams, RequestMethod}};


pub struct OAuthComponents {
    oauth_consumer_key: String, 
    oauth_consumer_secrete: String, 
    oauth_nonce: String, 
    oauth_signature: String, 
    oauth_signature_method: String, 
    oauth_timestamp: String, 
    oauth_token: String, 
    oauth_token_secrete: String, 
    oauth_version: String, 
}

impl OAuthComponents {
    fn builder(
        oauth_consumer_key: &str, 
        oauth_token: &str, 
        oauth_consumer_secrete: &str, 
        oauth_token_secrete: &str
    ) -> Result<OAuthComponents, Box<dyn Error>> {

        let oauth_nounce_base: [u8; 32] = rand::thread_rng().gen();
        let oauth_nounce: String = (&base64::encode(&oauth_nounce_base)).chars().map(
            |ch| {
                match ch {
                    'A'..='Z' => ch, 
                    'a'..='z' => ch, 
                    '0'..='9' => ch, 
                    _ => 'x'
                }
            }
        ).collect();
        
        Ok(
            OAuthComponents { 
                oauth_consumer_key: String::from(oauth_consumer_key),
                oauth_consumer_secrete: String::from(oauth_consumer_secrete),  
                oauth_nonce: oauth_nounce, 
                oauth_signature: String::from(""), 
                oauth_signature_method: String::from("HMAC-SHA1"), 
                oauth_timestamp: time::SystemTime::now()
                    .duration_since(time::UNIX_EPOCH)?
                    .as_secs().to_string(), 
                oauth_token: String::from(oauth_token), 
                oauth_token_secrete: String::from(oauth_token_secrete), 
                oauth_version: String::from("1.0") 
            }
        )
    }

    fn oauth_to_map(&self) -> HashMap<String, String> {
        HashMap::from([
            ("oauth_consumer_key".to_string(), (&self.oauth_consumer_key).to_string()), 
            ("oauth_nonce".to_string(), (&self.oauth_nonce).to_string()), 
            ("oauth_signature_method".to_string(), (&self.oauth_signature_method).to_string()), 
            ("oauth_timestamp".to_string(), (&self.oauth_timestamp).to_string()), 
            ("oauth_version".to_string(), (&self.oauth_version).to_string()), 
            ("oauth_token".to_string(), (&self.oauth_token).to_string()),
        ])
    }

    fn build_signature_base(
        &self, 
        url_base: &str, 
        request_param: &HashMap<String, String>, 
        request_method: &RequestMethod
    ) -> Result<String, Box<dyn Error>> {
        let mut signature_base = String::new();
        if let RequestMethod::Get = request_method {
            signature_base.push_str("GET");
        } else {
            signature_base.push_str("POST");
        }

        let mut request_param_pairs = percent_encode_pair(&request_param);

        let mut oauth_param_pairs = percent_encode_pair(&self.oauth_to_map());

        oauth_param_pairs.append(&mut request_param_pairs);
        oauth_param_pairs.sort();
        let param_string = oauth_param_pairs.join("&");
        let signature_base_complete = format!(
            "{}&{}&{}", 
            &signature_base, 
            &(percent_encode(url_base)), 
            &(percent_encode(&param_string))
        );
        Ok(signature_base_complete)
    }

    fn cal_signature(&self, signature_base: &str) -> String {
        let hmac_key = format!(
            "{}&{}", percent_encode(&self.oauth_consumer_secrete), 
            percent_encode(&self.oauth_token_secrete)
        );
        type HmacSha1 = Hmac<Sha1>; 
        let mut mac = HmacSha1::new_from_slice(
            &hmac_key.as_bytes()
        ).expect("The key should be valid");
        mac.update(signature_base.as_bytes());

        base64::encode(mac.finalize().into_bytes())
    }

    pub fn from_config(conf: &configuration::Config, params: &impl RequestParams) -> Result<(OAuthComponents, String), Box<dyn Error>> {
        let mut oauth_component = OAuthComponents::builder(&conf.oauth_consumer_key, &conf.oauth_token, &conf.oauth_consumer_secret, &conf.oauth_token_secret)?;

        let signature_base = oauth_component.build_signature_base(&params.get_base_url(), &params.to_hashmap(), &params.get_method())?;
        let signature = oauth_component.cal_signature(&signature_base);
        oauth_component.oauth_signature = signature.clone();

        let mut oauth_param_pairs = oauth_component.oauth_to_map(); 
        oauth_param_pairs.insert("oauth_signature".to_string(), signature);
        let mut formatted_pairs: Vec<String> = oauth_param_pairs.iter().map(|param_tup| {
            format!("{}=\"{}\"", param_tup.0, param_tup.1)
        }).collect();
        formatted_pairs.sort();
        let oauth_header = "OAuth ".to_string() + &formatted_pairs.join(", ");
        Ok((oauth_component, oauth_header))
    }

}


fn percent_encode(string_to_encode: &str) -> String {
    const NON_DOT_ASCII: &AsciiSet = &percent_encoding::NON_ALPHANUMERIC.remove(b'.').remove(b'_').remove(b'-').remove(b'~');
    percent_encoding::utf8_percent_encode(string_to_encode, NON_DOT_ASCII).to_string()
}

fn percent_encode_pair(param_map: &HashMap<String, String>) -> Vec<String> {
    param_map.iter().map(|param_tup| {
        format!("{}={}", percent_encode(param_tup.0), percent_encode(param_tup.1))
    }).collect()
}

#[cfg(test)]
mod tests {
    use crate::configuration::Config;

    use super::*;

    #[test]
    fn test_signature_base() -> () {
        let mut test_oauth_comp = OAuthComponents::builder("xvz1evFS4wEEPTGEFPHBog", "370773112-GmHxMAgYyLbNEtIKZeRNFsMKPR9EyMZeS9weJAEb", "kAcSOqF21Fu85e7zjz7ZN2U4ZRhfV3WpwPAoE3Z7kBw", "LswwdoUaIvS8ltyTt5jkRh4J50vUPVVHtR2YPi5kE").unwrap();
        test_oauth_comp.oauth_timestamp = String::from("1318622958");
        test_oauth_comp.oauth_nonce = String::from("kYjzVBB8Y0ZFabxSWbWovY3uYSQ2pTgmZeNu2VS4cg"); 

        let request_param_map = HashMap::from(
            [
                ("include_entities".to_string(), "true".to_string()), 
                ("status".to_string(), "Hello Ladies + Gentlemen, a signed OAuth request!".to_string())
            ]
        );
        let req_method = RequestMethod::Post;
        let signature_base = test_oauth_comp.build_signature_base("https://api.twitter.com/1.1/statuses/update.json", &request_param_map, &req_method).unwrap();

        let signature_base_ref = "POST&https%3A%2F%2Fapi.twitter.com%2F1.1%2Fstatuses%2Fupdate.json&include_entities%3Dtrue%26oauth_consumer_key%3Dxvz1evFS4wEEPTGEFPHBog%26oauth_nonce%3DkYjzVBB8Y0ZFabxSWbWovY3uYSQ2pTgmZeNu2VS4cg%26oauth_signature_method%3DHMAC-SHA1%26oauth_timestamp%3D1318622958%26oauth_token%3D370773112-GmHxMAgYyLbNEtIKZeRNFsMKPR9EyMZeS9weJAEb%26oauth_version%3D1.0%26status%3DHello%2520Ladies%2520%252B%2520Gentlemen%252C%2520a%2520signed%2520OAuth%2520request%2521";
        assert_eq!(&signature_base, signature_base_ref);

        let signature = test_oauth_comp.cal_signature(&signature_base);
        let signature_ref: &str = "hCtSmYh+iHYCEqBWrE7C7hYmtUk="; 
        assert_eq!(&signature, signature_ref);
    }
    
    struct IntimatedReqConf; 
    impl RequestParams for IntimatedReqConf {
        fn get_base_url(&self) -> String {
            "https://api.twitter.com/1.1/statuses/update.json".to_string()
        }
        
        fn get_method(&self) -> RequestMethod {
            RequestMethod::Post
        }

        fn to_hashmap(&self) -> HashMap<String, String> {
            HashMap::from(
                [
                    ("include_entities".to_string(), "true".to_string()), 
                    ("status".to_string(), "Hello Ladies + Gentlemen, a signed OAuth request!".to_string())
                ]
            )
        }
    }

    
    //#[test]
    //fn test_build_from_config() -> () {
    //    let test_conf = Config::configure("test_conf.yaml", true).unwrap();
    //    let intimated_req_conf = IntimatedReqConf;
    //    let (built_oauth, oauth_header) = OAuthComponents::from_config(&test_conf, &intimated_req_conf).unwrap();

    //    const ref_oauth_header: &str = r#"OAuth oauth_consumer_key="xvz1evFS4wEEPTGEFPHBog", oauth_nonce="kYjzVBB8Y0ZFabxSWbWovY3uYSQ2pTgmZeNu2VS4cg", oauth_signature="tnnArxj06cWHq44gCs1OSKk%2FjLY%3D", oauth_signature_method="HMAC-SHA1", oauth_timestamp="1318622958", oauth_token="370773112-GmHxMAgYyLbNEtIKZeRNFsMKPR9EyMZeS9weJAEb", oauth_version="1.0""#;
    //    assert_eq!(&oauth_header, ref_oauth_header)
    //}
    
}