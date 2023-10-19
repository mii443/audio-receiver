use async_trait::async_trait;
use serenity::{
    model::{channel::Message, prelude::Ready},
    prelude::{Context, EventHandler},
};

use crate::audio_receiver::audio_receive;

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, message: Message) {
        if message.content == "audio-receive" {
            audio_receive(&ctx, &message).await;
        }
    }

    async fn ready(&self, _ctx: Context, _ready: Ready) {
        println!("bot ready");
    }
}
