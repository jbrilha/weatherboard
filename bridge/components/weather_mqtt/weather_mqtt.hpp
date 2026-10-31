#include "esp_err.h"
#include "mqtt_client.h"

class WeatherMQTT {
  public:
    WeatherMQTT();
    ~WeatherMQTT();

    esp_err_t init(void);

  private:
    esp_mqtt_client_config_t client_config;
    esp_mqtt_client_handle_t client_handle;
};
