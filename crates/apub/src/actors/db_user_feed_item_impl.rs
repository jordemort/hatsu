use chrono::SecondsFormat;
use hatsu_utils::{
    AppError,
    user::feed::Feed
};
use url::Url;

use super::{JsonUserFeed, JsonUserFeedItem};

impl JsonUserFeed {
    pub async fn get_feed(feed: Feed, name: &str) -> Result<Self, AppError> {
        match feed {
            Feed { json: Some(url), .. } => Ok(Self::parse_json_feed(url).await?),
            Feed { atom: Some(url), .. } => Ok(Self::parse_xml_feed(url).await?),
            Feed { rss: Some(url), .. } => Ok(Self::parse_xml_feed(url).await?),
            Feed { json: None, atom: None, rss: None, .. } => Err(AppError::not_found("Feed Url", name))
        }
    }

    pub async fn parse_json_feed(url: Url) -> Result<Self, AppError> {
        Ok(reqwest::get(url)
            .await?
            .json::<Self>()
            .await?)
    }

    pub async fn parse_xml_feed(url: Url) -> Result<Self, AppError> {
        let feed = feed_rs::parser::parse(
            reqwest::get(url)
                .await?
                .text()
                .await?
                .as_bytes()
        )?;
            
        let items = feed.entries
            .iter()
            .map(|entry| JsonUserFeedItem {
                id: entry.id.clone(),
                url: None, // TODO
                title: entry.title.clone().map(|text| text.content),
                summary: entry.summary.clone().map(|text| text.content),
                language: None,
                tags: entry.categories.iter().map(|category| Some(category.label.clone().unwrap_or_else(|| category.term.clone()))).collect(),
                date_published: entry.published.map(|date| date.to_rfc3339_opts(SecondsFormat::Secs, true)),
                date_modified: entry.updated.map(|date| date.to_rfc3339_opts(SecondsFormat::Secs, true)),
            })
            .collect();
    
        Ok(Self {
            hatsu: None,
            feed_url: Url::parse(&feed.id)?,
            next_url: None,
            title: feed.title.unwrap().content,
            description: feed.description.map(|text| text.content),
            icon: feed.icon.map(|image| Url::parse(&image.uri).unwrap()),
            language: feed.language,
            items,
        })
    }
}