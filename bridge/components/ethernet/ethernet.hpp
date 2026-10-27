#pragma once

#include "esp_err.h"
#include "esp_eth.h"
#include "esp_netif_types.h"

class Ethernet {
  public:
    Ethernet();
    ~Ethernet();

    esp_err_t init(void);

  private:
    esp_eth_handle_t handle;
    esp_eth_mac_t *mac;
    esp_eth_phy_t *phy;
    esp_netif_t *netif;
    esp_eth_netif_glue_handle_t netif_glue;
    esp_event_handler_instance_t eth_evt_inst;
    esp_event_handler_instance_t ip_evt_inst;

    esp_eth_handle_t init_internal(void);
};
