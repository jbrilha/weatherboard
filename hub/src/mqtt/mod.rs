use std::time::Duration;

use common::ipc::{HUB_SOCKET, ToHub, ToServer};
use common::mqtt::WEATHER_TOPIC;
use common::weather::WeatherPayload;
use common::{MAX_PAYLOAD_LEN, Message, ToBoard};
use futures::SinkExt;
use futures::stream::StreamExt;
use rumqttc::{AsyncClient, Event, EventLoop, MqttOptions, Packet, QoS};
use tokio::net::UnixStream;
use tokio::sync::watch;
use tokio::time;
use tokio_util::bytes::{Bytes, BytesMut};
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tracing::{info, warn};

const RETRY_TIMEOUT: u64 = 30;
const MQTT_CLIENT_ID: &str = "weatherhub";

type FramedSock = Framed<UnixStream, LengthDelimitedCodec>;

pub struct MqttService {
    client: AsyncClient,
    event_loop: EventLoop,
    sender: watch::Sender<Option<WeatherPayload>>,
}

impl MqttService {
    pub fn new(
        client: AsyncClient,
        event_loop: EventLoop,
        sender: watch::Sender<Option<WeatherPayload>>,
    ) -> Self {
        Self {
            client,
            event_loop,
            sender,
        }
    }

    pub async fn default(sender: watch::Sender<Option<WeatherPayload>>) -> Self {
        let mqtt_opts = MqttOptions::new(MQTT_CLIENT_ID, "localhost", 1883);

        let (client, event_loop) = AsyncClient::new(mqtt_opts, 10);
        MqttService::new(client, event_loop, sender)
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        self.client
            .subscribe(WEATHER_TOPIC, QoS::AtMostOnce)
            .await?;
        loop {
            let event = self.event_loop.poll().await?;
            if let Event::Incoming(Packet::Publish(publish)) = event {
                match ToBoard::decode(&publish.payload) {
                    Ok(ToBoard::Weather(payload)) => {
                        info!("mqtt payload received: {payload:?}");
                        self.sender.send_if_modified(|current| {
                            let changed = *current != Some(payload);
                            *current = Some(payload);
                            changed
                        });
                    }
                    Ok(ToBoard::Config(config)) => {
                        warn!("config not handled yet: {config:?}")
                    }
                    Err(e) => warn!("rejected publish: {e:?}"),
                }
            }
        }
    }
}
