# AL-Raid-Bot

[
    ![License GPL-3.0](https://img.shields.io/github/license/nnazo/al-raid-bot?style=flat-square)
](https://github.com/nnazo/al-raid-bot/blob/master/LICENSE)

## Build
1. Have docker and docker-compose installed
2. Run `docker-compose up -d`

Alternatively,
1. Have Rust and Cargo installed
2. Run `cargo run --release`

## Usage
`!stop-task`
Note that this stops the bot after the current task iteration.

To look through recent users:
 * note that depth is the number of pages to look through
```
!start-task {
    "User": {
        "channelId": "webhook_channel_id",
        "token": "webhook_token",
        "job": {
            "keywords": ["words", "or phrases"],
            "media_ids": [121, 999],
            "depth": 10,
            "maxScoreThreshold": 3
        }
    }
}
```

To look through recent activities:
```
!start-task {
    "Activity": {
        "channelId": "webhook_channel_id",
        "token": "webhook_token",
        "job": {
            "keywords": ["more", "words", "or phrases"],
            "userJob": {
                "keywords": ["the"],
                "media_ids": [],
                "maxScoreThreshold": 0
            }
        }
    }
}
```