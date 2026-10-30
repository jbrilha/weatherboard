#include "lvgl.h"
#include "lvgl_demo_ui.hpp"
#include <SDL3/SDL.h>

static constexpr int32_t H_RES = 800;
static constexpr int32_t V_RES = 1280;
static constexpr int32_t BUF_LINES = V_RES / 10;

static SDL_Renderer *renderer;
static SDL_Texture *texture;
static lv_point_t mouse_pos;
static bool mouse_down;

static void flush_cb(lv_display_t *disp, const lv_area_t *area,
                     uint8_t *px_map) {
    SDL_Rect rect = {area->x1, area->y1, lv_area_get_width(area),
                     lv_area_get_height(area)};
    SDL_UpdateTexture(texture, &rect, px_map, rect.w * 3);

    if (lv_display_flush_is_last(disp)) {
        SDL_RenderClear(renderer);
        SDL_RenderTexture(renderer, texture, nullptr, nullptr);
        SDL_RenderPresent(renderer);
    }
    lv_display_flush_ready(disp);
}

static void mouse_read_cb(lv_indev_t *indev, lv_indev_data_t *data) {
    data->point = mouse_pos;
    data->state = mouse_down ? LV_INDEV_STATE_PRESSED : LV_INDEV_STATE_RELEASED;
}

static bool handle_events() {
    SDL_Event e;
    while (SDL_PollEvent(&e)) {
        SDL_ConvertEventToRenderCoordinates(renderer, &e);
        switch (e.type) {
        case SDL_EVENT_QUIT:
            return false;
        case SDL_EVENT_MOUSE_MOTION:
            mouse_pos = {(int32_t)e.motion.x, (int32_t)e.motion.y};
            break;
        case SDL_EVENT_MOUSE_BUTTON_DOWN:
        case SDL_EVENT_MOUSE_BUTTON_UP:
            mouse_pos = {(int32_t)e.button.x, (int32_t)e.button.y};
            mouse_down = e.button.down;
            break;
        }
    }
    return true;
}

int main() {
    SDL_Init(SDL_INIT_VIDEO);

    float scale = SDL_GetDisplayContentScale(SDL_GetPrimaryDisplay());
    if (scale <= 0) {
        scale = 1;
    }

    SDL_Window *window = SDL_CreateWindow(
        "bridge sim", (int)(H_RES / scale), (int)(V_RES / scale),
        SDL_WINDOW_HIGH_PIXEL_DENSITY | SDL_WINDOW_RESIZABLE);
    renderer = SDL_CreateRenderer(window, nullptr);
    SDL_SetRenderLogicalPresentation(renderer, H_RES, V_RES,
                                     SDL_LOGICAL_PRESENTATION_LETTERBOX);
    texture = SDL_CreateTexture(renderer, SDL_PIXELFORMAT_BGR24,
                                SDL_TEXTUREACCESS_STREAMING, H_RES, V_RES);

    lv_init();
    lv_tick_set_cb([]() -> uint32_t { return SDL_GetTicks(); });

    lv_display_t *disp = lv_display_create(H_RES, V_RES);
    lv_display_set_color_format(disp, LV_COLOR_FORMAT_RGB888);
    static uint8_t buf1[H_RES * BUF_LINES * 3];
    static uint8_t buf2[H_RES * BUF_LINES * 3];
    lv_display_set_buffers(disp, buf1, buf2, sizeof(buf1),
                           LV_DISPLAY_RENDER_MODE_PARTIAL);
    lv_display_set_flush_cb(disp, flush_cb);

    lv_indev_t *mouse = lv_indev_create();
    lv_indev_set_type(mouse, LV_INDEV_TYPE_POINTER);
    lv_indev_set_display(mouse, disp);
    lv_indev_set_read_cb(mouse, mouse_read_cb);

    example_lvgl_demo_ui(disp);

    while (handle_events()) {
        SDL_Delay(lv_timer_handler());
    }

    SDL_DestroyTexture(texture);
    SDL_DestroyRenderer(renderer);
    SDL_DestroyWindow(window);
    SDL_Quit();
}
