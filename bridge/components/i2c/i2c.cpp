#include "i2c.hpp"
#include "esp_log.h"
#include "soc/gpio_num.h"

static constexpr int32_t PHY_ADDR = 1;
static constexpr int RESET_GPIO_NUM = 51;
static constexpr int MDC_NUM = 31;
static constexpr int MDIO_NUM = 52;

static constexpr char TAG[] = "I2C";

static constexpr gpio_num_t SDA = GPIO_NUM_7;
static constexpr gpio_num_t SCL = GPIO_NUM_8;

I2C::I2C() : i2c_bus(nullptr) {
    bus_cfg = {
        .i2c_port = I2C_NUM_1,
        .sda_io_num = SDA,
        .scl_io_num = SCL,
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
}

I2C::~I2C() {
    if (i2c_bus != nullptr) {
        i2c_del_master_bus(i2c_bus);
    }
}

esp_err_t I2C::init(void) {
    ESP_ERROR_CHECK(i2c_new_master_bus(&bus_cfg, &i2c_bus));
    return ESP_OK;
}

i2c_master_bus_handle_t I2C::get_bus() { return i2c_bus; }

esp_err_t I2C::add_device(const i2c_device_config_t *dev_config,
                          i2c_master_dev_handle_t *ret_handle

) {
    return add_device_to_bus(i2c_bus, dev_config, ret_handle);
}

esp_err_t I2C::add_device_to_bus(i2c_master_bus_handle_t bus_handle,
                                 const i2c_device_config_t *dev_config,
                                 i2c_master_dev_handle_t *ret_handle

) {
    return i2c_master_bus_add_device(bus_handle, dev_config, ret_handle);
}

esp_err_t I2C::probe_addr_in_bus(i2c_master_bus_handle_t bus_handle,
                                 uint16_t addr, int timeout_ms

) {
    return i2c_master_probe(bus_handle, addr, timeout_ms);
}

esp_err_t I2C::probe_addr(uint16_t addr, int timeout_ms

) {
    return probe_addr_in_bus(i2c_bus, addr, timeout_ms);
}
