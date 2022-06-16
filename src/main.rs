use std::fs::File;

use anyhow::{Context, Result};
use tokio::task::spawn;
use serde::de::DeserializeOwned;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    let api_key = std::env::var("NEWRELIC_API_KEY")?;
    export(Box::leak(api_key.into_boxed_str())).await?;
    Ok(())
}

async fn export(api_key: &'static str) -> Result<()> {
    let response: monitor::Response = query_nerdgraph(monitor::QUERY, "", api_key).await?;
    let entities = response.data.actor.entity_search.results.entities;
    let csv_file = File::create("output/monitor.csv").context("CSV file creation failed")?;
    let mut wtr = csv::Writer::from_writer(csv_file);
    let mut js_tasks = vec![];
    for entity in entities {
        if entity.monitor_type.starts_with("SCRIPT") {
            js_tasks.push(spawn(export_js(entity.clone(), api_key)));
        }
        wtr.serialize(monitor::Monitor::from(entity))?;
    }
    wtr.flush()?;
    for task in js_tasks {
        task.await??;
    }
    Ok(())
}

async fn export_js(entity: monitor::Entity, api_key: &str) -> Result<()> {
    let js = script::get(entity.account_id, &entity.guid, api_key).await?;
    let name = entity.name.replace('/', "_");
    std::fs::write(format!("output/scripts/{name}.js"), js)?;
    Ok(())
}

mod monitor {
    use serde::{Deserialize, Serialize};

    pub const QUERY: &str = r#"{
      actor {
        entitySearch(query: "domain = 'SYNTH' AND type = 'MONITOR'") {
          results {
            entities {
              ... on SyntheticMonitorEntityOutline {
                name
                accountId
                guid
                monitorType
                monitoredUrl
                period
                tags {
                  key
                  values
                }
              }
            }
          }
        }
      }
    }"#;

    #[derive(Debug, Deserialize)]
    pub struct Response {
        pub data: Data,
    }
    #[derive(Debug, Deserialize)]
    pub struct Data {
        pub actor: Actor,
    }
    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Actor {
        pub entity_search: Results,
    }
    #[derive(Debug, Deserialize)]
    pub struct Results {
        pub results: Entities,
    }
    #[derive(Debug, Deserialize)]
    pub struct Entities {
        pub entities: Vec<Entity>,
    }
    #[derive(Clone, Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Entity {
        pub account_id: u32,
        pub monitor_type: String,
        pub monitored_url: Option<String>,
        pub name: String,
        pub period: u32,
        pub tags: Vec<Tag>,
        pub guid: String,
    }
    #[derive(Clone, Debug, Deserialize)]
    pub struct Tag {
        key: String,
        values: Vec<String>,
    }

    #[derive(Debug, Serialize)]
    pub struct Monitor {
        account: Option<String>,
        account_id: u32,
        name: String,
        monitor_type: String,
        monitored_url: Option<String>,
        period: u32,
        monitor_status: Option<String>,
        guid: String,
    }

    impl From<Entity> for Monitor {
        fn from(entity: Entity) -> Self {
            let mut monitor_status = None;
            let mut account = None;
            for tag in entity.tags {
                if tag.key == "monitorStatus" {
                    monitor_status = tag.values.into_iter().next();
                } else if tag.key == "account" {
                    account = tag.values.into_iter().next();
                }
            }
            Monitor {
                account_id: entity.account_id,
                monitor_type: entity.monitor_type,
                monitored_url: entity.monitored_url,
                name: entity.name,
                period: entity.period,
                monitor_status,
                account,
                guid: entity.guid,
            }
        }
    }
}

mod script {
    use anyhow::{Error, Result};
    use serde_json::Value;

    pub async fn get(account_id: u32, guid: &str, api_key: &str) -> Result<String> {
        let query = format!(
            r#"{{
          actor {{
            account(id: {account_id}) {{
              synthetics {{
                script(monitorGuid: "{guid}") {{
                  text
                }}
              }}
            }}
          }}
        }}"#
        );
        let mut response: Value = super::query_nerdgraph(&query, "", api_key).await?;
        loop {
            response = match response {
                Value::Object(map) => map.into_iter().next().unwrap().1,
                _ => break,
            }
        }
        match response {
            Value::String(s) => Ok(s),
            _ => Err(Error::msg("invalid script query response")),
        }
    }
}

async fn query_nerdgraph<T: DeserializeOwned>(
    query: &str,
    variables: &str,
    api_key: &str,
) -> Result<T> {
    let client = reqwest::Client::new();
    let response = client
        .post("https://api.newrelic.com/graphql")
        .header("API-Key", api_key)
        .json(&json!({"query": query, "variables": variables}))
        .send()
        .await?;
    Ok(response.json().await?)
}
