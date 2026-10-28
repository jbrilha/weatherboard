#include "display.hpp"
#include "esp_err.h"
#include "esp_lcd_jd9365_10_1.h"
#include "esp_lcd_mipi_dsi.h"
#include "esp_lcd_panel_ops.h"
#include "esp_lcd_touch.h"
#include "esp_lcd_touch_gt911.h"
#include "esp_ldo_regulator.h"
#include "hal/lcd_types.h"
#include "soc/clk_tree_defs.h"
#include "soc/gpio_num.h"

static constexpr char TAG[] = "Display";

// 80000000/(20+20+40+800)/(4+10+30+1280) = 68Hz
static constexpr uint32_t MIPI_DSI_DPI_CLK_MHZ = 80;
static constexpr uint32_t MIPI_DSI_LCD_H_RES = 800;
static constexpr uint32_t MIPI_DSI_LCD_V_RES = 1280;
static constexpr uint32_t MIPI_DSI_LCD_HSYNC = 20;
static constexpr uint32_t MIPI_DSI_LCD_HBP = 20;
static constexpr uint32_t MIPI_DSI_LCD_HFP = 40;
static constexpr uint32_t MIPI_DSI_LCD_VSYNC = 4;
static constexpr uint32_t MIPI_DSI_LCD_VBP = 10;
static constexpr uint32_t MIPI_DSI_LCD_VFP = 30;

static constexpr uint32_t MIPI_DSI_LANE_NUM = 2;             // 2 data lanes
static constexpr uint32_t MIPI_DSI_LANE_BITRATE_MBPS = 1500; // 1.5Gbps

Display::Display()
    : mipi_dsi_bus(nullptr), mipi_dbi_io(nullptr), panel_handle(nullptr),
      vendor_config{} {
    bus_config = {
        .bus_id = 0,
        .num_data_lanes = MIPI_DSI_LANE_NUM,
        .phy_clk_src = MIPI_DSI_PHY_CLK_SRC_DEFAULT,
        .lane_bit_rate_mbps = MIPI_DSI_LANE_BITRATE_MBPS,
        .flags = {},
    };

    dbi_config = {
        .virtual_channel = 0,
        .lcd_cmd_bits = 8,
        .lcd_param_bits = 8,
    };

    dpi_config = {
        .virtual_channel = 0,
        .dpi_clk_src = MIPI_DSI_DPI_CLK_SRC_DEFAULT,
        .dpi_clock_freq_mhz = MIPI_DSI_DPI_CLK_MHZ,
        .in_color_format = LCD_COLOR_FMT_RGB888,
        .out_color_format = LCD_COLOR_FMT_RGB888,
        .num_fbs = 1,
        .video_timing =
            {
                .h_size = MIPI_DSI_LCD_H_RES,
                .v_size = MIPI_DSI_LCD_V_RES,
                .hsync_pulse_width = MIPI_DSI_LCD_HSYNC,
                .hsync_back_porch = MIPI_DSI_LCD_HBP,
                .hsync_front_porch = MIPI_DSI_LCD_HFP,
                .vsync_pulse_width = MIPI_DSI_LCD_VSYNC,
                .vsync_back_porch = MIPI_DSI_LCD_VBP,
                .vsync_front_porch = MIPI_DSI_LCD_VFP,
            },
        .flags = {},
    };

    vendor_config.init_cmds = nullptr;
    vendor_config.init_cmds_size = 0;
    vendor_config.mipi_config.dsi_bus = mipi_dsi_bus;
    vendor_config.mipi_config.dpi_config = &dpi_config;
    vendor_config.mipi_config.lane_num = MIPI_DSI_LANE_NUM;
    vendor_config.flags.use_mipi_interface = 1;

    lcd_dev_config = {
        .rgb_ele_order = LCD_RGB_ELEMENT_ORDER_RGB,
        .data_endian = LCD_RGB_DATA_ENDIAN_BIG,
        .bits_per_pixel = 24,
        .reset_gpio_num =
            GPIO_NUM_NC, // TODO check the schematic to see if this exists
        .vendor_config = &vendor_config,
        .flags = {},
    };
}

esp_err_t Display::init(void) {
    ESP_ERROR_CHECK(esp_lcd_new_dsi_bus(&bus_config, &mipi_dsi_bus));
    ESP_ERROR_CHECK(
        esp_lcd_new_panel_io_dbi(mipi_dsi_bus, &dbi_config, &mipi_dbi_io));
    ESP_ERROR_CHECK(
        esp_lcd_new_panel_jd9365(mipi_dbi_io, &lcd_dev_config, &panel_handle));

    ESP_ERROR_CHECK(esp_lcd_panel_reset(panel_handle));
    ESP_ERROR_CHECK(esp_lcd_panel_init(panel_handle));

    return ESP_OK;
}
