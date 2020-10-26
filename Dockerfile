FROM rust:1.45

WORKDIR /usr/src/al-raid-bot
COPY . .
RUN cargo install --path .

CMD [ "al-raid-bot" ]