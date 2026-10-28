#pragma once

#include "driver/i2c_master.h"
#include "esp_err.h"
#include "esp_lcd_jd9365_10_1.h"
#include "esp_lcd_mipi_dsi.h"
#include "esp_lcd_touch.h"
#include "esp_lcd_types.h"
#include "esp_ldo_regulator.h"

constexpr uint32_t MIPI_DSI_LCD_H_RES = 800;
constexpr uint32_t MIPI_DSI_LCD_V_RES = 1280;

class Display {
  public:
    Display();
    ~Display();

    esp_err_t init(void);

    esp_lcd_panel_handle_t get_panel_handle(void);
    esp_lcd_touch_handle_t get_touch_handle(void);

    void set_brightness(uint8_t level);

  private:
    esp_lcd_dsi_bus_handle_t mipi_dsi_bus;
    esp_lcd_dsi_bus_config_t bus_config;

    esp_lcd_panel_io_handle_t mipi_dbi_io;
    esp_lcd_dbi_io_config_t dbi_config;

    esp_lcd_panel_handle_t panel_handle;
    esp_lcd_dpi_panel_config_t dpi_config;

    jd9365_vendor_config_t vendor_config;
    esp_lcd_panel_dev_config_t lcd_dev_config;

    esp_lcd_touch_handle_t touch_handle;
    esp_lcd_touch_config_t touch_config;

    esp_lcd_panel_io_i2c_config_t io_cfg;
    i2c_device_config_t backlight_cfg;
    i2c_master_dev_handle_t backlight_dev;

    i2c_master_bus_handle_t i2c_bus;
    esp_lcd_panel_io_handle_t tp_io;
    esp_ldo_channel_handle_t ldo_mipi_phy;

    esp_err_t init_panel(void);
    esp_err_t init_touch(void);
    void enable_dsi_phy_power(void);
};
