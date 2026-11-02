/*
 * SPDX-FileCopyrightText: 2023-2026 Espressif Systems (Shanghai) CO LTD
 *
 * SPDX-License-Identifier: CC0-1.0
 */

#include "esp_err.h"
#include "esp_event.h"
#include "esp_log.h"
#include "esp_netif.h"
#include "ethernet.hpp"
#include "freertos/FreeRTOS.h"
#include "freertos/projdefs.h"
#include "freertos/semphr.h"
#include "freertos/task.h"
#include "host/ble_hs.h"
#include "mdns.h"
#include "nimble/nimble_port.h"
#include "nimble/nimble_port_freertos.h"
#include "nvs_flash.h"
#include "sdkconfig.h"
#include <stdio.h>
#include <sys/lock.h>
#include <sys/param.h>
#include <unistd.h>

#include "ble.hpp"
#include "lvgl_port.hpp"
#include "ui.hpp"
#include "weather_mqtt.hpp"

static constexpr char TAG[] = "bridge";

static void system_init() {
    esp_err_t err = nvs_flash_init();
    if (err == ESP_ERR_NVS_NO_FREE_PAGES ||
        err == ESP_ERR_NVS_NEW_VERSION_FOUND) {
        ESP_ERROR_CHECK(nvs_flash_erase());
        err = nvs_flash_init();
    }
    ESP_ERROR_CHECK(err);

    ESP_ERROR_CHECK(esp_netif_init());
    ESP_ERROR_CHECK(esp_event_loop_create_default());

    ESP_ERROR_CHECK(mdns_init());
    ESP_ERROR_CHECK(mdns_hostname_set(TAG));
}

extern "C" void app_main(void) {
    system_init();

    static BLE ble = BLE();
    ble.init();

    static Ethernet eth = Ethernet();
    eth.init();

    static WeatherMQTT mqtt = WeatherMQTT();
    mqtt.init();

    static LVGLPort lvgl_port = LVGLPort();
    if (lvgl_port.init() == ESP_OK) {
        auto brightness_func = [](uint8_t v) { lvgl_port.set_brightness(v); };
        ui::demo::callbacks demo_cbs = {
            .set_brightness = brightness_func,
        };

        lv_lock();
        ui::init(lvgl_port.get_lv_display());
        ui::demo::set_callbacks(demo_cbs);
        ui::demo::show();
        lv_unlock();
    }
}
