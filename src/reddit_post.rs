use misc;
use regex::Regex;
use serde::Deserialize;
use serde::Serialize;
use std::error::Error;
use std::fs;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RedditPost {
    pub score: i32,
    pub url: String,
    pub title: String,
    pub date: String,
    pub author: String,
}

pub fn list_levels(path: &str) -> Result<Vec<RedditPost>, Box<dyn Error>> {
    let json = fs::read_to_string(path)?;
    let json: Vec<RedditPost> =
        serde_json::from_str(&json).expect("Failed to deserialize JSON data");
    Ok(json)
}

const PATTERN: &str = "(?s)\
			(\
			Hexcells level v1\n\
			[^\n]*\n\
			(?:[^\n]*\n){3}\
			(?:(?:[^\n]*\\.\\.[^\n]*\n)){32}\
			[^\n]*\\.\\.[^\n<]*\
			)\
			[\n<]";

pub fn strdefns_of_post(
    level: &RedditPost,
    cache_dir: &str,
) -> Result<Vec<String>, Box<dyn Error>> {
    let html = misc::get_url_with_cache(&level.url, cache_dir)?;
    let regex = Regex::new(PATTERN)?;
    let occurrences: Vec<_> = regex.captures_iter(&html).collect();
    let mut res = vec![];
    for occ in occurrences {
        let s = occ.get(1).ok_or("Unreachable")?.as_str().to_string();
        res.push(s)
    }
    Ok(res)
}
