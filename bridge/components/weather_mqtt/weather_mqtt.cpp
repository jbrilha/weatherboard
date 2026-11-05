#include "weather_mqtt.hpp"
#include "esp_check.h"
#include "esp_err.h"
#include "esp_event.h"
#include "esp_event_base.h"
#include "esp_log.h"
#include "mqtt_client.h"
#include "weatherboard.hpp"
#include <cstdint>
#include <stdio.h>

static constexpr char TAG[] = "WeatherMQTT";

ESP_EVENT_DEFINE_BASE(BRIDGE_MQTT_EVENTS);

namespace {
uint8_t latest_payload[WB_MAX_PAYLOAD_LEN];
uint8_t latest_len = 0;
portMUX_TYPE weather_lock = portMUX_INITIALIZER_UNLOCKED;

void mqtt_event_handler(void *handler_args, esp_event_base_t base,
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
        // TODO per-topic data handling
        {
            wb::weather_t weather;
            const auto *data = reinterpret_cast<const uint8_t *>(event->data);
            if (event->data_len <= WB_MAX_PAYLOAD_LEN &&
                wb::decode_weather(data, event->data_len, &weather)) {
                ESP_LOGI(TAG, "weather: temp=%d C, humidity=%u%%",
                         weather.temp_c, weather.humidity_pct);
                taskENTER_CRITICAL(&weather_lock);
                memcpy(latest_payload, data, event->data_len);
                latest_len = event->data_len;
                taskEXIT_CRITICAL(&weather_lock);
                esp_event_post(BRIDGE_MQTT_EVENTS, WEATHER_RECEIVED_EVENT,
                               nullptr, 0, 0);
            }
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

} // namespace

WeatherMQTT::WeatherMQTT() : client_config{}, client_handle(nullptr) {

    client_config.broker = {.address = {.uri = "mqtt://pi5.local:1883"}};
    client_handle = esp_mqtt_client_init(&client_config);
}

WeatherMQTT::~WeatherMQTT() {}

esp_err_t WeatherMQTT::init() {
    ESP_RETURN_ON_ERROR(
        esp_mqtt_client_register_event(
            client_handle, static_cast<esp_mqtt_event_id_t>(ESP_EVENT_ANY_ID),
            mqtt_event_handler, nullptr),
        TAG, "register MQTT event handler failed");

    ESP_RETURN_ON_ERROR(esp_mqtt_client_start(client_handle), TAG,
                        "start MQTT client failed");

    return ESP_OK;
}

esp_err_t get_latest_weather(wb::weather_t *out) {
    if (out == nullptr) {
        return ESP_ERR_INVALID_ARG;
    }

    uint8_t buf[WB_MAX_PAYLOAD_LEN];
    uint8_t len;
    esp_err_t ret = get_latest_weather_payload(buf, sizeof buf, &len);
    if (ret != ESP_OK) {
        return ret;
    }

    return wb::decode_weather(buf, len, out) ? ESP_OK : ESP_ERR_INVALID_STATE;
}

esp_err_t get_latest_weather_payload(uint8_t *buf, uint8_t cap, uint8_t *len) {
    if (buf == nullptr || len == nullptr) {
        return ESP_ERR_INVALID_ARG;
    }

    esp_err_t err = ESP_OK;

    taskENTER_CRITICAL(&weather_lock);
    if (latest_len == 0) {
        err = ESP_ERR_NOT_FOUND;
    } else if (cap < latest_len) {
        err = ESP_ERR_INVALID_SIZE;
    } else {
        memcpy(buf, latest_payload, latest_len);
        *len = latest_len;
    }
    taskEXIT_CRITICAL(&weather_lock);

    return err;
}
