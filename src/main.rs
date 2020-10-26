use anyhow::anyhow;
use anyhow::Result;
use discord::{
    model::{ChannelId, Event, Message, ReadyEvent},
    Connection, Discord,
};
use std::collections::BTreeSet;
use once_cell::sync::Lazy;
use std::sync::Mutex;

pub mod anilist;
pub mod resources;
pub mod embeds;
pub mod webhooks;
pub mod util;

use webhooks::*;

static CANCEL: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().unwrap();
    let token = dotenv::var("BOT_TOKEN").expect("discord token not provided");
    let discord = Discord::from_bot_token(&token).expect("could not initialize bot");
    let (connection, ready_event) = discord.connect().expect("discord connection failed");
    Box::leak(Box::new(RaidBot {
        discord,
        connection,
        _ready_event: ready_event,
        job: None,
        join_handle: None,
        // page: 1,
    })).run().await
}

struct RaidBot {
    discord: Discord,
    connection: Connection,
    _ready_event: ReadyEvent,
    job: Option<WebhookJob>,
    join_handle: Option<tokio::task::JoinHandle<()>>
    // page: i32,
}

impl RaidBot {
    async fn run(&'static mut self) -> Result<()> {
        loop {
            match self.connection.recv_event() {
                Ok(Event::MessageCreate(message)) => {
                    match self.handle_message(message).await { //.await
                        Ok(_) => {
                            if let Some(_) = self.join_handle {
                                continue;
                            }
                            match &mut self.job {
                                Some(WebhookJob::Activity(job)) => {
                                    let mut job = job.clone();
                                    self.join_handle = Some(tokio::task::spawn(async move {
                                        let mut cancel = {
                                            *CANCEL.lock().unwrap()
                                        };
                                        while !cancel {
                                            util::wait(10);
                                            match job.job.find_activities(1) { //.await
                                                Ok(activities) => {
                                                    for (activity, matches) in activities {
                                                        if let Err(err) = job.send_embed_activity(activity, matches) { //.await
                                                            println!("could not send user embed: {:?}", err);
                                                        }
                                                    }
                                                },
                                                Err(err) => {
                                                    println!("err in find activities: {:?}", err);
                                                }
                                            }
                                            cancel = {
                                                *CANCEL.lock().unwrap()
                                            };
                                        }
                                    }));
                                }
                                Some(WebhookJob::User(job)) => {
                                    // Look through `depth` pages of users
                                    let mut job = job.clone();
                                    self.join_handle = Some(tokio::task::spawn(async move {
                                        let mut cancel = {
                                            *CANCEL.lock().unwrap()
                                        };
                                        loop {
                                            let depth = job.job.depth + 1;
                                            for page in 1..depth {
                                                println!("page #: {}", page);
                                                match job.job.find_users(page) { //.await
                                                    Ok(users) => {
                                                        for (user, matches) in users {
                                                            if let Err(err) = job.send_embed_user(user, matches) { //.await
                                                                println!("could not send user embed: {:?}", err);
                                                            }
                                                        }
                                                    },
                                                    Err(err) => {
                                                        println!("err in find users: {:?}", err);
                                                    }
                                                }
                                                cancel = {
                                                    *CANCEL.lock().unwrap()
                                                };
                                                if cancel {
                                                    break;
                                                }
                                                // util::wait(2);
                                            }
                                            if cancel {
                                                break;
                                            }
                                        }
                                    }));
                                }
                                None => {}
                            }
                        },
                        Err(err) => {
                            println!("message recv err: {:?}", err);
                        }
                    }
                }
                Ok(_) => {}
                Err(discord::Error::Closed(code, body)) => {
                    println!("Gateway closed. code {:?}: {}", code, body);
                    break;
                }
                Err(err) => {
                    println!("Error receiving event: {:?}", err);
                }
            }
        }

        // self.discord.send_embed()

        Ok(())
    }

    async fn handle_message(&mut self, message: Message) -> Result<()> {
        if message.content.starts_with('!') {
            let mut cmd_iter = message.content.splitn(2, ' ');
            let cmd = cmd_iter
                .next()
                .ok_or(anyhow!("no command present in iterator"))?;

            if cmd.starts_with("!start-task") {
                let body = cmd_iter
                    .next()
                    .ok_or(anyhow!("no body accompanying command"))?;
                let mut job = serde_json::from_str::<WebhookJob>(body)?;
                match &mut job {
                    WebhookJob::User(job) => {
                        job.job.found_user_ids = Some(BTreeSet::new());
                    },
                    WebhookJob::Activity(job) => {
                        job.job.found_activity_ids = Some(BTreeSet::new());
                    }
                }
                if let Some(handle) = &mut self.join_handle {
                    println!("cancelling task to replace it...");
                    handle.abort();
                    *CANCEL.lock().unwrap() = true;
                    println!("task cancelled? {:?}", handle.await);
                }
                self.join_handle = None;
                self.job = Some(job);
                *CANCEL.lock().unwrap() = false;
                self.handle_message_response(
                    message.channel_id,
                    "Started task successfully. Any previously running task was cancelled.",
                )?;
            } else if cmd.starts_with("!stop-task") {
                self.job = None;
                if let Some(handle) = &mut self.join_handle {
                    println!("cancelling task...");
                    handle.abort();
                    *CANCEL.lock().unwrap() = true;
                    println!("task cancelled? {:?}", handle.await);
                }
                self.join_handle = None;
                // self.page = 1;
                self.handle_message_response(message.channel_id, "Stopping current task.")?;
            }

            Ok(())
        } else {
            Err(anyhow!("no command detected"))
        }
    }

    fn handle_message_response(&mut self, channel_id: ChannelId, description: &str) -> Result<()> {
        self.discord
            .send_embed(channel_id, "", |embed| embed.description(description))?;
        Ok(())
    }

}

/*
Example commands

!start-raid {
    "User": {
        "channelId": "id",
        "token": "tok",
        "job": {
            "keywords": ["the"],
            "media_ids": [121],
            "depth": 10,
            "maxScoreThreshold": 10
        }
    }
}

!start-raid {
    "Activity": {
        "channelId": "id",
        "token": "tok",
        "job": {
            "keywords": ["the"],
            "userJob": {
                "keywords": ["the"],
                "media_ids": [121],
                "depth": 10,
                "maxScoreThreshold": 10
            }
        }
    }
}

*/