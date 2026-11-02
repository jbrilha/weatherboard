#include "ble.hpp"
#include "esp_event.h"
#include "esp_hosted_bt_host_stack.h"
#include "esp_log.h"
#include "host/ble_hs.h"
#include "host/util/util.h"
#include "nimble/nimble_port.h"
#include "nimble/nimble_port_freertos.h"
#include "services/gap/ble_svc_gap.h"

#include "esp_central.h"
#include "weather_mqtt.hpp"
#include "weatherboard.hpp"

#include <cstdlib>
#include <cstring>

static constexpr char TAG[] = "BLE";
namespace {

static int blecent_gap_event(struct ble_gap_event *event, void *arg);

ble_uuid128_t to_ble_uuid(wb::uuid128_t uuid) {
    ble_uuid128_t out{};
    out.u.type = BLE_UUID_TYPE_128;
    memcpy(out.value, uuid.bytes, sizeof out.value);
    return out;
}

const ble_uuid128_t remote_svc_uuid =
    to_ble_uuid(wb::display_service_uuid_le());
const ble_uuid128_t remote_chr_uuid =
    to_ble_uuid(wb::weather_payload_uuid_le());
const ble_uuid16_t cccd_uuid = BLE_UUID16_INIT(BLE_GATT_DSC_CLT_CFG_UUID16);

WeatherMQTT *weather_source = nullptr;

int blecent_should_connect(const struct ble_gap_disc_desc *disc) {
    struct ble_hs_adv_fields fields;
    int rc;
    uint32_t peer_addr[6];

    memset(peer_addr, 0x0, sizeof peer_addr);

    /* The device has to be advertising connectability. */
    if (disc->event_type != BLE_HCI_ADV_RPT_EVTYPE_ADV_IND &&
        disc->event_type != BLE_HCI_ADV_RPT_EVTYPE_DIR_IND) {

        return 0;
    }

    rc = ble_hs_adv_parse_fields(&fields, disc->data, disc->length_data);
    if (rc != 0) {
        return 0;
    }

    for (int i = 0; i < fields.num_uuids128; i++) {
        if (ble_uuid_cmp(&fields.uuids128[i].u, &remote_svc_uuid.u) == 0) {
            return 1;
        }
    }

    return 0;
}

void blecent_connect_if_interesting(void *disc) {
    uint8_t own_addr_type;
    int rc;
    ble_addr_t *addr;

    /* Don't do anything if we don't care about this advertiser. */
    if (!blecent_should_connect((struct ble_gap_disc_desc *)disc)) {
        return;
    }

#if !(MYNEWT_VAL(BLE_HOST_ALLOW_CONNECT_WITH_SCAN))
    /* Scanning must be stopped before a connection can be initiated. */
    rc = ble_gap_disc_cancel();
    if (rc != 0) {
        MODLOG_DFLT(DEBUG, "Failed to cancel scan; rc=%d\n", rc);
        return;
    }
#endif

    /* Figure out address to use for connect (no privacy for now) */
    rc = ble_hs_id_infer_auto(0, &own_addr_type);
    if (rc != 0) {
        MODLOG_DFLT(ERROR, "error determining address type; rc=%d\n", rc);
        return;
    }

    /* Try to connect the the advertiser.  Allow 30 seconds (30000 ms) for
     * timeout.
     */
    addr = &((struct ble_gap_disc_desc *)disc)->addr;

    rc = ble_gap_connect(own_addr_type, addr, 30000, NULL, blecent_gap_event,
                         NULL);
    if (rc != 0) {
        MODLOG_DFLT(ERROR,
                    "Error: Failed to connect to device; addr_type=%d "
                    "addr=%s; rc=%d\n",
                    addr->type, addr_str(addr->val), rc);
        return;
    }
}

int blecent_on_custom_read(uint16_t conn_handle,
                           const struct ble_gatt_error *error,
                           struct ble_gatt_attr *attr, void *arg) {
    MODLOG_DFLT(INFO,
                "Read complete for the subscribable characteristic; "
                "status=%d conn_handle=%d",
                error->status, conn_handle);
    if (error->status == 0) {
        MODLOG_DFLT(INFO, " attr_handle=%d value=", attr->handle);
        print_mbuf(attr->om);
    }
    MODLOG_DFLT(INFO, "\n");

    return 0;
}

/**
 * Application Callback. Called when the custom subscribable characteristic
 * in the remote GATT server is written to.
 * Client has previously subscribed to this characeteristic,
 * so expect a notification from the server.
 **/
int blecent_on_custom_write(uint16_t conn_handle,
                            const struct ble_gatt_error *error,
                            struct ble_gatt_attr *attr, void *arg) {
    const struct peer_chr *chr;
    const struct peer *peer;
    int rc;

    MODLOG_DFLT(INFO,
                "Write to the custom subscribable characteristic complete; "
                "status=%d conn_handle=%d attr_handle=%d\n",
                error->status, conn_handle, attr->handle);

    peer = peer_find(conn_handle);
    if (peer == NULL) {
        MODLOG_DFLT(WARN,
                    "Peer not found (conn_handle=%d), likely disconnected\n",
                    conn_handle);
        return 0;
    }
    chr = peer_chr_find_uuid(peer, &remote_svc_uuid.u, &remote_chr_uuid.u);
    if (chr == NULL) {
        MODLOG_DFLT(ERROR, "Error: Peer doesn't have the custom subscribable "
                           "characteristic\n");
        goto err;
    }

    /*** Performs a read on the characteristic, the result is handled in
     * blecent_on_new_read callback ***/
    rc = ble_gattc_read(conn_handle, chr->chr.val_handle,
                        blecent_on_custom_read, NULL);
    if (rc != 0) {
        MODLOG_DFLT(
            ERROR,
            "Error: Failed to read the custom subscribable characteristic; "
            "rc=%d\n",
            rc);
        goto err;
    }

    return 0;
err:
    /* Terminate the connection */
    return ble_gap_terminate(peer->conn_handle, BLE_ERR_REM_USER_CONN_TERM);
}

int blecent_on_mtu(uint16_t conn_handle, const struct ble_gatt_error *error,
                   uint16_t mtu, void *arg) {
    uint16_t val_handle = (uint16_t)(uintptr_t)arg;
    uint8_t payload[WB_MAX_PAYLOAD_LEN];
    size_t len;
    int rc;

    MODLOG_DFLT(INFO,
                "MTU exchange complete; status=%d conn_handle=%d mtu=%d\n",
                error->status, conn_handle, mtu);

    len = weather_source->get_latest_payload(payload, sizeof payload);
    if (len == 0) {
        MODLOG_DFLT(WARN, "No weather payload to send yet\n");
        return 0;
    }
    if (len > (size_t)(ble_att_mtu(conn_handle) - 3)) {
        MODLOG_DFLT(ERROR, "Weather payload (%u bytes) exceeds MTU %u\n",
                    (unsigned)len, ble_att_mtu(conn_handle));
        return ble_gap_terminate(conn_handle, BLE_ERR_REM_USER_CONN_TERM);
    }

    rc = ble_gattc_write_flat(conn_handle, val_handle, payload, len,
                              blecent_on_custom_write, NULL);
    if (rc != 0) {
        MODLOG_DFLT(ERROR, "Error: Failed to write weather payload; rc=%d\n",
                    rc);
        return ble_gap_terminate(conn_handle, BLE_ERR_REM_USER_CONN_TERM);
    }

    return 0;
}

/**
 * Application Callback. Called when the custom subscribable characteristic
 * is subscribed to.
 **/
int blecent_on_custom_subscribe(uint16_t conn_handle,
                                const struct ble_gatt_error *error,
                                struct ble_gatt_attr *attr, void *arg) {
    const struct peer_chr *chr;
    int rc;
    const struct peer *peer;

    MODLOG_DFLT(INFO,
                "Subscribe to the custom subscribable characteristic complete; "
                "status=%d conn_handle=%d",
                error->status, conn_handle);

    if (error->status == 0) {
        MODLOG_DFLT(INFO, " attr_handle=%d value=", attr->handle);
        print_mbuf(attr->om);
    }
    MODLOG_DFLT(INFO, "\n");

    peer = peer_find(conn_handle);
    chr = peer_chr_find_uuid(peer, &remote_svc_uuid.u, &remote_chr_uuid.u);
    if (chr == NULL) {
        MODLOG_DFLT(
            ERROR,
            "Error: Peer doesn't have the subscribable characteristic\n");
        goto err;
    }

    rc = ble_gattc_exchange_mtu(conn_handle, blecent_on_mtu,
                                (void *)(uintptr_t)chr->chr.val_handle);
    if (rc != 0) {
        MODLOG_DFLT(ERROR, "Error: Failed to exchange MTU; rc=%d\n", rc);
        goto err;
    }

    return 0;
err:
    /* Terminate the connection */
    return ble_gap_terminate(peer->conn_handle, BLE_ERR_REM_USER_CONN_TERM);
}

void blecent_custom_gatt_operations(const struct peer *peer) {
    const struct peer_dsc *dsc;
    int rc;
    uint8_t value[2];

    dsc = peer_dsc_find_uuid(peer, &remote_svc_uuid.u, &remote_chr_uuid.u,
                             &cccd_uuid.u);
    if (dsc == NULL) {
        MODLOG_DFLT(
            ERROR,
            "Error: Peer lacks a CCCD for the subscribable characteristic\n");
        goto err;
    }

    /*** Write 0x00 and 0x01 (The subscription code) to the CCCD ***/
    value[0] = 1;
    value[1] = 0;
    rc = ble_gattc_write_flat(peer->conn_handle, dsc->dsc.handle, value,
                              sizeof(value), blecent_on_custom_subscribe, NULL);
    if (rc != 0) {
        MODLOG_DFLT(
            ERROR,
            "Error: Failed to subscribe to the subscribable characteristic; "
            "rc=%d\n",
            rc);
        goto err;
    }

    return;
err:
    /* Terminate the connection */
    ble_gap_terminate(peer->conn_handle, BLE_ERR_REM_USER_CONN_TERM);
}

void blecent_on_disc_complete(const struct peer *peer, int status, void *arg) {

    if (status != 0) {
        /* Service discovery failed.  Terminate the connection. */
        MODLOG_DFLT(ERROR,
                    "Error: Service discovery failed; status=%d "
                    "conn_handle=%d\n",
                    status, peer->conn_handle);
        ble_gap_terminate(peer->conn_handle, BLE_ERR_REM_USER_CONN_TERM);
        return;
    }

    /* Service discovery has completed successfully.  Now we have a complete
     * list of services, characteristics, and descriptors that the peer
     * supports.
     */
    MODLOG_DFLT(INFO,
                "Service discovery complete; status=%d "
                "conn_handle=%d\n",
                status, peer->conn_handle);

    blecent_custom_gatt_operations(peer);
}

void blecent_scan(void) {
    uint8_t own_addr_type;
    struct ble_gap_disc_params disc_params = {};
    int rc;

    /* Figure out address to use while advertising (no privacy for now) */
    rc = ble_hs_id_infer_auto(0, &own_addr_type);
    if (rc != 0) {
        MODLOG_DFLT(ERROR, "error determining address type; rc=%d\n", rc);
        return;
    }

    /* Tell the controller to filter duplicates; we don't want to process
     * repeated advertisements from the same device.
     */
    disc_params.filter_duplicates = 1;

    /**
     * Perform a passive scan.  I.e., don't send follow-up scan requests to
     * each advertiser.
     */
    disc_params.passive = 1;

    /* Use defaults for the rest of the parameters. */
    disc_params.itvl = 0;
    disc_params.window = 0;
    disc_params.filter_policy = 0;
    disc_params.limited = 0;

    rc = ble_gap_disc(own_addr_type, BLE_HS_FOREVER, &disc_params,
                      blecent_gap_event, NULL);
    if (rc != 0) {
        MODLOG_DFLT(ERROR, "Error initiating GAP discovery procedure; rc=%d\n",
                    rc);
    }
}

int blecent_gap_event(struct ble_gap_event *event, void *arg) {
#if NIMBLE_BLE_CONNECT
    struct ble_gap_conn_desc desc;
#endif
    struct ble_hs_adv_fields fields;
    int rc;

    switch (event->type) {
    case BLE_GAP_EVENT_DISC:
        rc = ble_hs_adv_parse_fields(&fields, event->disc.data,
                                     event->disc.length_data);
        if (rc != 0) {
            return 0;
        }

        /* An advertisement report was received during GAP discovery. */
        print_adv_fields(&fields);

        /* Try to connect to the advertiser if it looks interesting. */
        blecent_connect_if_interesting(&event->disc);
        return 0;
#if NIMBLE_BLE_CONNECT
    case BLE_GAP_EVENT_CONNECT:
        /* A new connection was established or a connection attempt failed. */
        if (event->connect.status == 0) {
            /* Connection successfully established. */
            MODLOG_DFLT(INFO, "Connection established ");

            rc = ble_gap_conn_find(event->connect.conn_handle, &desc);
            assert(rc == 0);
            print_conn_desc(&desc);
            MODLOG_DFLT(INFO, "\n");

            /* Remember peer. */
            rc = peer_add(event->connect.conn_handle);
            if (rc != 0) {
                MODLOG_DFLT(ERROR, "Failed to add peer; rc=%d\n", rc);
                return 0;
            }

#if MYNEWT_VAL(BLE_POWER_CONTROL)
            blecent_power_control(event->connect.conn_handle);
#endif

#if CONFIG_EXAMPLE_ENCRYPTION
            /** Initiate security - It will perform
             * Pairing (Exchange keys)
             * Bonding (Store keys)
             * Encryption (Enable encryption)
             * Will invoke event BLE_GAP_EVENT_ENC_CHANGE
             **/
            rc = ble_gap_security_initiate(event->connect.conn_handle);
            if (rc != 0) {
                MODLOG_DFLT(INFO, "Security could not be initiated, rc = %d\n",
                            rc);
                return ble_gap_terminate(event->connect.conn_handle,
                                         BLE_ERR_REM_USER_CONN_TERM);
            } else {
                MODLOG_DFLT(INFO, "Connection secured\n");
            }
#else
#if MYNEWT_VAL(BLE_GATTC)
#if MYNEWT_VAL(BLE_GATT_CACHING_ASSOC_ENABLE)
            rc = ble_gattc_cache_assoc(desc.peer_id_addr);
            if (rc != 0) {
                MODLOG_DFLT(ERROR, "Cache Association Failed; rc=%d\n", rc);
                return 0;
            }
#else
            /* Perform service discovery */
            rc = peer_disc_all(event->connect.conn_handle,
                               blecent_on_disc_complete, NULL);
            if (rc != 0) {
                MODLOG_DFLT(ERROR, "Failed to discover services; rc=%d\n", rc);
                return 0;
            }
#endif // BLE_GATT_CACHING_ASSOC_ENABLE
#endif // BLE_GATTC
#endif // EXAMPLE_ENCRYPTION
        } else {
            /* Connection attempt failed; resume scanning. */
            MODLOG_DFLT(ERROR, "Error: Connection failed; status=%d\n",
                        event->connect.status);
            blecent_scan();
        }

        return 0;

    case BLE_GAP_EVENT_DISCONNECT:
        /* Connection terminated. */
        MODLOG_DFLT(INFO, "disconnect; reason=%d ", event->disconnect.reason);
        print_conn_desc(&event->disconnect.conn);
        MODLOG_DFLT(INFO, "\n");

        /* Forget about peer. */
        peer_delete(event->disconnect.conn.conn_handle);

        /* Resume scanning. */
        blecent_scan();
        return 0;

    case BLE_GAP_EVENT_DISC_COMPLETE:
        MODLOG_DFLT(INFO, "discovery complete; reason=%d\n",
                    event->disc_complete.reason);
        return 0;

    case BLE_GAP_EVENT_ENC_CHANGE:
        /* Encryption has been enabled or disabled for this connection. */
        MODLOG_DFLT(INFO, "encryption change event; status=%d ",
                    event->enc_change.status);
        rc = ble_gap_conn_find(event->enc_change.conn_handle, &desc);
        assert(rc == 0);
        print_conn_desc(&desc);
#if CONFIG_EXAMPLE_ENCRYPTION && MYNEWT_VAL(BLE_GATTC)
#if MYNEWT_VAL(BLE_GATT_CACHING_ASSOC_ENABLE)
        rc = ble_gattc_cache_assoc(desc.peer_id_addr);
        if (rc != 0) {
            MODLOG_DFLT(ERROR, "Cache Association Failed; rc=%d\n", rc);
            return 0;
        }
#else
        /*** Go for service discovery after encryption has been successfully
         * enabled ***/
        rc = peer_disc_all(event->enc_change.conn_handle,
                           blecent_on_disc_complete, NULL);
        if (rc != 0) {
            MODLOG_DFLT(ERROR, "Failed to discover services; rc=%d\n", rc);
            return 0;
        }
#endif // BLE_GATT_CACHING_ASSOC_ENABLE
#endif // EXAMPLE_ENCRYPTION
        return 0;

    case BLE_GAP_EVENT_CACHE_ASSOC:
#if MYNEWT_VAL(BLE_GATT_CACHING_ASSOC_ENABLE)
        /* Cache association result for this connection */
        MODLOG_DFLT(
            INFO,
            "cache association; conn_handle=%d status=%d cache_state=%s\n",
            event->cache_assoc.conn_handle, event->cache_assoc.status,
            (event->cache_assoc.cache_state == 0) ? "INVALID" : "LOADED");
        /* Perform service discovery */
        rc = peer_disc_all(event->cache_assoc.conn_handle,
                           blecent_on_disc_complete, NULL);
        if (rc != 0) {
            MODLOG_DFLT(ERROR, "Failed to discover services; rc=%d\n", rc);
            return 0;
        }
#endif
        return 0;

    case BLE_GAP_EVENT_NOTIFY_RX:
        /* Peer sent us a notification or indication. */
        MODLOG_DFLT(INFO,
                    "received %s; conn_handle=%d attr_handle=%d "
                    "attr_len=%d\n",
                    event->notify_rx.indication ? "indication" : "notification",
                    event->notify_rx.conn_handle, event->notify_rx.attr_handle,
                    OS_MBUF_PKTLEN(event->notify_rx.om));

        /* Attribute data is contained in event->notify_rx.om. Use
         * `os_mbuf_copydata` to copy the data received in notification mbuf */
        return 0;

    case BLE_GAP_EVENT_MTU:
        MODLOG_DFLT(INFO, "mtu update event; conn_handle=%d cid=%d mtu=%d\n",
                    event->mtu.conn_handle, event->mtu.channel_id,
                    event->mtu.value);
        return 0;

    case BLE_GAP_EVENT_REPEAT_PAIRING:
        /* We already have a bond with the peer, but it is attempting to
         * establish a new secure link.  This app sacrifices security for
         * convenience: just throw away the old bond and accept the new link.
         */

        /* Delete the old bond. */
        rc = ble_gap_conn_find(event->repeat_pairing.conn_handle, &desc);
        assert(rc == 0);
        ble_store_util_delete_peer(&desc.peer_id_addr);

        /* Return BLE_GAP_REPEAT_PAIRING_RETRY to indicate that the host should
         * continue with the pairing operation.
         */
        return BLE_GAP_REPEAT_PAIRING_RETRY;

        return 0;

#endif
    default:
        return 0;
    }
}

void bridge_ble_on_sync(void) {
    ESP_LOGI(TAG, "host synced");

    int rc;

    /* Make sure we have proper identity address set (public preferred) */
    rc = ble_hs_util_ensure_addr(0);
    assert(rc == 0);

    /* Begin scanning for a peripheral to connect to. */
    blecent_scan();
}

void bridge_ble_on_reset(int reason) {
    ESP_LOGW(TAG, "host reset (reason=%d)", reason);
}

void bridge_ble_host_task(void *arg) {
    nimble_port_run();
    nimble_port_freertos_deinit();
}

} // namespace

BLE::BLE() : bt_config{} {
    bt_config = (esp_hosted_bt_host_stack_cfg_t){
        .stack = ESP_HOSTED_BT_HOST_STACK_NIMBLE,
        .bt_mac = __null,
        .bring_up_controller = true,
        .controller_ready_timeout_ms = 5000U,
        .custom = {},
    };
}

BLE::~BLE() {}

esp_err_t BLE::init(WeatherMQTT *weather) {
    weather_source = weather;
    ESP_ERROR_CHECK(esp_hosted_bt_host_stack_setup(&bt_config));
    ESP_ERROR_CHECK(nimble_port_init());
    ble_hs_cfg.sync_cb = bridge_ble_on_sync;
    ble_hs_cfg.reset_cb = bridge_ble_on_reset;

    int rc;
    rc = peer_init(MYNEWT_VAL(BLE_MAX_CONNECTIONS), 64, 64, 64);
    assert(rc == 0);

    int m;
    /* Set the default device name. */
    m = ble_svc_gap_device_name_set("weatherbridge");
    assert(m == 0);

    nimble_port_freertos_init(bridge_ble_host_task);
    return ESP_OK;
}
