#pragma once

#include "display.hpp"
#include "esp_err.h"
#include "esp_timer.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "lvgl.h"

class UI {
  public:
    UI();
    ~UI();

    esp_err_t init(void);
    void draw_example(void);

  private:
    Display display;
    lv_display_t *lv_display;
    lv_indev_t *lv_touch;
    lv_obj_t *lv_cursor;
    esp_timer_handle_t tick_timer;
    TaskHandle_t lv_task;
    void *buf1;
    void *buf2;
};
