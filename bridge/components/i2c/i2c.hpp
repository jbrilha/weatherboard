#pragma once

#include "driver/i2c_master.h"
#include "esp_err.h"

class I2C {
  public:
    I2C();
    ~I2C();
    I2C(const I2C &) = delete;
    I2C &operator=(const I2C &) = delete;

    esp_err_t init(void);

    i2c_master_bus_handle_t get_bus();

    static esp_err_t add_device_to_bus(i2c_master_bus_handle_t bus_handle,
                                       const i2c_device_config_t *dev_config,
                                       i2c_master_dev_handle_t *ret_handle);

    esp_err_t add_device(const i2c_device_config_t *dev_config,
                         i2c_master_dev_handle_t *ret_handle);

    static esp_err_t probe_addr_in_bus(i2c_master_bus_handle_t bus_handle,
                                       uint16_t addr, int timeout_ms);

    esp_err_t probe_addr(uint16_t addr, int timeout_ms);

  private:
    i2c_master_bus_config_t bus_cfg;
    i2c_master_bus_handle_t i2c_bus;
};
