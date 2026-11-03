#include "system.hpp"
#include "bridge_ota.hpp"
#include "esp_attr.h"
#include "esp_core_dump.h"
#include "esp_event.h"
#include "esp_log.h"
#include "esp_netif.h"
#include "esp_ota_ops.h"
#include "esp_timer.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "mdns.h"
#include "nvs.h"
#include "nvs_flash.h"
#include <inttypes.h>
#include <string.h>

static constexpr char TAG[] = "SYSTEM";
static constexpr char MDNS_HOSTNAME[] = "bridge";

constexpr int NETIF_ATTEMPTS = 3;
constexpr uint32_t NETIF_BACKOFF_MS = 100;
constexpr uint64_t OTA_VALIDATE_DELAY_US = 60ULL * 1000 * 1000;
constexpr uint64_t OTA_VALIDATE_DELAY_S = OTA_VALIDATE_DELAY_US / 1000000;
constexpr uint32_t SAFE_MODE_CLEAN_RUN_S = 600;
constexpr uint8_t SAFE_MODE_BOOT_THRESHOLD = 5;
constexpr uint32_t RTC_BOOT_MAGIC = 0xB007C0DE;

const char *system_reset_reason_str(esp_reset_reason_t reason) {
    switch (reason) {
    case ESP_RST_POWERON:
        return "power-on";
    case ESP_RST_EXT:
        return "external pin";
    case ESP_RST_SW:
        return "software";
    case ESP_RST_PANIC:
        return "panic";
    case ESP_RST_INT_WDT:
        return "interrupt watchdog";
    case ESP_RST_TASK_WDT:
        return "task watchdog";
    case ESP_RST_WDT:
        return "other watchdog";
    case ESP_RST_DEEPSLEEP:
        return "deep sleep wake";
    case ESP_RST_BROWNOUT:
        return "brownout";
    case ESP_RST_SDIO:
        return "sdio";
    default:
        return "unknown";
    }
}

namespace {
system_status_t status = {};
bool initialized = false;
system_health_fn health_check = nullptr;
esp_timer_handle_t validate_timer = nullptr;

bool reset_was_abnormal(esp_reset_reason_t r) {
    return r == ESP_RST_PANIC || r == ESP_RST_INT_WDT ||
           r == ESP_RST_TASK_WDT || r == ESP_RST_WDT || r == ESP_RST_BROWNOUT;
}

const char *BOOT_NS = "bridge_boot";
const char *BOOT_KEY = "attempts";

RTC_NOINIT_ATTR uint32_t rtc_boot_attempts;
RTC_NOINIT_ATTR uint32_t rtc_boot_magic;

esp_timer_handle_t clean_run_timer = nullptr;

uint32_t boot_attempts_load(void) {
    uint32_t stored = 0;
    nvs_handle_t h;
    if (status.nvs_ok && nvs_open(BOOT_NS, NVS_READONLY, &h) == ESP_OK) {
        nvs_get_u32(h, BOOT_KEY, &stored);
        nvs_close(h);
    }
    uint32_t rtc = rtc_boot_magic == RTC_BOOT_MAGIC ? rtc_boot_attempts : 0;
    return rtc > stored ? rtc : stored;
}

void boot_attempts_store(uint32_t n) {
    rtc_boot_attempts = n;
    rtc_boot_magic = RTC_BOOT_MAGIC;

    if (!status.nvs_ok) {
        return;
    }
    nvs_handle_t h;
    if (nvs_open(BOOT_NS, NVS_READWRITE, &h) != ESP_OK) {
        return;
    }
    nvs_set_u32(h, BOOT_KEY, n);
    nvs_commit(h);
    nvs_close(h);
}

void clean_run_cb(void *arg) {
    (void)arg;
    if (status.boot_attempts == 0) {
        return;
    }
    boot_attempts_store(0);
    status.boot_attempts = 0;
    ESP_LOGW(TAG, "ran for %" PRIu32 " s, boot counter reset",
             SAFE_MODE_CLEAN_RUN_S);
}

void arm_clean_run_timer(void) {
    const esp_timer_create_args_t args = {
        .callback = clean_run_cb,
        .arg = nullptr,
        .dispatch_method = ESP_TIMER_TASK,
        .name = "clean_run",
        .skip_unhandled_events = false,
    };
    if (esp_timer_create(&args, &clean_run_timer) != ESP_OK ||
        esp_timer_start_once(clean_run_timer, (uint64_t)SAFE_MODE_CLEAN_RUN_S *
                                                  1000000ULL) != ESP_OK) {
        ESP_LOGE(TAG, "could not arm the clean-run timer, clearing now");
        boot_attempts_store(0);
        status.boot_attempts = 0;
    }
}

void count_boot(void) {
    const bool abnormal = reset_was_abnormal(status.reset_reason);
    uint32_t attempts = boot_attempts_load();

    if (system_ota_is_pending_verify() && attempts > 0) {
        ESP_LOGW(TAG, "new image, boot counter reset from %" PRIu32, attempts);
        attempts = 0;
        boot_attempts_store(0);
    } else if (abnormal) {
        attempts++;
        boot_attempts_store(attempts);
    }
    status.boot_attempts = attempts;

    if (attempts >= SAFE_MODE_BOOT_THRESHOLD) {
        status.safe_mode = true;
        ESP_LOGE(TAG, "SAFE MODE: %" PRIu32 " abnormal boots (threshold %d)",
                 attempts, SAFE_MODE_BOOT_THRESHOLD);
        return;
    }

    if (abnormal) {
        ESP_LOGW(TAG, "abnormal boot %" PRIu32 " of %d before safe mode",
                 attempts, SAFE_MODE_BOOT_THRESHOLD);
    } else if (attempts > 0) {
        ESP_LOGW(TAG, "clean reset, holding at %" PRIu32 " abnormal boots",
                 attempts);
    }

    arm_clean_run_timer();
}

bool init_nvs(void) {
    esp_err_t ret = nvs_flash_init();
    if (ret == ESP_ERR_NVS_NO_FREE_PAGES ||
        ret == ESP_ERR_NVS_NEW_VERSION_FOUND) {
        ESP_LOGW(TAG, "nvs needs erase (%s)", esp_err_to_name(ret));
        esp_err_t erase = nvs_flash_erase();
        if (erase != ESP_OK) {
            ESP_LOGE(TAG, "nvs erase failed (%s), running without nvs",
                     esp_err_to_name(erase));
            return false;
        }
        status.nvs_erased = true;
        ret = nvs_flash_init();
    }
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "nvs init failed (%s), running without nvs",
                 esp_err_to_name(ret));
        return false;
    }
    return true;
}

bool init_netif(void) {
    uint32_t backoff = NETIF_BACKOFF_MS;
    for (int i = 0; i < NETIF_ATTEMPTS; i++) {
        esp_err_t ret = esp_netif_init();
        if (ret == ESP_OK) {
            return true;
        }
        ESP_LOGW(TAG, "netif init %d/%d failed (%s)", i + 1, NETIF_ATTEMPTS,
                 esp_err_to_name(ret));
        vTaskDelay(pdMS_TO_TICKS(backoff));
        backoff *= 2;
    }
    ESP_LOGE(TAG, "netif init failed, no ip networking on this boot");
    return false;
}

bool init_mdns(void) {
    esp_err_t ret = mdns_init();
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "mdns init failed (%s), .local names won't resolve",
                 esp_err_to_name(ret));
        return false;
    }
    ret = mdns_hostname_set(MDNS_HOSTNAME);
    if (ret != ESP_OK) {
        ESP_LOGW(TAG, "mdns hostname set failed (%s)", esp_err_to_name(ret));
    }
    return true;
}

const char *ok_str(bool ok) { return ok ? "ok" : "FAIL"; }

esp_core_dump_summary_t cd_summary;
bool cd_summary_valid = false;

void check_coredump(void) {
    if (esp_core_dump_image_check() != ESP_OK) {
        return;
    }
    status.coredump_present = true;

    if (esp_core_dump_get_summary(&cd_summary) != ESP_OK) {
        ESP_LOGE(TAG, "core dump present, but its summary is unreadable");
        return;
    }

    cd_summary_valid = true;
    ESP_LOGE(TAG, "core dump present: task=%s pc=0x%08" PRIx32,
             cd_summary.exc_task, cd_summary.exc_pc);
}

void validate_cb(void *arg) {
    (void)arg;
    bool healthy = (health_check != nullptr) ? health_check() : true;
    if (healthy) {
        system_ota_mark_valid();
        return;
    }
    ESP_LOGE(TAG, "health check failed, rolling back to the previous image");
    esp_ota_mark_app_invalid_rollback_and_reboot();
}

void arm_ota_validation(void) {
    if (!system_ota_is_pending_verify()) {
        return;
    }

    ESP_LOGW(TAG, "image is PENDING_VERIFY, health check in %llu s",
             OTA_VALIDATE_DELAY_S);

    const esp_timer_create_args_t args = {
        .callback = validate_cb,
        .arg = nullptr,
        .dispatch_method = ESP_TIMER_TASK,
        .name = "ota_validate",
        .skip_unhandled_events = false,
    };
    if (esp_timer_create(&args, &validate_timer) != ESP_OK ||
        esp_timer_start_once(validate_timer, OTA_VALIDATE_DELAY_US) != ESP_OK) {
        ESP_LOGE(TAG, "could not arm validation timer, marking valid now");
        system_ota_mark_valid();
    }
}

} // namespace

bool system_coredump_get(system_coredump_t *out) {
    if (out == nullptr) {
        return false;
    }

    memset(out, 0, sizeof(*out));
    out->present = status.coredump_present;
    if (!cd_summary_valid) {
        return out->present;
    }

    strlcpy(out->task, cd_summary.exc_task, sizeof(out->task));
    out->pc = cd_summary.exc_pc;
    return true;
}

esp_err_t system_coredump_clear(void) {
    esp_err_t ret = esp_core_dump_image_erase();
    if (ret == ESP_OK) {
        status.coredump_present = false;
        cd_summary_valid = false;
        ESP_LOGW(TAG, "core dump erased");
    } else {
        ESP_LOGE(TAG, "core dump erase failed (%s)", esp_err_to_name(ret));
    }
    return ret;
}

void system_mark_clean_run(void) {
    if (status.boot_attempts == 0) {
        return;
    }
    boot_attempts_store(0);
    status.boot_attempts = 0;
    ESP_LOGW(TAG, "clean run confirmed early, boot counter reset");
}

bool system_in_safe_mode(void) { return status.safe_mode; }

uint32_t system_boot_attempts(void) { return status.boot_attempts; }

bool system_ota_is_pending_verify(void) {
    const esp_partition_t *running = esp_ota_get_running_partition();
    esp_ota_img_states_t state;
    if ((running == nullptr) ||
        esp_ota_get_state_partition(running, &state) != ESP_OK) {
        return false;
    }
    return state == ESP_OTA_IMG_PENDING_VERIFY;
}

void system_ota_set_health_check(system_health_fn fn) { health_check = fn; }

esp_err_t system_ota_mark_valid(void) {
    esp_err_t ret = esp_ota_mark_app_valid_cancel_rollback();
    if (ret == ESP_OK) {
        ESP_LOGW(TAG, "image marked valid, rollback cancelled");
    } else {
        ESP_LOGE(TAG, "mark app valid failed (%s)", esp_err_to_name(ret));
    }
    return ret;
}

system_status_t system_init(void) {
    if (initialized) {
        return status;
    }

    status.reset_reason = esp_reset_reason();
    ESP_LOGW(TAG, "boot: reset reason = %s",
             system_reset_reason_str(status.reset_reason));
    if (reset_was_abnormal(status.reset_reason)) {
        ESP_LOGE(TAG, "previous run ended abnormally (%s)",
                 system_reset_reason_str(status.reset_reason));
    }

    check_coredump();

    status.nvs_ok = init_nvs();

    count_boot();

    status.netif_ok = init_netif();

    esp_err_t ret = esp_event_loop_create_default();
    status.event_loop_ok = (ret == ESP_OK || ret == ESP_ERR_INVALID_STATE);
    if (!status.event_loop_ok) {
        ESP_LOGE(TAG, "event loop create failed (%s)", esp_err_to_name(ret));
    }

    status.mdns_ok = status.netif_ok && status.event_loop_ok && init_mdns();

    initialized = true;

    ESP_LOGW(TAG, "init: nvs=%s netif=%s events=%s mdns=%s",
             ok_str(status.nvs_ok), ok_str(status.netif_ok),
             ok_str(status.event_loop_ok), ok_str(status.mdns_ok));
    if (status.nvs_erased) {
        ESP_LOGW(TAG, "nvs was erased this boot, provisioning state is gone");
    }
    if (status.coredump_present) {
        ESP_LOGE(TAG, "a core dump is waiting to be reported and erased");
    }
    if (status.safe_mode) {
        ESP_LOGE(TAG, "SAFE MODE: protocols and peripherals stay down, only "
                      "the ota path comes up");
    }
    if (system_is_degraded()) {
        ESP_LOGE(TAG, "node is running DEGRADED");
    }

    arm_ota_validation();

    return status;
}

const system_status_t *system_get_status(void) { return &status; }

bool system_is_degraded(void) {
    return !(status.nvs_ok && status.netif_ok && status.event_loop_ok &&
             status.mdns_ok);
}

void system_ota_start(void) { bridge_ota_init(); }

void system_enter_safe_mode(void) {
    ESP_LOGE(TAG, "safe mode: boot attempt %" PRIu32 ", bringing up one link",
             system_boot_attempts());

    bridge_ota_init();

    while (true) {
        vTaskDelay(pdMS_TO_TICKS(30000));
        ESP_LOGW(TAG, "safe mode: %s, waiting for an ota update",
                 bridge_ota_state_str(bridge_ota_get_state()));
    }
}
