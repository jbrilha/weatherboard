#pragma once

#include "esp_err.h"
#include <stdint.h>

typedef enum : uint8_t {
    OTA_STATE_IDLE = 0,
    OTA_STATE_CHECKING,
    OTA_STATE_DOWNLOADING,
    OTA_STATE_FAILED,
} bridge_ota_state_t;

esp_err_t bridge_ota_init(void);

bridge_ota_state_t bridge_ota_get_state(void);
const char *bridge_ota_state_str(bridge_ota_state_t s);
