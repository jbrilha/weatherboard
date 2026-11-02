#pragma once

#include "esp_err.h"
#include "esp_hosted_bt_host_stack.h"

class WeatherMQTT;

class BLE {
  public:
    BLE();
    ~BLE();

    esp_err_t init(WeatherMQTT *weather);

  private:
    esp_hosted_bt_host_stack_cfg_t bt_config;
};
