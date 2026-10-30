use std::{collections::HashMap, net::SocketAddr, str::FromStr};

use rumqttd::{Broker, Config, ConnectionSettings, Notification, RouterConfig, ServerSettings};

pub struct MqttService {
    broker: Broker,
}

impl MqttService {
    pub fn new(broker: Broker) -> Self {
        Self { broker }
    }

    pub fn default() -> anyhow::Result<Self> {
        let router_config = RouterConfig {
            max_connections: 5,
            max_outgoing_packet_count: 100,
            max_segment_size: 1048576,
            max_segment_count: 2,
            initialized_filters: Some(vec!["weatherboard/weather".to_string()]),
            ..Default::default()
        };

        let connections = ConnectionSettings {
            connection_timeout_ms: 60000,
            max_payload_size: 20480,
            max_inflight_count: 50,
            auth: None,
            external_auth: None,
            dynamic_filters: false,
        };

        let server_settings = ServerSettings {
            name: "v4-1".to_string(),
            listen: SocketAddr::from(([0, 0, 0, 0], 1883)),
            next_connection_delay_ms: 1,
            connections,
            tls: None,
        };

        let mut v4: HashMap<String, ServerSettings> = HashMap::new();
        v4.insert("v4-1".to_string(), server_settings);

        let config = Config {
            id: 0,
            router: router_config,
            v4: Some(v4),
            ..Default::default()
        };

        let broker = Broker::new(config);
        Ok(MqttService::new(broker))
    }
}
