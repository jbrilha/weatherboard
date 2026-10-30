#include "ui.hpp"
#include "display.hpp"
#include "esp_err.h"
#include "esp_heap_caps.h"
#include "esp_lcd_panel_ops.h"
#include "esp_log.h"
#include "esp_timer.h"
#include "lvgl.h"
#include "lvgl_demo_ui.hpp"
#include <sys/unistd.h>

#include <cassert>

static constexpr char TAG[] = "UI";

// number of display lines in each draw buffer
static constexpr uint32_t LVGL_DRAW_BUF_LINES = (MIPI_DSI_LCD_V_RES / 32);
static constexpr uint64_t LVGL_TICK_PERIOD_MS = 2;
static constexpr uint32_t LVGL_TASK_STACK_SIZE = (4 * 1024);
static constexpr uint32_t LVGL_TASK_PRIORITY = 2;
static constexpr uint32_t LVGL_TASK_MAX_DELAY_MS = 500;
static constexpr uint32_t LVGL_TASK_MIN_DELAY_MS = (1000 / CONFIG_FREERTOS_HZ);

UI::UI()
    : lv_display(nullptr), lv_touch(nullptr), lv_cursor(nullptr),
      tick_timer(nullptr), lv_task(nullptr), buf1(nullptr), buf2(nullptr) {}

UI::~UI() {
    if (tick_timer != nullptr) {
        esp_timer_stop(tick_timer);
        esp_timer_delete(tick_timer);
    }
    if (lv_is_initialized()) {
        lv_lock();
        if (lv_task != nullptr) {
            vTaskDelete(lv_task);
        }
        lv_unlock();
        esp_lcd_dpi_panel_event_callbacks_t cbs = {};
        esp_lcd_dpi_panel_register_event_callbacks(display.get_panel_handle(),
                                                   &cbs, nullptr);
        lv_deinit();
    }
    heap_caps_free(buf1);
    heap_caps_free(buf2);
}

static void lvgl_flush_cb(lv_display_t *disp, const lv_area_t *area,
                          uint8_t *px_map) {
    esp_lcd_panel_handle_t panel_handle =
        static_cast<esp_lcd_panel_handle_t>(lv_display_get_user_data(disp));
    int offsetx1 = area->x1;
    int offsetx2 = area->x2;
    int offsety1 = area->y1;
    int offsety2 = area->y2;
    // pass the draw buffer to the driver
    esp_lcd_panel_draw_bitmap(panel_handle, offsetx1, offsety1, offsetx2 + 1,
                              offsety2 + 1, px_map);
}

static void lvgl_touch_cb(lv_indev_t *indev, lv_indev_data_t *data) {
    esp_lcd_touch_handle_t tp =
        static_cast<esp_lcd_touch_handle_t>(lv_indev_get_user_data(indev));
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

static bool notify_lvgl_flush_ready(esp_lcd_panel_handle_t panel,
                                    esp_lcd_dpi_panel_event_data_t *edata,
                                    void *user_ctx) {
    lv_display_t *disp = (lv_display_t *)user_ctx;
    lv_display_flush_ready(disp);
    return false;
}

static void increase_lvgl_tick(void *arg) { lv_tick_inc(LVGL_TICK_PERIOD_MS); }

static void lvgl_port_task(void *arg) {
    ESP_LOGI(TAG, "Starting LVGL task");
    uint32_t time_till_next_ms = 0;
    while (true) {
        lv_lock();
        time_till_next_ms = lv_timer_handler();
        lv_unlock();
        // in case of triggering a task watch dog time out
        time_till_next_ms = MAX(time_till_next_ms, LVGL_TASK_MIN_DELAY_MS);
        // in case of lvgl display not ready yet
        time_till_next_ms = MIN(time_till_next_ms, LVGL_TASK_MAX_DELAY_MS);
        usleep(1000 * time_till_next_ms);
    }
}

esp_err_t UI::init() {

    ESP_ERROR_CHECK(display.init());
    esp_lcd_panel_handle_t panel = display.get_panel_handle();

    lv_init();
    lv_display = lv_display_create(MIPI_DSI_LCD_H_RES, MIPI_DSI_LCD_V_RES);
    lv_display_set_user_data(lv_display, panel);
    lv_display_set_color_format(lv_display, LV_COLOR_FORMAT_RGB888);

    size_t alignment = 1;

    size_t draw_buffer_sz =
        MIPI_DSI_LCD_H_RES * LVGL_DRAW_BUF_LINES * sizeof(lv_color_t);

    buf1 = heap_caps_aligned_calloc(alignment, 1, draw_buffer_sz,
                                    MALLOC_CAP_INTERNAL | MALLOC_CAP_DMA);
    assert(buf1);
    buf2 = heap_caps_aligned_calloc(alignment, 1, draw_buffer_sz,
                                    MALLOC_CAP_INTERNAL | MALLOC_CAP_DMA);
    assert(buf2);

    lv_display_set_buffers(lv_display, buf1, buf2, draw_buffer_sz,
                           LV_DISPLAY_RENDER_MODE_PARTIAL);
    lv_display_set_flush_cb(lv_display, lvgl_flush_cb);

    esp_lcd_dpi_panel_event_callbacks_t cbs = {};
    cbs.on_color_trans_done = notify_lvgl_flush_ready;
    ESP_ERROR_CHECK(
        esp_lcd_dpi_panel_register_event_callbacks(panel, &cbs, lv_display));

    ESP_LOGI(TAG, "Use esp_timer as LVGL tick timer");
    const esp_timer_create_args_t lvgl_tick_timer_args = {
        .callback = &increase_lvgl_tick,
        .arg = nullptr,
        .dispatch_method = ESP_TIMER_TASK,
        .name = "lvgl_tick",
        .skip_unhandled_events = false,
    };
    ESP_ERROR_CHECK(esp_timer_create(&lvgl_tick_timer_args, &tick_timer));
    ESP_ERROR_CHECK(
        esp_timer_start_periodic(tick_timer, LVGL_TICK_PERIOD_MS * 1000));

    lv_touch = lv_indev_create();
    lv_indev_set_type(lv_touch, LV_INDEV_TYPE_POINTER);
    lv_indev_set_display(lv_touch, lv_display);
    lv_indev_set_user_data(lv_touch, display.get_touch_handle());
    lv_indev_set_read_cb(lv_touch, lvgl_touch_cb);

    lv_cursor = lv_obj_create(lv_layer_sys());
    lv_obj_remove_style_all(lv_cursor);
    lv_obj_set_size(lv_cursor, 20, 20);
    lv_obj_set_style_radius(lv_cursor, LV_RADIUS_CIRCLE, 0);
    lv_obj_set_style_bg_color(lv_cursor, lv_palette_main(LV_PALETTE_RED), 0);
    lv_obj_set_style_bg_opa(lv_cursor, LV_OPA_30, 0);
    lv_indev_set_cursor(lv_touch, lv_cursor);

    xTaskCreate(lvgl_port_task, "LVGL", LVGL_TASK_STACK_SIZE, NULL,
                LVGL_TASK_PRIORITY, &lv_task);

    return ESP_OK;
}

static void example_brightness_slider_cb(lv_event_t *e) {
    auto *display = static_cast<Display *>(lv_event_get_user_data(e));

    display->set_brightness(lv_slider_get_value(lv_event_get_target_obj(e)));
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

void UI::draw_example(void) {
    lv_lock();
    example_lvgl_demo_ui(this->lv_display);

    lv_obj_t *slider = lv_slider_create(lv_layer_top());
    lv_obj_set_width(slider, MIPI_DSI_LCD_H_RES * 3 / 4);
    lv_obj_align(slider, LV_ALIGN_BOTTOM_MID, 0, -60);
    lv_slider_set_range(slider, 10, 255);
    lv_slider_set_value(slider, 255, LV_ANIM_OFF);
    lv_obj_add_event_cb(slider, example_brightness_slider_cb,
                        LV_EVENT_VALUE_CHANGED, &this->display);

    lv_obj_t *fill = lv_obj_create(lv_layer_sys());
    lv_obj_remove_style_all(fill);
    lv_obj_set_size(fill, LV_PCT(100), LV_PCT(100));
    lv_obj_set_style_bg_opa(fill, LV_OPA_COVER, 0);
    lv_obj_add_flag(fill, static_cast<lv_obj_flag_t>(LV_OBJ_FLAG_HIDDEN |
                                                     LV_OBJ_FLAG_CLICKABLE));
    lv_obj_add_event_cb(fill, example_fill_test_cb, LV_EVENT_CLICKED, fill);

    lv_obj_t *fill_btn = lv_button_create(lv_layer_top());
    lv_obj_align(fill_btn, LV_ALIGN_TOP_RIGHT, -20, 20);
    lv_label_set_text(lv_label_create(fill_btn), "Fill test");
    lv_obj_add_event_cb(fill_btn, example_fill_test_cb, LV_EVENT_CLICKED, fill);

    lv_unlock();
}
