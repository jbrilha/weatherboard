#include "weather_mqtt.hpp"
#include "esp_err.h"
#include "esp_log.h"
#include "mqtt_client.h"
#include "weatherboard.hpp"
#include <stdio.h>

static constexpr char TAG[] = "WeatherMQTT";

static wb::weather_t latest_weather;
static uint8_t latest_payload[WB_MAX_PAYLOAD_LEN];
static size_t latest_payload_len = 0;

static void mqtt_event_handler(void *handler_args, esp_event_base_t base,
                               int32_t event_id, void *event_data) {
    ESP_LOGD(TAG, "Event dispatched from event loop base=%s, event_id=%" PRIi32,
             base, event_id);
    esp_mqtt_event_handle_t event =
        static_cast<esp_mqtt_event_handle_t>(event_data);
    esp_mqtt_client_handle_t client = event->client;
    int msg_id;
    switch ((esp_mqtt_event_id_t)event_id) {
    case MQTT_EVENT_CONNECTED:
        ESP_LOGI(TAG, "MQTT_EVENT_CONNECTED");

        msg_id = esp_mqtt_client_subscribe(
            client, wb::topic_str(wb::topic_t::WEATHER), 0);
        ESP_LOGI(TAG, "sent weather subscribe successful, msg_id=%d", msg_id);

        msg_id = esp_mqtt_client_subscribe(client, "topic/qos0", 0);
        ESP_LOGI(TAG, "sent subscribe successful, msg_id=%d", msg_id);

        msg_id = esp_mqtt_client_subscribe(client, "topic/qos1", 1);
        ESP_LOGI(TAG, "sent subscribe successful, msg_id=%d", msg_id);

        msg_id = esp_mqtt_client_unsubscribe(client, "topic/qos1");
        ESP_LOGI(TAG, "sent unsubscribe successful, msg_id=%d", msg_id);
        break;
    case MQTT_EVENT_DISCONNECTED:
        ESP_LOGI(TAG, "MQTT_EVENT_DISCONNECTED");
        break;

    case MQTT_EVENT_SUBSCRIBED:
        ESP_LOGI(TAG, "MQTT_EVENT_SUBSCRIBED, msg_id=%d, return code=0x%02x ",
                 event->msg_id, (uint8_t)*event->data);
        msg_id = esp_mqtt_client_publish(client, "topic/qos0", "data", 0, 0, 0);
        ESP_LOGI(TAG, "sent publish successful, msg_id=%d", msg_id);
        break;
    case MQTT_EVENT_UNSUBSCRIBED:
        ESP_LOGI(TAG, "MQTT_EVENT_UNSUBSCRIBED, msg_id=%d", event->msg_id);
        break;
    case MQTT_EVENT_PUBLISHED:
        ESP_LOGI(TAG, "MQTT_EVENT_PUBLISHED, msg_id=%d", event->msg_id);
        break;
    case MQTT_EVENT_DATA:
        ESP_LOGI(TAG, "MQTT_EVENT_DATA");
        printf("TOPIC=%.*s\r\n", event->topic_len, event->topic);
        printf("DATA=%.*s\r\n", event->data_len, event->data);
        ESP_LOG_BUFFER_HEX(TAG, event->data, event->data_len);
        if (wb::decode_weather(reinterpret_cast<const uint8_t *>(event->data),
                               event->data_len, &latest_weather) &&
            event->data_len <= (int)sizeof latest_payload) {
            ESP_LOGI(TAG, "weather: temp=%d C, humidity=%u%%",
                     latest_weather.temp_c, latest_weather.humidity_pct);
            memcpy(latest_payload, event->data, event->data_len);
            latest_payload_len = event->data_len;
        }
        break;
    case MQTT_EVENT_ERROR:
        ESP_LOGI(TAG, "MQTT_EVENT_ERROR");
        if (event->error_handle->error_type == MQTT_ERROR_TYPE_TCP_TRANSPORT) {
            ESP_LOGI(TAG, "Last error code reported from esp-tls: 0x%x",
                     event->error_handle->esp_tls_last_esp_err);
            ESP_LOGI(TAG, "Last tls stack error number: 0x%x",
                     event->error_handle->esp_tls_stack_err);
            ESP_LOGI(TAG, "Last captured errno : %d (%s)",
                     event->error_handle->esp_transport_sock_errno,
                     strerror(event->error_handle->esp_transport_sock_errno));
        } else if (event->error_handle->error_type ==
                   MQTT_ERROR_TYPE_CONNECTION_REFUSED) {
            ESP_LOGI(TAG, "Connection refused error: 0x%x",
                     event->error_handle->connect_return_code);
        } else {
            ESP_LOGW(TAG, "Unknown error type: 0x%x",
                     event->error_handle->error_type);
        }
        break;
    default:
        ESP_LOGI(TAG, "Other event id:%d", event->event_id);
        break;
    }
}

WeatherMQTT::WeatherMQTT() : client_config{}, client_handle(nullptr) {

    client_config.broker = {.address = {.uri = "mqtt://pi5.local:1883"}};
    client_handle = esp_mqtt_client_init(&client_config);
}

WeatherMQTT::~WeatherMQTT() {}

esp_err_t WeatherMQTT::init() {
    ESP_ERROR_CHECK(esp_mqtt_client_register_event(
        client_handle, static_cast<esp_mqtt_event_id_t>(ESP_EVENT_ANY_ID),
        mqtt_event_handler, nullptr));

    ESP_ERROR_CHECK(esp_mqtt_client_start(client_handle));

    return ESP_OK;
}

size_t WeatherMQTT::get_latest_payload(uint8_t *out, size_t len) {
    if (latest_payload_len == 0 || latest_payload_len > len) {
        return 0;
    }

    memcpy(out, latest_payload, latest_payload_len);
    return latest_payload_len;
}
