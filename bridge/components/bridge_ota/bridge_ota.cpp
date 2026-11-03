#include "bridge_ota.hpp"
#include "esp_app_desc.h"
#include "esp_http_client.h"
#include "esp_https_ota.h"
#include "esp_log.h"
#include "esp_ota_ops.h"
#include "esp_system.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "sdkconfig.h"
#include <atomic>
#include <string.h>

namespace {

constexpr char TAG[] = "bridge_ota";

std::atomic<bridge_ota_state_t> state{OTA_STATE_IDLE};
TaskHandle_t task = nullptr;

bool same_image(const esp_app_desc_t &a, const esp_app_desc_t &b) {
    return memcmp(a.app_elf_sha256, b.app_elf_sha256,
                  sizeof(a.app_elf_sha256)) == 0;
}

bool should_install(const esp_app_desc_t &incoming) {
    if (same_image(incoming, *esp_app_get_description())) {
        return false;
    }

    const esp_partition_t *invalid = esp_ota_get_last_invalid_partition();
    esp_app_desc_t rejected;
    if ((invalid != nullptr) &&
        esp_ota_get_partition_description(invalid, &rejected) == ESP_OK &&
        same_image(incoming, rejected)) {
        ESP_LOGW(TAG, "%s was rolled back before, skipping", incoming.version);
        return false;
    }

    return true;
}

esp_err_t check_and_install() {
    esp_http_client_config_t http = {};
    http.url = CONFIG_BRIDGE_OTA_URL;
    http.keep_alive_enable = true;

    esp_https_ota_config_t cfg = {};
    cfg.http_config = &http;

    esp_https_ota_handle_t handle = nullptr;
    esp_err_t err = esp_https_ota_begin(&cfg, &handle);
    if (err != ESP_OK) {
        return err;
    }

    esp_app_desc_t incoming;
    err = esp_https_ota_get_img_desc(handle, &incoming);
    if (err != ESP_OK || !should_install(incoming)) {
        esp_https_ota_abort(handle);
        return err;
    }

    ESP_LOGW(TAG, "installing %s over %s", incoming.version,
             esp_app_get_description()->version);
    state = OTA_STATE_DOWNLOADING;

    do {
        err = esp_https_ota_perform(handle);
    } while (err == ESP_ERR_HTTPS_OTA_IN_PROGRESS);

    if (err != ESP_OK) {
        esp_https_ota_abort(handle);
        return err;
    }

    err = esp_https_ota_finish(handle);
    if (err != ESP_OK) {
        return err;
    }

    ESP_LOGW(TAG, "update applied, restarting");
    esp_restart();
}

void ota_task(void * /*args*/) {
    vTaskDelay(pdMS_TO_TICKS(CONFIG_BRIDGE_OTA_FIRST_CHECK_S * 1000));

    while (true) {
        state = OTA_STATE_CHECKING;
        esp_err_t err = check_and_install();
        if (err == ESP_OK) {
            state = OTA_STATE_IDLE;
        } else {
            state = OTA_STATE_FAILED;
            ESP_LOGE(TAG, "check against %s failed (%s)", CONFIG_BRIDGE_OTA_URL,
                     esp_err_to_name(err));
        }
        vTaskDelay(pdMS_TO_TICKS(CONFIG_BRIDGE_OTA_POLL_S * 1000));
    }
}

} // namespace

esp_err_t bridge_ota_init(void) {
    if (task != nullptr) {
        return ESP_ERR_INVALID_STATE;
    }
    if (xTaskCreate(ota_task, "bridge_ota", 8192, nullptr, 5, &task) !=
        pdPASS) {
        return ESP_ERR_NO_MEM;
    }
    return ESP_OK;
}

bridge_ota_state_t bridge_ota_get_state(void) { return state; }

const char *bridge_ota_state_str(bridge_ota_state_t s) {
    switch (s) {
    case OTA_STATE_IDLE:
        return "idle";
    case OTA_STATE_CHECKING:
        return "checking";
    case OTA_STATE_DOWNLOADING:
        return "downloading";
    case OTA_STATE_FAILED:
        return "failed";
    default:
        return "unknown";
    }
}
