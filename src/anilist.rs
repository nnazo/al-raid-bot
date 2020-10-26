use crate::resources::Query;
use anyhow::{anyhow, Result};
use reqwest::{blocking::Client, StatusCode};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Map, Value};
// use tokio::time;

#[derive(Deserialize, Debug)]
pub struct QueryError {
    pub message: Option<String>,
    pub status: Option<i32>,
}

#[derive(Deserialize, Debug)]
pub struct QueryResponse<R> {
    pub data: Option<R>,
    pub errors: Option<Vec<QueryError>>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PageInfo {
    has_next_page: bool,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct PageResponse<R> {
    pub page: Page<R>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Page<R> {
    // pub page_info: PageInfo,
    pub page: R,
}

pub type Activities = Option<Vec<Activity>>;
pub type ActivityReplies = Option<Vec<ActivityReply>>;
pub type MediaListEntries = Option<Vec<MediaList>>;
pub type Users = Option<Vec<User>>;

pub fn query_from_file<R>(
    query_path: &str,
    variables: &Option<Map<String, Value>>,
) -> Result<QueryResponse<R>>
where
    R: DeserializeOwned,
{
    let query: String = Query::get(query_path).map_or_else(
        || Err(anyhow!("could not load query from \"{}\"", query_path)),
        |query| {
            std::str::from_utf8(&*query).map_or_else(
                |err| {
                    Err(anyhow!(
                        "failed to covert \"{}\" query to utf8: {}",
                        query_path,
                        err
                    ))
                },
                |s| Ok(s.to_string()),
            )
        },
    )?;
    query_graphql(&query, variables)//.await
}

pub fn query_graphql<R>(
    query_str: &str,
    variables: &Option<Map<String, Value>>,
) -> Result<QueryResponse<R>>
where
    R: DeserializeOwned,
{
    let query = if let Some(vars) = &variables {
        json!({ "query": query_str, "variables": vars })
    } else {
        json!({ "query": query_str })
    };

    let max_rate_limit_count: i32 = 5;
    for _ in 0..max_rate_limit_count {
        let client = Client::new();
        let resp = client
            .post("https://graphql.anilist.co")
            .header("Content-Type", "application/json")
            .json(&query)
            .send()?;
            //.await?;

        match resp.status() {
            StatusCode::TOO_MANY_REQUESTS => {
                let secs;
                let retry = resp.headers().get("Retry-After");
                if let Some(val) = retry {
                    let header = String::from_utf8_lossy(val.as_bytes());
                    secs = header.parse::<u64>().unwrap_or(60);
                } else {
                    secs = 60;
                }

                let begin = std::time::Instant::now();
                loop {
                    println!("ratelimiting for {}s...", secs);
                    let since = std::time::Instant::now().checked_duration_since(begin);
                    if let Some(since) = since {
                        if since.as_secs() >= secs {
                            break;
                        }
                    }
                }
                
                // time::sleep(time::Duration::from_secs(secs))?;//.await; 
            }
            StatusCode::OK | _ => {
                // println!("resp: {}", resp.text().await?);
                let response: QueryResponse<R> = resp.json()?;//.await?;
                return Ok(response);
                // return Err(anyhow!("temp"));
            }
        }
    }

    Err(anyhow!("Exceeded the maximum rate limit count (5)"))
}

pub fn query_in_media_list(
    user_id: i32,
    media_ids: &Vec<i32>,
) -> Result<QueryResponse<PageResponse<MediaListEntries>>> {
    let variables = json!({
        "userId": user_id,
        "mediaIds": media_ids,
    });
    if let serde_json::Value::Object(variables) = variables {
        query_from_file("in_media_list.gql", &Some(variables))//.await
    } else {
        Err(anyhow!("media list query variables was not a json object"))
    }
}

pub fn query_activities(page: i32) -> Result<QueryResponse<PageResponse<Activities>>> {
    let variables = json!({
        "page": page,
    });
    if let serde_json::Value::Object(variables) = variables {
        query_from_file("activities.gql", &Some(variables))//.await
    } else {
        Err(anyhow!("activity query variables was not a json object"))
    }
}

pub fn query_activity_replies(
    page: i32,
    activity_id: i32,
) -> Result<QueryResponse<PageResponse<ActivityReplies>>> {
    let variables = json!({
        "page": page,
        "activityId": activity_id,
    });
    if let serde_json::Value::Object(variables) = variables {
        query_from_file("activity_replies.gql", &Some(variables))//.await
    } else {
        Err(anyhow!(
            "activity replies query variables was not a json object"
        ))
    }
}

pub fn query_users(page: i32) -> Result<QueryResponse<PageResponse<Users>>> {
    let variables = json!({
        "page": page,
    });
    if let serde_json::Value::Object(variables) = variables {
        query_from_file("users.gql", &Some(variables))//.await
    } else {
        Err(anyhow!(
            "users query variables was not a json object"
        ))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MediaList {
    pub score: Option<f64>,
    pub notes: Option<String>,
    pub media_id: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    pub activity_type: ActivityType,
    pub id: i32,
    pub user: User,
    pub recipient: Option<User>,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ActivityType {
    TextActivity,
    MessageActivity,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: i32,
    pub name: String,
    pub about: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ActivityReply {
    pub id: i32,
    pub activity_id: i32,
    pub user: User,
    pub text: String,
}
