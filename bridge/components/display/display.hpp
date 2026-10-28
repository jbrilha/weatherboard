#pragma once

#include "esp_err.h"
#include "esp_lcd_jd9365_10_1.h"
#include "esp_lcd_mipi_dsi.h"
#include "esp_lcd_types.h"

class Display {
  public:
    Display();
    ~Display();

    esp_err_t init(void);

  private:
    esp_lcd_dsi_bus_handle_t mipi_dsi_bus;
    esp_lcd_dsi_bus_config_t bus_config;

    esp_lcd_panel_io_handle_t mipi_dbi_io;
    esp_lcd_dbi_io_config_t dbi_config;

    esp_lcd_panel_handle_t panel_handle;
    esp_lcd_dpi_panel_config_t dpi_config;

    jd9365_vendor_config_t vendor_config;
    esp_lcd_panel_dev_config_t lcd_dev_config;
};
