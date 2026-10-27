#include "ethernet.hpp"
#include "esp_eth_driver.h"
#include "esp_eth_phy_ip101.h"
#include "esp_event.h"
#include "esp_log.h"
#include "esp_netif.h"

static constexpr char TAG[] = "Ethernet";

static void eth_event_handler(void *arg, esp_event_base_t event_base,
                              int32_t event_id, void *event_data) {
    uint8_t mac_addr[6] = {0};
    esp_eth_handle_t eth_handle = *(esp_eth_handle_t *)event_data;

    switch (event_id) {
    case ETHERNET_EVENT_CONNECTED:
        esp_eth_ioctl(eth_handle, ETH_CMD_G_MAC_ADDR, mac_addr);
        ESP_LOGI(TAG, "Ethernet Link Up");
        ESP_LOGI(TAG, "Ethernet HW Addr %02x:%02x:%02x:%02x:%02x:%02x",
                 mac_addr[0], mac_addr[1], mac_addr[2], mac_addr[3],
                 mac_addr[4], mac_addr[5]);
        break;
    case ETHERNET_EVENT_DISCONNECTED:
        ESP_LOGW(TAG, "Ethernet Link Down");
        break;
    case ETHERNET_EVENT_START:
        ESP_LOGI(TAG, "Ethernet Started");
        break;
    case ETHERNET_EVENT_STOP:
        ESP_LOGW(TAG, "Ethernet Stopped");
        break;
    default:
        break;
    }
}

static void got_ip_event_handler(void *arg, esp_event_base_t event_base,
                                 int32_t event_id, void *event_data) {
    ip_event_got_ip_t *event = (ip_event_got_ip_t *)event_data;
    const esp_netif_ip_info_t *ip_info = &event->ip_info;

    ESP_LOGI(TAG, "Ethernet Got IP Address");
    ESP_LOGI(TAG, "~~~~~~~~~~~");
    ESP_LOGI(TAG, "ETHIP:" IPSTR, IP2STR(&ip_info->ip));
    ESP_LOGI(TAG, "ETHMASK:" IPSTR, IP2STR(&ip_info->netmask));
    ESP_LOGI(TAG, "ETHGW:" IPSTR, IP2STR(&ip_info->gw));
    ESP_LOGI(TAG, "~~~~~~~~~~~");
}

Ethernet::Ethernet()
    : handle(nullptr), mac(nullptr), phy(nullptr), netif(nullptr),
      netif_glue(nullptr), eth_evt_inst(nullptr), ip_evt_inst(nullptr) {}

Ethernet::~Ethernet() {
    if (handle != nullptr) {
        esp_eth_stop(handle);
        esp_eth_del_netif_glue(netif_glue);
        esp_netif_destroy(netif);

        if (esp_eth_driver_uninstall(handle) == ESP_OK) {
            mac->del(mac);
            phy->del(phy);
        }
    }

    // TODO unregistering after the uninstall might lead to a use after free but
    // for now I want to see the stopped logs
    esp_event_handler_instance_unregister(ETH_EVENT, ESP_EVENT_ANY_ID,
                                          eth_evt_inst);
    esp_event_handler_instance_unregister(IP_EVENT, IP_EVENT_ETH_GOT_IP,
                                          ip_evt_inst);
}

esp_err_t Ethernet::init(void) {
    this->handle = init_internal();
    if (this->handle == nullptr) {
        return ESP_ERR_INVALID_STATE;
    }

    ESP_ERROR_CHECK(esp_netif_init());
    ESP_ERROR_CHECK(esp_event_loop_create_default());

    esp_netif_config_t cfg = ESP_NETIF_DEFAULT_ETH();
    this->netif = esp_netif_new(&cfg);
    this->netif_glue = esp_eth_new_netif_glue(this->handle);
    ESP_ERROR_CHECK(esp_netif_attach(this->netif, this->netif_glue));

    ESP_ERROR_CHECK(esp_event_handler_instance_register(
        ETH_EVENT, ESP_EVENT_ANY_ID, &eth_event_handler, nullptr,
        &eth_evt_inst));
    ESP_ERROR_CHECK(esp_event_handler_instance_register(
        IP_EVENT, IP_EVENT_ETH_GOT_IP, &got_ip_event_handler, nullptr,
        &ip_evt_inst));

    ESP_ERROR_CHECK(esp_eth_start(this->handle));

    return ESP_OK;
}

esp_eth_handle_t Ethernet::init_internal(void) {
    eth_mac_config_t mac_config = ETH_MAC_DEFAULT_CONFIG();
    eth_phy_config_t phy_config = ETH_PHY_DEFAULT_CONFIG();

    phy_config.phy_addr = 1;
    phy_config.reset_gpio_num = 51;
    eth_esp32_emac_config_t esp32_emac_config = ETH_ESP32_EMAC_DEFAULT_CONFIG();
    esp32_emac_config.smi_gpio.mdc_num = 31;
    esp32_emac_config.smi_gpio.mdio_num = 52;

    this->mac = esp_eth_mac_new_esp32(&esp32_emac_config, &mac_config);
    this->phy = esp_eth_phy_new_ip101(&phy_config);

    esp_eth_handle_t eth_handle = nullptr;
    esp_eth_config_t config = ETH_DEFAULT_CONFIG(this->mac, this->phy);
    if (esp_eth_driver_install(&config, &eth_handle) != ESP_OK) {
        if (eth_handle != nullptr) {
            esp_eth_driver_uninstall(eth_handle);
        }
        if (mac != nullptr) {
            mac->del(mac);
        }
        if (phy != nullptr) {
            phy->del(phy);
        }

        return nullptr;
    }

    return eth_handle;
}
