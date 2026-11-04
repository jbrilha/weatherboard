#include "audio.hpp"
#include "esp_err.h"
#include "ethernet.hpp"
#include "freertos/FreeRTOS.h"
#include "freertos/projdefs.h"
#include "freertos/semphr.h"
#include "freertos/task.h"
#include <sys/lock.h>
#include <sys/param.h>
#include <unistd.h>

#include "ble.hpp"
#include "lvgl_port.hpp"
#include "system.hpp"
#include "ui.hpp"
#include "weather_mqtt.hpp"

extern "C" void app_main(void) {
    system_init();

    static Ethernet eth = Ethernet();
    eth.init();

    if (system_in_safe_mode()) {
        system_enter_safe_mode();
    }

    system_ota_start();

    static WeatherMQTT mqtt = WeatherMQTT();
    mqtt.init();

    static BLE ble = BLE();
    ble.init();

    static I2C i2c = I2C();
    i2c.init();

    static Audio audio = Audio(i2c);
    static LVGLPort lvgl_port = LVGLPort(i2c);
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

    if (audio.init() == ESP_OK) {
        audio.play_music();
        vTaskDelay(pdMS_TO_TICKS(5000));
        audio.stop_music();
    }
}
