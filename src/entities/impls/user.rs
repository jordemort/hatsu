use std::env;

use activitypub_federation::{
    config::Data,
    http_signatures::generate_actor_keypair,
    protocol::verification::verify_domains_match,
    traits::{Actor, Object}
};
use chrono::{Local, NaiveDateTime};
use sea_orm::*;
use url::Url;

use crate::{
    AppData,
    entities::{
        prelude::*,
        user::Model as DbUser
    },
    error::AppError,
    objects::user::Person,
    utilities::get_site_feed,
};

impl DbUser {
  // 创建新用户
  // Create a new user
  // TODO: 从网站获取数据
  // TODO: Getting data from websites
  pub async fn new(preferred_username: &str) -> Result<Self, AppError> {
      let hostname = env::var("HATSU_DOMAIN")?;
      let id = Url::parse(&format!("https://{}/u/{}", hostname, &preferred_username))?;
      let inbox = Url::parse(&format!("https://{}/u/{}/inbox", hostname, &preferred_username))?;
      let outbox = Url::parse(&format!("https://{}/u/{}/outbox", hostname, &preferred_username))?;
      let keypair = generate_actor_keypair()?;

      let feed = get_site_feed(preferred_username.to_string()).await?;

      tracing::info!(
          "User Feed: {}, {}, {}",
          feed.json.unwrap_or_else(|| "null".to_string()),
          feed.atom.unwrap_or_else(|| "null".to_string()),
          feed.rss.unwrap_or_else(|| "null".to_string()),
      );

      Ok(Self {
          id: id.to_string(),
          name: "Hatsu".to_string(),
          preferred_username: preferred_username.to_string(),
          inbox: inbox.to_string(),
          outbox: outbox.to_string(),
          local: true,
          public_key: keypair.public_key,
          private_key: Some(keypair.private_key),
          last_refreshed_at: Local::now().naive_local().format("%Y-%m-%d %H:%M:%S").to_string(),
          // followers: vec![],
      })
  }
}

#[async_trait::async_trait]
impl Object for DbUser {
    type DataType = AppData;
    type Kind = Person;
    type Error = AppError;

    fn last_refreshed_at(&self) -> Option<NaiveDateTime> {
        Some(NaiveDateTime::parse_from_str(&self.last_refreshed_at, "%Y-%m-%d %H:%M:%S").unwrap())
    }

    // 从 ID 读取
    async fn read_from_id(
        object_id: Url,
        data: &Data<Self::DataType>,
    ) -> Result<Option<Self>, Self::Error> {
        Ok(User::find_by_id(&object_id.to_string())
            .one(&data.conn)
            .await?)
    }

    // 转换为 ActivityStreams JSON
    async fn into_json(self, _data: &Data<Self::DataType>) -> Result<Self::Kind, Self::Error> {
        Ok(Person {
            name: self.name.clone(),
            preferred_username: self.preferred_username.clone(),
            kind: Default::default(),
            id: Url::parse(&self.id).unwrap().into(),
            inbox: Url::parse(&self.inbox)?,
            outbox: Url::parse(&self.outbox)?,
            public_key: self.public_key(),
        })
    }

    // 验证
    async fn verify(
        json: &Self::Kind,
        expected_domain: &Url,
        _data: &Data<Self::DataType>,
    ) -> Result<(), Self::Error> {
        verify_domains_match(json.id.inner(), expected_domain)?;
        Ok(())
    }

    // 转换为本地格式
    async fn from_json(
        json: Self::Kind,
        _data: &Data<Self::DataType>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            id: json.id.to_string(),
            name: json.name,
            preferred_username: json.preferred_username,
            inbox: json.inbox.to_string(),
            outbox: json.outbox.to_string(),
            public_key: json.public_key.public_key_pem,
            private_key: None,
            last_refreshed_at: Local::now().naive_local().format("%Y-%m-%d %H:%M:%S").to_string(),
            // followers: vec![],
            local: false,
        })
    }

    // 删除用户
    async fn delete(self, data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        let _delete_user = User::delete_by_id(&self.id.to_string())
            .exec(&data.conn)
            .await?;
        Ok(())
    }
}

impl Actor for DbUser {
    fn id(&self) -> Url {
        Url::parse(&self.id).unwrap()
    }

    fn public_key_pem(&self) -> &str {
        &self.public_key
    }

    fn private_key_pem(&self) -> Option<String> {
        self.private_key.clone()
    }

    fn inbox(&self) -> Url {
        Url::parse(&self.inbox).unwrap()
    }
}
