#include "esp_err.h"
#include "mqtt_client.h"
#include "weatherboard.hpp"

class WeatherMQTT {
  public:
    WeatherMQTT();
    ~WeatherMQTT();

    esp_err_t init(void);
    size_t get_latest_payload(uint8_t *out, size_t len);

  private:
    esp_mqtt_client_config_t client_config;
    esp_mqtt_client_handle_t client_handle;
};
