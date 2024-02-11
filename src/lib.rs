use chrono::{self, DateTime, FixedOffset, TimeDelta};
use colored::*;
use quick_xml::de::from_str;
use rss::{Channel, Item};
use serde;
use std::{error::Error, str::FromStr};

#[derive(Debug, serde::Deserialize)]
pub struct Opml {
    #[serde(rename = "@version")]
    pub version: String,
    pub head: Head,
    pub body: BodyList,
}

#[derive(Debug, serde::Deserialize)]
pub struct Head {
    pub title: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct BodyList {
    pub outline: Vec<Outline>,
}

#[derive(Debug, serde::Deserialize)]
pub struct Outline {
    #[serde(rename = "@text")]
    pub text: Option<String>,
    #[serde(rename = "@title")]
    pub title: String,
    #[serde(rename = "@type")]
    pub outline_type: String,
    #[serde(rename = "@xmlUrl")]
    pub xml_url: String,
}

pub fn read_opml(file: &str) -> Result<Opml, Box<dyn Error>> {
    let content = std::fs::read_to_string(file)?;
    let doc: Opml = from_str(&content)?;
    Ok(doc)
}

pub fn get_rss_outlines(opml: &Opml) -> Vec<&Outline> {
    opml.body
        .outline
        .iter()
        .filter(|outline| outline.outline_type == "rss")
        .collect()
}

pub async fn read_feed(url: &str) -> Result<Channel, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let content = client.get(url).send().await?.bytes().await?;
    let channel = Channel::read_from(&content[..])?;
    Ok(channel)
}

pub struct FeedItem {
    guid: Option<String>,
    pub title: String,
    pub link: String,
    pub pub_date: DateTime<FixedOffset>,
    pub source_name: String,
    pub source_url: String,
}

impl FeedItem {
    pub fn make(item: &Item, source_name: &str, source_link: &str) -> Result<Self, String> {
        let guid = item.guid().map(|x| x.value.clone());
        let title = item
            .title()
            .map(|s| s.to_owned())
            .ok_or("Title not found".to_owned())?;
        let link = item
            .link()
            .map(|s| s.to_owned())
            .ok_or("Link not found".to_owned())?;
        let raw_pub_date = item.pub_date().ok_or("Pub date not found")?;
        let pub_date = DateTime::parse_from_rfc2822(raw_pub_date)
            .or(DateTime::from_str(raw_pub_date))
            .map_err(|err| err.to_string())?;
        Ok(FeedItem {
            guid,
            title,
            link,
            pub_date,
            source_name: source_name.to_owned(),
            source_url: source_link.to_owned(),
        })
    }

    /// Returns guid of the item. If not found, then constructs pseudo guid from title and link
    pub fn get_id(&self) -> String {
        self.guid
            .as_ref()
            .map(|s| s.clone())
            .unwrap_or_else(|| format!("{}-{}", self.title, self.link))
    }

    pub fn show(&self, now: DateTime<FixedOffset>, already_seen: bool) {
        let title = self.title.as_str();
        let link = self.link.as_str();
        let source = self.source_name.as_str();
        let dt_ago = date_diff(now - self.pub_date);
        if already_seen {
            println!("{}: {} ({}) {}", source, title.hidden(), dt_ago.dimmed(), link);
        } else {
            println!("{} (*new*): {} ({}) {}", source, title.bold(), dt_ago.dimmed(), link);
        }
    }
}

pub fn read_feed_items(channel: &Channel) -> Vec<FeedItem> {
    let converted = channel
        .items()
        .iter()
        .map(|item| FeedItem::make(item, channel.title(), channel.link()));

    let failed = converted.clone().filter_map(Result::err);
    let successful = converted.filter_map(Result::ok);

    failed.for_each(|err| eprintln!("{} Invalid RSS item in feed: {}", "[WARNING]".red(), err));
    successful.collect()
}

/// Converts time delta to human friendly string
/// e.g. "just now", "1 day ago", etc
pub fn date_diff(delta: TimeDelta) -> String {
    if delta.num_days() == 365 {
        "year ago".to_owned()
    } else if delta.num_days() > 365 {
        format!("{} years ago", delta.num_days() % 365)
    } else if delta.num_weeks() == 4 {
        "month ago".to_owned()
    } else if delta.num_weeks() > 4 {
        format!("{} months ago", delta.num_days() % 30)
    } else if delta.num_weeks() == 1 {
        "week ago".to_owned()
    } else if delta.num_weeks() > 1 {
        format!("{} weeks ago", delta.num_weeks())
    } else if delta.num_days() == 1 {
        "day ago".to_owned()
    } else if delta.num_days() > 1 {
        format!("{} days ago", delta.num_days())
    } else if delta.num_hours() == 1 {
        "hour ago".to_owned()
    } else if delta.num_hours() > 1 {
        format!("{} hours ago", delta.num_hours())
    } else if delta.num_minutes() == 1 {
        "minute ago".to_owned()
    } else if delta.num_minutes() > 1 {
        format!("{} minutes ago", delta.num_minutes())
    } else {
        "just now".to_owned()
    }
}
