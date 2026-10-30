use std::{
    collections::HashMap,
    net::{Ipv4Addr, SocketAddr},
};

use common::mqtt::WEATHER_TOPIC;
use rumqttd::{Broker, Config, ConnectionSettings, RouterConfig, ServerSettings};

const BROKER_ID: usize = 0;
const HOST: Ipv4Addr = Ipv4Addr::UNSPECIFIED;
const PORT: u16 = 1883;
const SERVER_NAME: &str = "v4-1";

const MAX_CONNECTIONS: usize = 5;
const MAX_OUTGOING_PACKET_COUNT: u64 = 100;
const MAX_SEGMENT_SIZE: usize = 1024 * 1024;
const MAX_SEGMENT_COUNT: usize = 2;

const CONNECTION_TIMEOUT_MS: u16 = 60_000;
const MAX_PAYLOAD_SIZE: usize = 20 * 1024;
const MAX_INFLIGHT_COUNT: usize = 50;
const NEXT_CONNECTION_DELAY_MS: u64 = 1;

pub struct MqttService {
    broker: Broker,
}

impl MqttService {
    pub fn new(broker: Broker) -> Self {
        Self { broker }
    }

    pub fn default() -> anyhow::Result<Self> {
        let router_config = RouterConfig {
            max_connections: MAX_CONNECTIONS,
            max_outgoing_packet_count: MAX_OUTGOING_PACKET_COUNT,
            max_segment_size: MAX_SEGMENT_SIZE,
            max_segment_count: MAX_SEGMENT_COUNT,
            initialized_filters: Some(vec![WEATHER_TOPIC.to_string()]),
            ..Default::default()
        };

        let connections = ConnectionSettings {
            connection_timeout_ms: CONNECTION_TIMEOUT_MS,
            max_payload_size: MAX_PAYLOAD_SIZE,
            max_inflight_count: MAX_INFLIGHT_COUNT,
            auth: None,
            external_auth: None,
            dynamic_filters: false,
        };

        let server_settings = ServerSettings {
            name: SERVER_NAME.to_string(),
            listen: SocketAddr::from((HOST, PORT)),
            next_connection_delay_ms: NEXT_CONNECTION_DELAY_MS,
            connections,
            tls: None,
        };

        let mut v4: HashMap<String, ServerSettings> = HashMap::new();
        v4.insert(SERVER_NAME.to_string(), server_settings);

        let config = Config {
            id: BROKER_ID,
            router: router_config,
            v4: Some(v4),
            ..Default::default()
        };

        let broker = Broker::new(config);
        Ok(MqttService::new(broker))
    }
}
