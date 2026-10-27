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
#include "freertos/FreeRTOS.h"
#include "freertos/semphr.h"
#include "freertos/task.h"
#include "host/ble_hs.h"
#include "include/lvgl/core/lv_obj_style.h"
#include "lvgl.h"
#include "nimble/nimble_port.h"
#include "nimble/nimble_port_freertos.h"
#include "nvs_flash.h"
#include "sdkconfig.h"
#include <stdio.h>
#include <sys/lock.h>
#include <sys/param.h>
#include <unistd.h>

static const char *TAG = "example";

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
//////////////////// Please update the following configuration according to your
/// LCD Spec //////////////////////////////
////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

// Refresh Rate = 80000000/(20+20+40+800)/(4+10+30+1280) = 68Hz
#define EXAMPLE_MIPI_DSI_DPI_CLK_MHZ 80
#define EXAMPLE_MIPI_DSI_LCD_H_RES 800
#define EXAMPLE_MIPI_DSI_LCD_V_RES 1280
#define EXAMPLE_MIPI_DSI_LCD_HSYNC 20
#define EXAMPLE_MIPI_DSI_LCD_HBP 20
#define EXAMPLE_MIPI_DSI_LCD_HFP 40
#define EXAMPLE_MIPI_DSI_LCD_VSYNC 4
#define EXAMPLE_MIPI_DSI_LCD_VBP 10
#define EXAMPLE_MIPI_DSI_LCD_VFP 30

#define EXAMPLE_MIPI_DSI_LANE_NUM 2             // 2 data lanes
#define EXAMPLE_MIPI_DSI_LANE_BITRATE_MBPS 1500 // 1.5Gbps

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
//////////////////// Please update the following configuration according to your
/// Board Design //////////////////////////
////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

// The "VDD_MIPI_DPHY" should be supplied with 2.5V, it can source from the
// internal LDO regulator or from external LDO chip
#define EXAMPLE_MIPI_DSI_PHY_PWR_LDO_CHAN                                      \
    3 // LDO_VO3 is connected to VDD_MIPI_DPHY
#define EXAMPLE_MIPI_DSI_PHY_PWR_LDO_VOLTAGE_MV 2500
#define EXAMPLE_LCD_BK_LIGHT_ON_LEVEL 1
#define EXAMPLE_LCD_BK_LIGHT_OFF_LEVEL !EXAMPLE_LCD_BK_LIGHT_ON_LEVEL
#define EXAMPLE_PIN_NUM_BK_LIGHT -1
#define EXAMPLE_PIN_NUM_LCD_RST -1

#define EXAMPLE_PIN_NUM_TOUCH_SDA 7
#define EXAMPLE_PIN_NUM_TOUCH_SCL 8
#define EXAMPLE_BK_LIGHT_I2C_ADDR 0x45
#define EXAMPLE_BK_LIGHT_I2C_REG 0x96

#if CONFIG_EXAMPLE_MONITOR_REFRESH_BY_GPIO
#define EXAMPLE_PIN_NUM_REFRESH_MONITOR                                        \
    20 // Monitor the Refresh Rate by toggling the GPIO
#endif

#define ALIGN_UP(num, align) (((num) + ((align) - 1)) & ~((align) - 1))
#define ALIGN_DOWN(num, align) ((num) & ~((align) - 1))

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
//////////////////// Please update the following configuration according to your
/// Application ///////////////////////////
////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#define EXAMPLE_LVGL_DRAW_BUF_LINES                                            \
    (EXAMPLE_MIPI_DSI_LCD_V_RES /                                              \
     32) // number of display lines in each draw buffer
#define EXAMPLE_LVGL_TICK_PERIOD_MS 2
#define EXAMPLE_LVGL_TASK_STACK_SIZE (4 * 1024)
#define EXAMPLE_LVGL_TASK_PRIORITY 2
#define EXAMPLE_LVGL_TASK_MAX_DELAY_MS 500
#define EXAMPLE_LVGL_TASK_MIN_DELAY_MS 1000 / CONFIG_FREERTOS_HZ

// LVGL library is not thread-safe, this example will call LVGL APIs from
// different tasks, so use a mutex to protect it
static _lock_t lvgl_api_lock;

extern void example_lvgl_demo_ui(lv_display_t *disp);

#if CONFIG_EXAMPLE_USE_DMA2D_COPY_FRAME
void example_rounder_flush_area_cb(lv_event_t *event) {
    lv_area_t *area = lv_event_get_invalidated_area(event);
    area->x1 = ALIGN_DOWN(area->x1, 16);
    area->x2 = ALIGN_UP(area->x2, 16) - 1;
}
#endif

static void example_lvgl_flush_cb(lv_display_t *disp, const lv_area_t *area,
                                  uint8_t *px_map) {
    esp_lcd_panel_handle_t panel_handle = static_cast<esp_lcd_panel_handle_t>(lv_display_get_user_data(disp));
    int offsetx1 = area->x1;
    int offsetx2 = area->x2;
    int offsety1 = area->y1;
    int offsety2 = area->y2;
    // pass the draw buffer to the driver
    esp_lcd_panel_draw_bitmap(panel_handle, offsetx1, offsety1, offsetx2 + 1,
                              offsety2 + 1, px_map);
}

static void example_lvgl_touch_cb(lv_indev_t *indev, lv_indev_data_t *data) {
    esp_lcd_touch_handle_t tp = static_cast<esp_lcd_touch_handle_t>(lv_indev_get_user_data(indev));
    esp_lcd_touch_point_data_t point;
    uint8_t cnt = 0;
    esp_lcd_touch_read_data(tp);
    esp_lcd_touch_get_data(tp, &point, &cnt, 1);
    if (cnt > 0) {
        data->point.x = point.x;
        data->point.y = point.y;
        data->state = LV_INDEV_STATE_PRESSED;
    } else {
        data->state = LV_INDEV_STATE_RELEASED;
    }
}

static i2c_master_dev_handle_t bk_light_dev;

static void example_bsp_set_lcd_brightness(uint8_t level) {
    uint8_t buf[2] = {EXAMPLE_BK_LIGHT_I2C_REG, level};
    ESP_ERROR_CHECK_WITHOUT_ABORT(
        i2c_master_transmit(bk_light_dev, buf, sizeof(buf), 50));
}

static void example_brightness_slider_cb(lv_event_t *e) {
    example_bsp_set_lcd_brightness(
        lv_slider_get_value(lv_event_get_target_obj(e)));
}

static void example_fill_test_cb(lv_event_t *e) {
    lv_obj_t *fill = static_cast<lv_obj_t *>(lv_event_get_user_data(e));
    if (lv_obj_has_flag(fill, LV_OBJ_FLAG_HIDDEN)) {
        lv_obj_set_style_bg_color(fill, lv_color_white(), 0);
        lv_obj_remove_flag(fill, LV_OBJ_FLAG_HIDDEN);
    } else if (lv_color_eq(lv_obj_get_style_bg_color(fill, LV_PART_MAIN),
                           lv_color_white())) {
        lv_obj_set_style_bg_color(fill, lv_color_black(), 0);
    } else {
        lv_obj_add_flag(fill, LV_OBJ_FLAG_HIDDEN);
    }
}

static esp_lcd_touch_handle_t example_bsp_init_touch_and_brightness(void) {
    i2c_master_bus_handle_t i2c_bus;
    i2c_master_bus_config_t bus_cfg = {
        .i2c_port = -1,
        .sda_io_num = (gpio_num_t) EXAMPLE_PIN_NUM_TOUCH_SDA,
        .scl_io_num = (gpio_num_t) EXAMPLE_PIN_NUM_TOUCH_SCL,
        .clk_source = I2C_CLK_SRC_DEFAULT,
        .glitch_ignore_cnt = 7,
        .flags = {.enable_internal_pullup = true},
    };
    ESP_ERROR_CHECK(i2c_new_master_bus(&bus_cfg, &i2c_bus));

    i2c_device_config_t bk_light_cfg = {
        .device_address = EXAMPLE_BK_LIGHT_I2C_ADDR,
        .scl_speed_hz = 100000,
    };
    ESP_ERROR_CHECK(
        i2c_master_bus_add_device(i2c_bus, &bk_light_cfg, &bk_light_dev));
    example_bsp_set_lcd_brightness(255);

    esp_lcd_panel_io_i2c_config_t io_cfg = ESP_LCD_TOUCH_IO_I2C_GT911_CONFIG();
    if (i2c_master_probe(i2c_bus, ESP_LCD_TOUCH_IO_I2C_GT911_ADDRESS, 50) !=
        ESP_OK) {
        io_cfg.dev_addr = ESP_LCD_TOUCH_IO_I2C_GT911_ADDRESS_BACKUP;
    }
    ESP_LOGI(TAG, "Touch controller at 0x%02X", (unsigned)io_cfg.dev_addr);
    esp_lcd_panel_io_handle_t tp_io;
    ESP_ERROR_CHECK(esp_lcd_new_panel_io_i2c(i2c_bus, &io_cfg, &tp_io));

    esp_lcd_touch_config_t tp_cfg = {
        .x_max = EXAMPLE_MIPI_DSI_LCD_H_RES,
        .y_max = EXAMPLE_MIPI_DSI_LCD_V_RES,
        .rst_gpio_num = GPIO_NUM_NC,
        .int_gpio_num = GPIO_NUM_NC,
        .flags =
            {
                .swap_xy = 0,
                .mirror_x = 0,
                .mirror_y = 0,
            },
    };
    esp_lcd_touch_handle_t tp;
    ESP_ERROR_CHECK(esp_lcd_touch_new_i2c_gt911(tp_io, &tp_cfg, &tp));
    return tp;
}

static void example_increase_lvgl_tick(void *arg) {
    /* Tell LVGL how many milliseconds has elapsed */
    lv_tick_inc(EXAMPLE_LVGL_TICK_PERIOD_MS);
}

static void example_lvgl_port_task(void *arg) {
    ESP_LOGI(TAG, "Starting LVGL task");
    uint32_t time_till_next_ms = 0;
    while (1) {
        _lock_acquire(&lvgl_api_lock);
        time_till_next_ms = lv_timer_handler();
        _lock_release(&lvgl_api_lock);
        // in case of triggering a task watch dog time out
        time_till_next_ms =
            MAX(time_till_next_ms, EXAMPLE_LVGL_TASK_MIN_DELAY_MS);
        // in case of lvgl display not ready yet
        time_till_next_ms =
            MIN(time_till_next_ms, EXAMPLE_LVGL_TASK_MAX_DELAY_MS);
        usleep(1000 * time_till_next_ms);
    }
}

static bool
example_notify_lvgl_flush_ready(esp_lcd_panel_handle_t panel,
                                esp_lcd_dpi_panel_event_data_t *edata,
                                void *user_ctx) {
    lv_display_t *disp = (lv_display_t *)user_ctx;
    lv_display_flush_ready(disp);
    return false;
}

#if CONFIG_EXAMPLE_MONITOR_REFRESH_BY_GPIO
static bool example_monitor_refresh_rate(esp_lcd_panel_handle_t panel,
                                         esp_lcd_dpi_panel_event_data_t *edata,
                                         void *user_ctx) {
    static int io_level = 0;
    // please note, the real refresh rate should be 2*frequency of this GPIO
    // toggling
    gpio_set_level(EXAMPLE_PIN_NUM_REFRESH_MONITOR, io_level);
    io_level = !io_level;
    return false;
}
#endif

static void example_bsp_enable_dsi_phy_power(void) {
    // Turn on the power for MIPI DSI PHY, so it can go from "No Power" state to
    // "Shutdown" state
    esp_ldo_channel_handle_t ldo_mipi_phy = NULL;
#ifdef EXAMPLE_MIPI_DSI_PHY_PWR_LDO_CHAN
    esp_ldo_channel_config_t ldo_mipi_phy_config = {
        .chan_id = EXAMPLE_MIPI_DSI_PHY_PWR_LDO_CHAN,
        .voltage_mv = EXAMPLE_MIPI_DSI_PHY_PWR_LDO_VOLTAGE_MV,
    };
    ESP_ERROR_CHECK(
        esp_ldo_acquire_channel(&ldo_mipi_phy_config, &ldo_mipi_phy));
    ESP_LOGI(TAG, "MIPI DSI PHY Powered on");
#endif
}

static void example_bsp_init_lcd_backlight(void) {
#if EXAMPLE_PIN_NUM_BK_LIGHT >= 0
    gpio_config_t bk_gpio_config = {.mode = GPIO_MODE_OUTPUT,
                                    .pin_bit_mask =
                                        1ULL << EXAMPLE_PIN_NUM_BK_LIGHT};
    ESP_ERROR_CHECK(gpio_config(&bk_gpio_config));
#endif
}

static void example_bsp_set_lcd_backlight(uint32_t level) {
#if EXAMPLE_PIN_NUM_BK_LIGHT >= 0
    gpio_set_level(EXAMPLE_PIN_NUM_BK_LIGHT, level);
#endif
}

#if CONFIG_EXAMPLE_MONITOR_REFRESH_BY_GPIO
static void example_bsp_init_refresh_monitor_io(void) {
    gpio_config_t monitor_io_conf = {
        .mode = GPIO_MODE_OUTPUT,
        .pin_bit_mask = 1ULL << EXAMPLE_PIN_NUM_REFRESH_MONITOR,
    };
    ESP_ERROR_CHECK(gpio_config(&monitor_io_conf));
}
#endif

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
#if CONFIG_EXAMPLE_MONITOR_REFRESH_BY_GPIO
    example_bsp_init_refresh_monitor_io();
#endif

    example_bsp_enable_dsi_phy_power();
    example_bsp_init_lcd_backlight();
    example_bsp_set_lcd_backlight(EXAMPLE_LCD_BK_LIGHT_OFF_LEVEL);

    // create MIPI DSI bus first, it will initialize the DSI PHY as well
    esp_lcd_dsi_bus_handle_t mipi_dsi_bus;
    esp_lcd_dsi_bus_config_t bus_config = {
        .bus_id = 0,
        .num_data_lanes = EXAMPLE_MIPI_DSI_LANE_NUM,
        .lane_bit_rate_mbps = EXAMPLE_MIPI_DSI_LANE_BITRATE_MBPS,
    };
    ESP_ERROR_CHECK(esp_lcd_new_dsi_bus(&bus_config, &mipi_dsi_bus));

    ESP_LOGI(TAG, "Install MIPI DSI LCD control IO");
    esp_lcd_panel_io_handle_t mipi_dbi_io;
    // we use DBI interface to send LCD commands and parameters
    esp_lcd_dbi_io_config_t dbi_config = {
        .virtual_channel = 0,
        .lcd_cmd_bits = 8,   // according to the LCD spec
        .lcd_param_bits = 8, // according to the LCD spec
    };
    ESP_ERROR_CHECK(
        esp_lcd_new_panel_io_dbi(mipi_dsi_bus, &dbi_config, &mipi_dbi_io));

    ESP_LOGI(TAG, "Install MIPI DSI LCD data panel");
    esp_lcd_panel_handle_t mipi_dpi_panel = NULL;
    esp_lcd_dpi_panel_config_t dpi_config = {
        .virtual_channel = 0,
        .dpi_clk_src = MIPI_DSI_DPI_CLK_SRC_DEFAULT,
        .dpi_clock_freq_mhz = EXAMPLE_MIPI_DSI_DPI_CLK_MHZ,
        .in_color_format = LCD_COLOR_FMT_RGB888,
        .video_timing =
            {
                .h_size = EXAMPLE_MIPI_DSI_LCD_H_RES,
                .v_size = EXAMPLE_MIPI_DSI_LCD_V_RES,
                .hsync_pulse_width = EXAMPLE_MIPI_DSI_LCD_HSYNC,
                .hsync_back_porch = EXAMPLE_MIPI_DSI_LCD_HBP,
                .hsync_front_porch = EXAMPLE_MIPI_DSI_LCD_HFP,
                .vsync_pulse_width = EXAMPLE_MIPI_DSI_LCD_VSYNC,
                .vsync_back_porch = EXAMPLE_MIPI_DSI_LCD_VBP,
                .vsync_front_porch = EXAMPLE_MIPI_DSI_LCD_VFP,
            },
    };

    jd9365_vendor_config_t vendor_config = {
        .mipi_config =
            {
                .dsi_bus = mipi_dsi_bus,
                .dpi_config = &dpi_config,
                .lane_num = EXAMPLE_MIPI_DSI_LANE_NUM,
            },
        .flags =
            {
                .use_mipi_interface = 1,
            },
    };
    esp_lcd_panel_dev_config_t lcd_dev_config = {
        .rgb_ele_order = LCD_RGB_ELEMENT_ORDER_RGB,
        .bits_per_pixel = 24,
        .reset_gpio_num = (gpio_num_t) EXAMPLE_PIN_NUM_LCD_RST,
        .vendor_config = &vendor_config,
    };
    ESP_ERROR_CHECK(esp_lcd_new_panel_jd9365(mipi_dbi_io, &lcd_dev_config,
                                             &mipi_dpi_panel));

#if CONFIG_EXAMPLE_USE_DMA2D_COPY_FRAME
    // use DMA2D to copy draw buffer into frame buffer
    ESP_ERROR_CHECK(esp_lcd_dpi_panel_enable_dma2d(mipi_dpi_panel));
    ESP_LOGI(TAG, "DPI panel added DMA2D draw bitmap hook");
#endif

    ESP_ERROR_CHECK(esp_lcd_panel_reset(mipi_dpi_panel));
    ESP_ERROR_CHECK(esp_lcd_panel_init(mipi_dpi_panel));
    // turn on backlight
    example_bsp_set_lcd_backlight(EXAMPLE_LCD_BK_LIGHT_ON_LEVEL);

    ESP_LOGI(TAG, "Initialize LVGL library");
    lv_init();
    // create a lvgl display
    lv_display_t *display = lv_display_create(EXAMPLE_MIPI_DSI_LCD_H_RES,
                                              EXAMPLE_MIPI_DSI_LCD_V_RES);
    // associate the mipi panel handle to the display
    lv_display_set_user_data(display, mipi_dpi_panel);
    // set color depth
    lv_display_set_color_format(display, LV_COLOR_FORMAT_RGB888);
    // create draw buffer
    void *buf1 = NULL;
    void *buf2 = NULL;
    ESP_LOGI(TAG, "Allocate separate LVGL draw buffers");

    size_t alignment = 1;
#if CONFIG_EXAMPLE_USE_DMA2D_COPY_FRAME
    if (esp_efuse_is_flash_encryption_enabled()) {
        alignment = SOC_GDMA_EXT_MEM_ENC_ALIGNMENT;
        if (EXAMPLE_MIPI_DSI_LCD_H_RES % alignment != 0) {
            ESP_LOGW(TAG,
                     "EXAMPLE_MIPI_DSI_LCD_H_RES is not aligned to %d, may "
                     "cause MSPI error",
                     alignment);
        }
    }
#endif
    size_t draw_buffer_sz = EXAMPLE_MIPI_DSI_LCD_H_RES *
                            EXAMPLE_LVGL_DRAW_BUF_LINES * sizeof(lv_color_t);

    // Note:
    // Keep the display buffer in **internal** RAM can speed up the UI because
    // LVGL uses it a lot and it should have a fast access time It also keeps
    // LVGL rendering traffic off PSRAM, which the DPI panel needs for frame
    // buffer reads
    buf1 = heap_caps_aligned_calloc(alignment, 1, draw_buffer_sz,
                                    MALLOC_CAP_INTERNAL | MALLOC_CAP_DMA);
    assert(buf1);
    buf2 = heap_caps_aligned_calloc(alignment, 1, draw_buffer_sz,
                                    MALLOC_CAP_INTERNAL | MALLOC_CAP_DMA);
    assert(buf2);
    // initialize LVGL draw buffers
    lv_display_set_buffers(display, buf1, buf2, draw_buffer_sz,
                           LV_DISPLAY_RENDER_MODE_PARTIAL);
    // set the callback which can copy the rendered image to an area of the
    // display
    lv_display_set_flush_cb(display, example_lvgl_flush_cb);

#if CONFIG_EXAMPLE_USE_DMA2D_COPY_FRAME
    // If flash encryption is enabled, DMA2D requires the flush buffer address
    // and size to be aligned to 16 bytes. We need to round the flush area to
    // the multiple of 16.
    if (esp_efuse_is_flash_encryption_enabled()) {
        ESP_LOGI(TAG, "Register event callback for LVGL flush area rounding");
        lv_display_add_event_cb(display, example_rounder_flush_area_cb,
                                LV_EVENT_INVALIDATE_AREA, NULL);
    }
#endif

    ESP_LOGI(
        TAG,
        "Register DPI panel event callback for LVGL flush ready notification");
    esp_lcd_dpi_panel_event_callbacks_t cbs = {
        .on_color_trans_done = example_notify_lvgl_flush_ready,
#if CONFIG_EXAMPLE_MONITOR_REFRESH_BY_GPIO
        .on_vsync = example_monitor_refresh_rate,
#endif
    };
    ESP_ERROR_CHECK(esp_lcd_dpi_panel_register_event_callbacks(mipi_dpi_panel,
                                                               &cbs, display));

    ESP_LOGI(TAG, "Use esp_timer as LVGL tick timer");
    const esp_timer_create_args_t lvgl_tick_timer_args = {
        .callback = &example_increase_lvgl_tick, .name = "lvgl_tick"};
    esp_timer_handle_t lvgl_tick_timer = NULL;
    ESP_ERROR_CHECK(esp_timer_create(&lvgl_tick_timer_args, &lvgl_tick_timer));
    ESP_ERROR_CHECK(esp_timer_start_periodic(
        lvgl_tick_timer, EXAMPLE_LVGL_TICK_PERIOD_MS * 1000));

    ESP_LOGI(TAG, "Initialize touch");
    lv_indev_t *touch = lv_indev_create();
    lv_indev_set_type(touch, LV_INDEV_TYPE_POINTER);
    lv_indev_set_display(touch, display);
    lv_indev_set_user_data(touch, example_bsp_init_touch_and_brightness());
    lv_indev_set_read_cb(touch, example_lvgl_touch_cb);

    lv_obj_t *cursor = lv_obj_create(lv_layer_sys());
    lv_obj_remove_style_all(cursor);
    lv_obj_set_size(cursor, 30, 30);
    lv_obj_set_style_radius(cursor, LV_RADIUS_CIRCLE, 0);
    lv_obj_set_style_bg_color(cursor, lv_palette_main(LV_PALETTE_RED), 0);
    lv_obj_set_style_bg_opa(cursor, LV_OPA_COVER, 0);
    lv_indev_set_cursor(touch, cursor);

    ESP_LOGI(TAG, "Create LVGL task");
    xTaskCreate(example_lvgl_port_task, "LVGL", EXAMPLE_LVGL_TASK_STACK_SIZE,
                NULL, EXAMPLE_LVGL_TASK_PRIORITY, NULL);

    ESP_LOGI(TAG, "Display LVGL Meter Widget");
    _lock_acquire(&lvgl_api_lock);
    example_lvgl_demo_ui(display);

    lv_obj_t *slider = lv_slider_create(lv_layer_top());
    lv_obj_set_width(slider, EXAMPLE_MIPI_DSI_LCD_H_RES * 3 / 4);
    lv_obj_align(slider, LV_ALIGN_BOTTOM_MID, 0, -60);
    lv_slider_set_range(slider, 10, 255);
    lv_slider_set_value(slider, 255, LV_ANIM_OFF);
    lv_obj_add_event_cb(slider, example_brightness_slider_cb,
                        LV_EVENT_VALUE_CHANGED, NULL);

    lv_obj_t *fill = lv_obj_create(lv_layer_sys());
    lv_obj_remove_style_all(fill);
    lv_obj_set_size(fill, LV_PCT(100), LV_PCT(100));
    lv_obj_set_style_bg_opa(fill, LV_OPA_COVER, 0);
    lv_obj_add_flag(fill, static_cast<lv_obj_flag_t>(LV_OBJ_FLAG_HIDDEN | LV_OBJ_FLAG_CLICKABLE));
    lv_obj_add_event_cb(fill, example_fill_test_cb, LV_EVENT_CLICKED, fill);

    lv_obj_t *fill_btn = lv_button_create(lv_layer_top());
    lv_obj_align(fill_btn, LV_ALIGN_TOP_RIGHT, -20, 20);
    lv_label_set_text(lv_label_create(fill_btn), "Fill test");
    lv_obj_add_event_cb(fill_btn, example_fill_test_cb, LV_EVENT_CLICKED, fill);

    _lock_release(&lvgl_api_lock);

    bridge_ble_init();
}
