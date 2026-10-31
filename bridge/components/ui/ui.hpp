#pragma once

#include "lvgl.h"
#include <cstdint>
#include <functional>

namespace ui {

void init(lv_display_t *disp);

namespace demo {

struct callbacks {
    std::function<void(uint8_t)> set_brightness;
};

void set_callbacks(callbacks cbs);
void show();

} // namespace demo

} // namespace ui
