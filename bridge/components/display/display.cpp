#include "display.hpp"
#include "driver/i2c_master.h"
#include "esp_err.h"
#include "esp_lcd_jd9365_10_1.h"
#include "esp_lcd_mipi_dsi.h"
#include "esp_lcd_panel_ops.h"
#include "esp_lcd_touch.h"
#include "esp_lcd_touch_gt911.h"
#include "esp_lcd_types.h"
#include "esp_ldo_regulator.h"
#include "esp_log.h"
#include "hal/lcd_types.h"
#include "soc/clk_tree_defs.h"
#include "soc/gpio_num.h"
#include <memory>

static constexpr char TAG[] = "Display";

// 80000000/(20+20+40+800)/(4+10+30+1280) = 68Hz
static constexpr uint32_t MIPI_DSI_DPI_CLK_MHZ = 70;
static constexpr uint32_t MIPI_DSI_LCD_HSYNC = 20;
static constexpr uint32_t MIPI_DSI_LCD_HBP = 20;
static constexpr uint32_t MIPI_DSI_LCD_HFP = 40;
static constexpr uint32_t MIPI_DSI_LCD_VSYNC = 4;
static constexpr uint32_t MIPI_DSI_LCD_VBP = 10;
static constexpr uint32_t MIPI_DSI_LCD_VFP = 30;

static constexpr uint32_t MIPI_DSI_LANE_NUM = 2;             // 2 data lanes
static constexpr uint32_t MIPI_DSI_LANE_BITRATE_MBPS = 1500; // 1.5Gbps

// LDO_VO3 is connected to VDD_MIPI_DPHY
static constexpr uint32_t MIPI_DSI_PHY_PWR_LDO_CHAN = 3;
static constexpr uint32_t MIPI_DSI_PHY_PWR_LDO_VOLTAGE_MV = 2500;

static constexpr uint32_t GT911_ADDR = 0x5D;
static constexpr uint32_t GT911_CLK_HZ = 100000;
static constexpr uint32_t PIN_NUM_TOUCH_SDA = 7;
static constexpr uint32_t PIN_NUM_TOUCH_SCL = 8;
static constexpr uint32_t BACKLIGHT_I2C_ADDR = 0x45;
static constexpr uint32_t BACKLIGHT_I2C_REG = 0x96;

Display::Display()
    : mipi_dsi_bus(nullptr), mipi_dbi_io(nullptr), panel_handle(nullptr),
      vendor_config{}, touch_handle(nullptr), backlight_cfg{},
      backlight_dev(nullptr), i2c_bus(nullptr), tp_io(nullptr),
      ldo_mipi_phy(nullptr) {
    bus_config = {
        .bus_id = 0,
        .num_data_lanes = MIPI_DSI_LANE_NUM,
        .phy_clk_src = MIPI_DSI_PHY_PLLREF_CLK_SRC_DEFAULT,
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

    // ----------------------------------------- //

    touch_config = {
        .x_max = MIPI_DSI_LCD_H_RES,
        .y_max = MIPI_DSI_LCD_V_RES,
        .rst_gpio_num = GPIO_NUM_NC,
        .int_gpio_num = GPIO_NUM_NC,
        .levels = {},
        .flags =
            {
                .swap_xy = 0,
                .mirror_x = 0,
                .mirror_y = 0,
            },
        .process_coordinates = nullptr,
        .interrupt_callback = nullptr,
        .user_data = nullptr,
        .driver_data = nullptr,
    };

    backlight_cfg.device_address = BACKLIGHT_I2C_ADDR;
    backlight_cfg.scl_speed_hz = GT911_CLK_HZ;

    io_cfg = {
        .dev_addr = GT911_ADDR,
        .scl_speed_hz = GT911_CLK_HZ,
        .control_phase_bytes = 1,
        .dc_bit_offset = 0,
        .lcd_cmd_bits = 16,
        .lcd_param_bits = 0,
        .on_color_trans_done = nullptr,
        .user_ctx = nullptr,
        .flags =
            {
                .dc_low_on_data = 0,
                .disable_control_phase = 1,
            },
        .transaction_timeout_ms = 0,
    };
}

Display::~Display() {
    if (touch_handle != nullptr) {
        esp_lcd_touch_del(touch_handle);
    }
    if (tp_io != nullptr) {
        esp_lcd_panel_io_del(tp_io);
    }
    if (backlight_dev != nullptr) {
        i2c_master_bus_rm_device(backlight_dev);
    }
    if (i2c_bus != nullptr) {
        i2c_del_master_bus(i2c_bus);
    }
    if (panel_handle != nullptr) {
        esp_lcd_panel_del(panel_handle);
    }
    if (mipi_dbi_io != nullptr) {
        esp_lcd_panel_io_del(mipi_dbi_io);
    }
    if (mipi_dsi_bus != nullptr) {
        esp_lcd_del_dsi_bus(mipi_dsi_bus);
    }
    if (ldo_mipi_phy != nullptr) {
        esp_ldo_release_channel(ldo_mipi_phy);
    }
}

esp_err_t Display::init(void) {
    ESP_ERROR_CHECK(init_panel());
    ESP_ERROR_CHECK(init_touch());

    return ESP_OK;
}

void Display::enable_dsi_phy_power(void) {
    esp_ldo_channel_config_t ldo_mipi_phy_config = {
        .chan_id = MIPI_DSI_PHY_PWR_LDO_CHAN,
        .voltage_mv = MIPI_DSI_PHY_PWR_LDO_VOLTAGE_MV,
        .voltage_stable_delay_us = 0,
        .flags = {},
    };
    ESP_ERROR_CHECK(
        esp_ldo_acquire_channel(&ldo_mipi_phy_config, &ldo_mipi_phy));
    ESP_LOGI(TAG, "MIPI DSI PHY Powered on");
}

esp_err_t Display::init_panel(void) {
    enable_dsi_phy_power();

    ESP_ERROR_CHECK(esp_lcd_new_dsi_bus(&bus_config, &mipi_dsi_bus));

    vendor_config.mipi_config.dsi_bus = mipi_dsi_bus;

    ESP_ERROR_CHECK(
        esp_lcd_new_panel_io_dbi(mipi_dsi_bus, &dbi_config, &mipi_dbi_io));
    ESP_ERROR_CHECK(
        esp_lcd_new_panel_jd9365(mipi_dbi_io, &lcd_dev_config, &panel_handle));

    ESP_ERROR_CHECK(esp_lcd_dpi_panel_enable_dma2d(panel_handle));

    ESP_ERROR_CHECK(esp_lcd_panel_reset(panel_handle));
    ESP_ERROR_CHECK(esp_lcd_panel_init(panel_handle));

    return ESP_OK;
}

esp_err_t Display::init_touch(void) {
    // TODO move these to I2C component
    i2c_master_bus_config_t bus_cfg = {
        .i2c_port = -1,
        .sda_io_num = (gpio_num_t)PIN_NUM_TOUCH_SDA,
        .scl_io_num = (gpio_num_t)PIN_NUM_TOUCH_SCL,
        .clk_source = I2C_CLK_SRC_DEFAULT,
        .glitch_ignore_cnt = 7,
        .intr_priority = 0,
        .trans_queue_depth = 0,
        .flags =
            {
                .enable_internal_pullup = true,
                .allow_pd = false,
            },
    };
    ESP_ERROR_CHECK(i2c_new_master_bus(&bus_cfg, &i2c_bus));

    ESP_ERROR_CHECK(
        i2c_master_bus_add_device(i2c_bus, &backlight_cfg, &backlight_dev));
    set_brightness(255);

    if (i2c_master_probe(i2c_bus, ESP_LCD_TOUCH_IO_I2C_GT911_ADDRESS, 50) !=
        ESP_OK) {
        io_cfg.dev_addr = ESP_LCD_TOUCH_IO_I2C_GT911_ADDRESS_BACKUP;
    }
    ESP_LOGI(TAG, "Touch controller at 0x%02X", (unsigned)io_cfg.dev_addr);
    ESP_ERROR_CHECK(esp_lcd_new_panel_io_i2c(i2c_bus, &io_cfg, &tp_io));

    ESP_ERROR_CHECK(
        esp_lcd_touch_new_i2c_gt911(tp_io, &touch_config, &touch_handle));

    return ESP_OK;
}

esp_lcd_panel_handle_t Display::get_panel_handle() { return panel_handle; }
esp_lcd_touch_handle_t Display::get_touch_handle() { return touch_handle; }

void Display::set_brightness(uint8_t level) {
    uint8_t buf[2] = {BACKLIGHT_I2C_REG, level};
    ESP_ERROR_CHECK_WITHOUT_ABORT(
        i2c_master_transmit(backlight_dev, buf, sizeof(buf), 50));
}
