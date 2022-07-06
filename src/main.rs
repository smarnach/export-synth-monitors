use anyhow::{Context, Result};
use futures::{stream::FuturesUnordered, TryStreamExt};
use nerdgraph::Client;
use std::fs::File;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let api_key = std::env::var("NEWRELIC_API_KEY")?;
    export(Box::leak(api_key.into_boxed_str())).await?;
    Ok(())
}

async fn export(api_key: &'static str) -> Result<()> {
    let client = Client::new(api_key);
    let response: monitor::Response = client.query(monitor::QUERY, "").await?;
    let entities = response.data.actor.entity_search.results.entities;
    let csv_file = File::create("output/monitor.csv").context("CSV file creation failed")?;
    let mut wtr = csv::Writer::from_writer(csv_file);
    let exports = FuturesUnordered::new();
    for entity in entities {
        if entity.monitor_type.starts_with("SCRIPT") {
            exports.push(export_js(client.clone(), entity.clone()));
        }
        wtr.serialize(monitor::Monitor::from(entity))?;
    }
    wtr.flush()?;
    exports.try_collect().await
}

async fn export_js(client: Client<'static>, entity: monitor::Entity) -> Result<()> {
    let js = script::get(&client, entity.account_id, &entity.guid).await?;
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
    use super::nerdgraph::Client;
    use anyhow::{Error, Result};
    use serde_json::Value;

    pub async fn get(client: &Client<'_>, account_id: u32, guid: &str) -> Result<String> {
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
        let mut response: Value = client.query(&query, "").await?;
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

mod nerdgraph;
