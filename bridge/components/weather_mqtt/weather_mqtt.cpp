#include "weather_mqtt.hpp"
#include "mqtt_client.h"
#include <stdio.h>

void func(void) {

    esp_mqtt_client_config_t mqtt_cfg = {
        .broker = {.address = {.uri = "mqtt://test.mosquitto.org"}},
    };
}
