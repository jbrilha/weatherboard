#include "esp_err.h"
#include "esp_event_base.h"
#include "mqtt_client.h"
#include "weatherboard.hpp"

#include "esp_event.h"
#include <cstdint>

ESP_EVENT_DECLARE_BASE(BRIDGE_MQTT_EVENTS);

enum mqtt_event : int32_t {
    WEATHER_RECEIVED_EVENT,
};

class WeatherMQTT {
  public:
    WeatherMQTT();
    ~WeatherMQTT();

    esp_err_t init(void);

  private:
    esp_mqtt_client_config_t client_config;
    esp_mqtt_client_handle_t client_handle;
};

esp_err_t get_latest_weather(wb::weather_t *out);
esp_err_t get_latest_weather_payload(uint8_t *buf, uint8_t cap, uint8_t *len);
