#include "ui.hpp"

void ui::init(lv_display_t *disp) {
    lv_theme_default_init(disp, lv_palette_main(LV_PALETTE_BLUE),
                          lv_palette_main(LV_PALETTE_RED),
                          LV_THEME_DEFAULT_DARK, &lv_font_montserrat_16);
}
