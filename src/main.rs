use anyhow::Result;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{from_value, json, Value};

fn main() -> Result<()> {
    let api_key = std::env::var("NEWRELIC_API_KEY")?;
    let response: Value = query_nerdgraph(QUERY, "", &api_key)?;
    let entities: Vec<Entity> =
        from_value(response["data"]["actor"]["entitySearch"]["results"]["entities"].clone())?;
    let mut wtr = csv::Writer::from_writer(std::io::stdout());
    for entity in entities {
        wtr.serialize(Monitor::from(entity))?;
    }
    wtr.flush()?;
    Ok(())
}

const QUERY: &str = r#"{
  actor {
    entitySearch(query: "domain = 'SYNTH' AND type = 'MONITOR'") {
      results {
        entities {
          ... on SyntheticMonitorEntityOutline {
            name
            accountId
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
#[serde(rename_all = "camelCase")]
struct Entity {
    account_id: u32,
    monitor_type: String,
    monitored_url: Option<String>,
    name: String,
    period: u32,
    tags: Vec<Tag>,
}

#[derive(Debug, Deserialize)]
struct Tag {
    key: String,
    values: Vec<String>,
}

#[derive(Debug, Serialize)]
struct Monitor {
    account: Option<String>,
    account_id: u32,
    name: String,
    monitor_type: String,
    monitored_url: Option<String>,
    period: u32,
    monitor_status: Option<String>,
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
        }
    }
}

fn query_nerdgraph<T: DeserializeOwned>(query: &str, variables: &str, api_key: &str) -> Result<T> {
    let client = reqwest::blocking::Client::new();
    let response = client
        .post("https://api.newrelic.com/graphql")
        .header("API-Key", api_key)
        .json(&json!({"query": query, "variables": variables}))
        .send()?;
    Ok(response.json()?)
}
