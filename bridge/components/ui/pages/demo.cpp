#include "ui.hpp"

static ui::demo::callbacks cbs;
static lv_style_t style_bullet;
static lv_obj_t *scale1;

static lv_obj_t *create_scale_box(lv_obj_t *parent, const char *text1,
                                  const char *text2, const char *text3) {
    lv_obj_t *scale = lv_scale_create(parent);
    lv_obj_center(scale);
    lv_obj_set_size(scale, 600, 600);
    lv_scale_set_mode(scale, LV_SCALE_MODE_ROUND_OUTER);
    lv_scale_set_label_show(scale, false);
    lv_scale_set_post_draw(scale, true);
    lv_obj_set_width(scale, LV_PCT(100));
    lv_obj_set_style_pad_all(scale, 30, 0);

    lv_obj_t *bullet1 = lv_obj_create(parent);
    lv_obj_set_size(bullet1, 13, 13);
    lv_obj_remove_style(bullet1, NULL, LV_PART_SCROLLBAR);
    lv_obj_add_style(bullet1, &style_bullet, 0);
    lv_obj_set_style_bg_color(bullet1, lv_palette_main(LV_PALETTE_RED), 0);
    lv_obj_t *label1 = lv_label_create(parent);
    lv_label_set_text(label1, text1);

    lv_obj_t *bullet2 = lv_obj_create(parent);
    lv_obj_set_size(bullet2, 13, 13);
    lv_obj_remove_style(bullet2, NULL, LV_PART_SCROLLBAR);
    lv_obj_add_style(bullet2, &style_bullet, 0);
    lv_obj_set_style_bg_color(bullet2, lv_palette_main(LV_PALETTE_BLUE), 0);
    lv_obj_t *label2 = lv_label_create(parent);
    lv_label_set_text(label2, text2);

    lv_obj_t *bullet3 = lv_obj_create(parent);
    lv_obj_set_size(bullet3, 13, 13);
    lv_obj_remove_style(bullet3, NULL, LV_PART_SCROLLBAR);
    lv_obj_add_style(bullet3, &style_bullet, 0);
    lv_obj_set_style_bg_color(bullet3, lv_palette_main(LV_PALETTE_GREEN), 0);
    lv_obj_t *label3 = lv_label_create(parent);
    lv_label_set_text(label3, text3);

    static int32_t grid_col_dsc[] = {LV_GRID_CONTENT, LV_GRID_FR(1),
                                     LV_GRID_TEMPLATE_LAST};
    static int32_t grid_row_dsc[] = {LV_GRID_CONTENT, LV_GRID_CONTENT,
                                     LV_GRID_CONTENT, LV_GRID_CONTENT,
                                     LV_GRID_CONTENT, LV_GRID_TEMPLATE_LAST};
    lv_obj_set_grid_dsc_array(parent, grid_col_dsc, grid_row_dsc);
    lv_obj_set_grid_cell(scale, LV_GRID_ALIGN_START, 0, 2, LV_GRID_ALIGN_START,
                         1, 1);
    lv_obj_set_grid_cell(bullet1, LV_GRID_ALIGN_START, 0, 1,
                         LV_GRID_ALIGN_START, 2, 1);
    lv_obj_set_grid_cell(bullet2, LV_GRID_ALIGN_START, 0, 1,
                         LV_GRID_ALIGN_START, 3, 1);
    lv_obj_set_grid_cell(bullet3, LV_GRID_ALIGN_START, 0, 1,
                         LV_GRID_ALIGN_START, 4, 1);
    lv_obj_set_grid_cell(label1, LV_GRID_ALIGN_STRETCH, 1, 1,
                         LV_GRID_ALIGN_START, 2, 1);
    lv_obj_set_grid_cell(label2, LV_GRID_ALIGN_STRETCH, 1, 1,
                         LV_GRID_ALIGN_START, 3, 1);
    lv_obj_set_grid_cell(label3, LV_GRID_ALIGN_STRETCH, 1, 1,
                         LV_GRID_ALIGN_START, 4, 1);
    return scale;
}

static void scale1_indic1_anim_cb(void *var, int32_t v) {
    lv_arc_set_value(static_cast<lv_obj_t *>(var), v);

    lv_obj_t *card = lv_obj_get_parent(scale1);
    lv_obj_t *label = lv_obj_get_child(card, -5);
    lv_label_set_text_fmt(label, "Revenue: %" LV_PRId32 " %%", v);
}

static void scale1_indic2_anim_cb(void *var, int32_t v) {
    lv_arc_set_value(static_cast<lv_obj_t *>(var), v);

    lv_obj_t *card = lv_obj_get_parent(scale1);
    lv_obj_t *label = lv_obj_get_child(card, -3);
    lv_label_set_text_fmt(label, "Sales: %" LV_PRId32 " %%", v);
}

static void scale1_indic3_anim_cb(void *var, int32_t v) {
    lv_arc_set_value(static_cast<lv_obj_t *>(var), v);

    lv_obj_t *card = lv_obj_get_parent(scale1);
    lv_obj_t *label = lv_obj_get_child(card, -1);
    lv_label_set_text_fmt(label, "Costs: %" LV_PRId32 " %%", v);
}

static void create_scales(lv_obj_t *parent) {
    static bool style_ready = false;
    if (!style_ready) {
        lv_style_init(&style_bullet);
        lv_style_set_border_width(&style_bullet, 0);
        lv_style_set_radius(&style_bullet, LV_RADIUS_CIRCLE);
        style_ready = true;
    }

    scale1 = create_scale_box(parent, "Revenue", "Sales", "Costs");

    lv_obj_t *arc;
    arc = lv_arc_create(scale1);
    lv_obj_remove_style(arc, NULL, LV_PART_KNOB);
    lv_obj_remove_style(arc, NULL, LV_PART_MAIN);
    lv_obj_set_size(arc, lv_pct(100), lv_pct(100));
    lv_obj_set_style_arc_opa(arc, 0, 0);
    lv_obj_set_style_arc_width(arc, 15, LV_PART_INDICATOR);
    lv_obj_set_style_arc_color(arc, lv_palette_main(LV_PALETTE_BLUE),
                               LV_PART_INDICATOR);
    lv_obj_remove_flag(arc, LV_OBJ_FLAG_CLICKABLE);

    lv_anim_t a;
    lv_anim_init(&a);
    lv_anim_set_values(&a, 20, 100);
    lv_anim_set_repeat_count(&a, LV_ANIM_REPEAT_INFINITE);
    lv_anim_set_exec_cb(&a, scale1_indic1_anim_cb);
    lv_anim_set_var(&a, arc);
    lv_anim_set_duration(&a, 4100);
    lv_anim_set_playback_duration(&a, 2700);
    lv_anim_start(&a);

    arc = lv_arc_create(scale1);
    lv_obj_remove_style(arc, NULL, LV_PART_KNOB);
    lv_obj_set_size(arc, lv_pct(100), lv_pct(100));
    lv_obj_set_style_margin_all(arc, 20, 0);
    lv_obj_set_style_arc_opa(arc, 0, 0);
    lv_obj_set_style_arc_width(arc, 15, LV_PART_INDICATOR);
    lv_obj_set_style_arc_color(arc, lv_palette_main(LV_PALETTE_RED),
                               LV_PART_INDICATOR);
    lv_obj_remove_flag(arc, LV_OBJ_FLAG_CLICKABLE);
    lv_obj_center(arc);

    lv_anim_set_exec_cb(&a, scale1_indic2_anim_cb);
    lv_anim_set_var(&a, arc);
    lv_anim_set_duration(&a, 2600);
    lv_anim_set_playback_duration(&a, 3200);
    lv_anim_start(&a);

    arc = lv_arc_create(scale1);
    lv_obj_remove_style(arc, NULL, LV_PART_KNOB);
    lv_obj_set_size(arc, lv_pct(100), lv_pct(100));
    lv_obj_set_style_margin_all(arc, 40, 0);
    lv_obj_set_style_arc_opa(arc, 0, 0);
    lv_obj_set_style_arc_width(arc, 15, LV_PART_INDICATOR);
    lv_obj_set_style_arc_color(arc, lv_palette_main(LV_PALETTE_GREEN),
                               LV_PART_INDICATOR);
    lv_obj_remove_flag(arc, LV_OBJ_FLAG_CLICKABLE);
    lv_obj_center(arc);

    lv_anim_set_exec_cb(&a, scale1_indic3_anim_cb);
    lv_anim_set_var(&a, arc);
    lv_anim_set_duration(&a, 2800);
    lv_anim_set_playback_duration(&a, 1800);
    lv_anim_start(&a);
}

static void brightness_slider_cb(lv_event_t *e) {
    if (cbs.set_brightness) {
        uint8_t brightness = lv_slider_get_value(lv_event_get_target_obj(e));
        cbs.set_brightness(brightness);
    }
}

static void fill_test_cb(lv_event_t *e) {
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

void ui::demo::set_callbacks(callbacks c) { cbs = std::move(c); }

void ui::demo::show() {
    lv_obj_t *scr = lv_obj_create(nullptr);

    lv_obj_t *content = lv_obj_create(scr);
    lv_obj_remove_style_all(content);
    lv_obj_set_size(content, LV_PCT(100), LV_PCT(100));
    create_scales(content);

    lv_obj_t *slider = lv_slider_create(scr);
    lv_obj_set_width(slider, LV_PCT(75));
    lv_obj_align(slider, LV_ALIGN_BOTTOM_MID, 0, -60);
    lv_slider_set_range(slider, 10, 255);
    lv_slider_set_value(slider, 255, LV_ANIM_OFF);
    lv_obj_add_event_cb(slider, brightness_slider_cb, LV_EVENT_VALUE_CHANGED,
                        nullptr);

    lv_obj_t *fill_btn = lv_button_create(scr);
    lv_obj_align(fill_btn, LV_ALIGN_TOP_RIGHT, -20, 20);
    lv_label_set_text(lv_label_create(fill_btn), "Fill test");

    lv_obj_t *fill = lv_obj_create(scr);
    lv_obj_remove_style_all(fill);
    lv_obj_set_size(fill, LV_PCT(100), LV_PCT(100));
    lv_obj_set_style_bg_opa(fill, LV_OPA_COVER, 0);
    lv_obj_add_flag(fill, static_cast<lv_obj_flag_t>(LV_OBJ_FLAG_HIDDEN |
                                                     LV_OBJ_FLAG_CLICKABLE));
    lv_obj_add_event_cb(fill, fill_test_cb, LV_EVENT_CLICKED, fill);
    lv_obj_add_event_cb(fill_btn, fill_test_cb, LV_EVENT_CLICKED, fill);

    lv_screen_load_anim(scr, LV_SCREEN_LOAD_ANIM_FADE_IN, 200, 0, true);
}
