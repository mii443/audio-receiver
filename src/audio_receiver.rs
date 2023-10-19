use std::{collections::HashMap, io::Write, sync::Arc};

use async_trait::async_trait;
use byteorder::{LittleEndian, WriteBytesExt};
use serenity::{futures::lock::Mutex, model::channel::Message, prelude::Context};
use songbird::{
    model::payload::Speaking, CoreEvent, Event, EventContext, EventHandler as VoiceEventHandler,
};

#[derive(Clone)]
struct Receiver {
    pub ssrc_map: Arc<Mutex<HashMap<u32, u64>>>,
    pub audio_data: Arc<Mutex<HashMap<u32, Vec<i16>>>>,
}

impl Receiver {
    pub fn new() -> Self {
        Self {
            ssrc_map: Arc::new(Mutex::new(HashMap::new())),
            audio_data: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

fn pcm_to_wav(pcm: Vec<i16>) -> Vec<u8> {
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: 48000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut wav: Vec<u8> = vec![];
    wav.write_all(&spec.into_header_for_infinite_file())
        .unwrap();
    for &n in &pcm {
        wav.write_i16::<LittleEndian>(n).unwrap();
    }

    wav
}

#[async_trait]
impl VoiceEventHandler for Receiver {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        match ctx {
            EventContext::SpeakingStateUpdate(Speaking { ssrc, user_id, .. }) => {
                self.ssrc_map.lock().await.insert(*ssrc, user_id.unwrap().0);
            }
            EventContext::SpeakingUpdate(data) => {
                if !data.speaking {
                    let audio =
                        if let Some(audio) = self.audio_data.lock().await.clone().get(&data.ssrc) {
                            Some(audio.clone())
                        } else {
                            None
                        };

                    if !self.ssrc_map.lock().await.contains_key(&data.ssrc) {
                        return None;
                    }

                    if let Some(audio) = audio {
                        let wav = pcm_to_wav(audio.clone());
                        let mut file =
                            std::fs::File::create(format!("./audio/{}.wav", data.ssrc)).unwrap();

                        file.write_all(&wav).unwrap();
                        file.flush().unwrap();

                        self.audio_data.lock().await.remove(&data.ssrc);
                    }
                }
            }
            EventContext::VoicePacket(data) => {
                let ssrc = data.packet.ssrc;

                if let Some(audio) = data.audio {
                    if !self.audio_data.lock().await.contains_key(&ssrc) {
                        self.audio_data.lock().await.insert(ssrc, vec![]);
                    }
                    let mut prev_data = self.audio_data.lock().await.get(&ssrc).unwrap().clone();
                    prev_data.append(&mut audio.clone());
                    self.audio_data.lock().await.insert(ssrc, prev_data);
                }
            }
            _ => {}
        }
        None
    }
}

pub async fn audio_receive(ctx: &Context, message: &Message) {
    let guild = message
        .guild_id
        .unwrap()
        .to_guild_cached(&ctx.cache)
        .unwrap();
    let channel_id = guild
        .voice_states
        .get(&message.author.id)
        .and_then(|state| state.channel_id);

    if channel_id.is_none() {
        return;
    }

    let manager = songbird::get(ctx).await.unwrap();

    let (handler_lock, conn_result) = manager.join(guild.id.0, channel_id.unwrap().0).await;

    let receiver = Receiver::new();

    if let Ok(_) = conn_result {
        let mut handler = handler_lock.lock().await;

        handler.add_global_event(CoreEvent::SpeakingStateUpdate.into(), receiver.clone());

        handler.add_global_event(CoreEvent::SpeakingUpdate.into(), receiver.clone());

        handler.add_global_event(CoreEvent::VoicePacket.into(), receiver.clone());

        handler.add_global_event(CoreEvent::RtcpPacket.into(), receiver.clone());

        handler.add_global_event(CoreEvent::ClientDisconnect.into(), receiver);
    }
}
