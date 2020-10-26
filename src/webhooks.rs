use crate::anilist::*;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use reqwest::blocking::Client;
use crate::embeds::*;

#[derive(Serialize, Clone)]
pub struct WebhookRequest {
    pub embeds: Vec<Embed>,
}

impl WebhookRequest {
    pub fn from(url: String, title: String, fields: Vec<(String, String)>) -> Self {
        let mut embed = Embed::from(url, title);
        for (name, value) in fields {
            embed.fields.push(EmbedField::from(name, value));
        }
        WebhookRequest {
            embeds: [embed].to_vec(),
        }
    }
}



#[derive(Deserialize)]
pub enum WebhookJob {
    /// Find a user's account
    User(Job<UserJob>),
    /// Find an activity
    Activity(Job<ActivityJob>),
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Job<T> {
    pub channel_id: String,
    pub token: String,
    pub job: T,
}

impl<T> Job<T> {
    pub fn send_embed_activity(&self, activity: Activity, matches: Vec<(String, String)>) -> Result<()> {
        let url = self.url();
        let req = WebhookRequest::from(
            format!("https://anilist.co/activity/{}", activity.id),
            "Activity".to_string(),
            matches,
        );
        Self::send_embed(&req, &url)//.await
    }

    pub fn send_embed_user(&self, user: User, matches: Vec<(String, String)>) -> Result<()> {
        let url = self.url();
        let req = WebhookRequest::from(
            format!("https://anilist.co/user/{}", user.id),
            user.name,
            matches,
        );
        Self::send_embed(&req, &url)//.await
    }

    pub fn send_embed(embed: &WebhookRequest, url: &str) -> Result<()> {
        let client = Client::new();
        let _ = client
            .post(url)
            .header("Content-Type", "application/json")
            .json(embed)
            .send()?;
            // .await?;
        Ok(())
    }

    pub fn url(&self) -> String {
        format!("https://discord.com/api/webhooks/{}/{}", self.channel_id, self.token)
    }
}

#[derive(Deserialize, Clone)]
pub struct ActivityJob {
    /// Look through new activities for keywords
    pub keywords: Vec<String>,
    /// Optionally check the user's profile & list as well.
    pub user_job: Option<UserJob>,
    /// Activity IDs (parent) found
    pub found_activity_ids: Option<BTreeSet<i32>>,
    /// Activity reply ID -> parent activity ID
    _found_activity_reply_ids: Option<HashMap<i32, i32>>,
}

impl ActivityJob {
    pub fn find_activities(&mut self, page: i32) -> Result<Vec<(Activity, Vec<(String, String)>)>> {
        let mut matched_activities = vec![];
        println!("checking activities");
        let activity_resp = query_activities(page)?; //.await?;
        if let Some(activity_page) = activity_resp.data {
            let activities = activity_page.page;
            if let Some(activities) = activities.page {
                // Check each activity's content and user
                for activity in activities {
                    println!("{}", activity.id);

                    // Check activity content
                    if let Some(mut matches) = self.flag_activity(&activity) {
                        println!("  flagged activity");
                        // Check the username and bio
                        if let Some(user_job) = &mut self.user_job {
                            let user_matches = user_job.flag_user(&activity.user, &None);
                            if let Some(user_matches) = user_matches {
                                println!("user had stuff");
                                for m in user_matches {
                                    matches.push(m);
                                }
                            }
                        }
                        if let Some(found_activity_ids) = &mut self.found_activity_ids {
                            println!("init");
                            if !found_activity_ids.contains(&activity.id) {
                                println!("not sent yet");
                                found_activity_ids.insert(activity.id);
                                matched_activities.push((activity, matches));
                            } 
                        }
                    }
                }
            }
        }

        Ok(matched_activities)
    }

    pub fn flag_activity(&mut self, activity: &Activity) -> Option<Vec<(String, String)>> {
        let mut matches = Vec::new();

        for keyword in self.keywords.iter() {
            let keyword = keyword.to_lowercase();
            if activity.text.to_lowercase().contains(&keyword) {
                matches.push(("Activity".to_string(), format!("Contained keyword: {}", keyword)));
                matches.push(("User".to_string(), format!("https://anilist.co/user/{}", activity.user.id)));
                if let Some(recipient) = activity.recipient.clone() {
                    matches.push(("Message Recipient".to_string(), format!("https://anilist.co/user/{}", recipient.id)));
                }
            }
        }

        if matches.is_empty() {
            None
        } else {
            Some(matches)
        }
    }
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserJob {
    /// Check name and bio for keywords
    pub keywords: Vec<String>,
    /// Check the user's list entries for poorly rated media
    pub media_ids: Vec<i32>,
    /// How many multiples of 50 to look backwards for Users
    pub depth: i32,
    /// The score to check for (or below)
    pub max_score_threshold: Option<i32>,
    /// To keep track of already flagged uers
    pub found_user_ids: Option<BTreeSet<i32>>,
}

impl UserJob {
    pub fn find_users(&mut self, page: i32) -> Result<Vec<(User, Vec<(String, String)>)>> {
        let mut found_users = vec![];

        let user_resp = query_users(page)?; //.await?;
        if let Some(user_page) = user_resp.data {
            let users = user_page.page;
            if let Some(users) = users.page {
                // Check each user's info and list entries
                for user in users {
                    println!("{:#?}", user);
                    // Only check list entries if it was requested
                    let list = if !self.media_ids.is_empty() {
                        let list_resp = query_in_media_list(user.id, &self.media_ids)?; //.await?;
                        match list_resp.data {
                            Some(data) => data.page.page,
                            None => None,
                        }
                    } else {
                        None
                    };
                    println!("{:#?}", list);

                    // Check if user should be flagged or has been already flagged
                    if let Some(matches) =  self.flag_user(&user, &list) {
                        if let Some(found_user_ids) = &mut self.found_user_ids {
                            if !found_user_ids.contains(&user.id) {
                                found_user_ids.insert(user.id);
                                found_users.push((user, matches));
                            } 
                        }
                    }
                }
            }
        }

        Ok(found_users)
    }

    pub fn flag_user(&mut self, user: &User, matched_entries: &Option<Vec<MediaList>>) -> Option<Vec<(String, String)>> {
        let mut matches = Vec::new();
        for keyword in self.keywords.iter() {
            let keyword = keyword.to_lowercase();
            if user.name.to_lowercase().contains(&keyword) {
                matches.push(("Username".to_string(), format!("Username contained keyword: {}", keyword)));
            }
            if let Some(about) = &user.about {
                if about.to_lowercase().contains(&keyword) {
                    matches.push(("Bio".to_string(), format!("Bio contained keyword: {}", keyword)));
                }
            }
        }
        if let Some(entries) = matched_entries {
            println!("got entries");
            if let Some(user_matches) = self.flag_user_entries(entries) {
                for m in user_matches.into_iter() {
                    matches.push(m);
                }
            }
        }
        if matches.is_empty() {
            None
        } else {
            Some(matches)
        }
    }

    pub fn flag_user_entries(&mut self, entries: &Vec<MediaList>) -> Option<Vec<(String, String)>> {
        let max_score = self.max_score_threshold?;
        let mut matches = Vec::new();
        for entry in entries {
            if let Some(score) = entry.score {
                println!("{} <= {}", score, max_score);
                if score <= max_score as f64 && score != 0 as f64 {
                    matches.push(("List Entry Score".to_string(), format!("Media ID poorly scored: {}", entry.media_id)));
                }
            }
        }
        if matches.is_empty() {
            None
        } else {
            Some(matches)
        }
    }
}
