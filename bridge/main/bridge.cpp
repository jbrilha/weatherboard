/*
 * SPDX-FileCopyrightText: 2023-2026 Espressif Systems (Shanghai) CO LTD
 *
 * SPDX-License-Identifier: CC0-1.0
 */

#include "driver/gpio.h"
#include "driver/i2c_master.h"
#include "esp_efuse.h"
#include "esp_err.h"
#include "esp_hosted_bt_host_stack.h"
#include "esp_lcd_jd9365_10_1.h"
#include "esp_lcd_mipi_dsi.h"
#include "esp_lcd_panel_ops.h"
#include "esp_lcd_touch.h"
#include "esp_lcd_touch_gt911.h"
#include "esp_ldo_regulator.h"
#include "esp_log.h"
#include "esp_timer.h"
#include "ethernet.hpp"
#include "freertos/FreeRTOS.h"
#include "freertos/projdefs.h"
#include "freertos/semphr.h"
#include "freertos/task.h"
#include "host/ble_hs.h"
#include "nimble/nimble_port.h"
#include "nimble/nimble_port_freertos.h"
#include "nvs_flash.h"
#include "sdkconfig.h"
#include <stdio.h>
#include <sys/lock.h>
#include <sys/param.h>
#include <unistd.h>

#include "lvgl_demo_ui.hpp"
#include "ui.hpp"

static constexpr char TAG[] = "bridge";

static void bridge_ble_on_sync(void) { ESP_LOGI(TAG, "BLE host synced"); }

static void bridge_ble_on_reset(int reason) {
    ESP_LOGW(TAG, "BLE host reset (reason=%d)", reason);
}

static void bridge_ble_host_task(void *arg) {
    nimble_port_run();
    nimble_port_freertos_deinit();
}

static void bridge_ble_init(void) {
    esp_err_t err = nvs_flash_init();
    if (err == ESP_ERR_NVS_NO_FREE_PAGES ||
        err == ESP_ERR_NVS_NEW_VERSION_FOUND) {
        ESP_ERROR_CHECK(nvs_flash_erase());
        err = nvs_flash_init();
    }
    ESP_ERROR_CHECK(err);

    esp_hosted_bt_host_stack_cfg_t bt_cfg =
        ESP_HOSTED_BT_HOST_STACK_CONFIG_DEFAULT();
    ESP_ERROR_CHECK(esp_hosted_bt_host_stack_setup(&bt_cfg));
    ESP_ERROR_CHECK(nimble_port_init());
    ble_hs_cfg.sync_cb = bridge_ble_on_sync;
    ble_hs_cfg.reset_cb = bridge_ble_on_reset;
    nimble_port_freertos_init(bridge_ble_host_task);
}

extern "C" void app_main(void) {
    bridge_ble_init();

    static Ethernet eth = Ethernet();
    eth.init();

    static UI ui = UI();
    if (ui.init() == ESP_OK) {
        ui.draw_example();
    }
}
