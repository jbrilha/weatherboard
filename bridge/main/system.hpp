#pragma once

#include "esp_err.h"
#include "esp_system.h"
#include <stdbool.h>
#include <stdint.h>

typedef struct {
    bool nvs_ok;
    bool nvs_erased;
    bool netif_ok;
    bool event_loop_ok;
    bool mdns_ok;
    bool coredump_present;
    bool safe_mode;
    uint32_t boot_attempts;
    esp_reset_reason_t reset_reason;
} system_status_t;

typedef struct {
    bool present;
    char task[16];
    uint32_t pc;
} system_coredump_t;

typedef bool (*system_health_fn)(void);

system_status_t system_init(void);
const system_status_t *system_get_status(void);
bool system_is_degraded(void);
const char *system_reset_reason_str(esp_reset_reason_t reason);

bool system_ota_is_pending_verify(void);
void system_ota_set_health_check(system_health_fn fn);
esp_err_t system_ota_mark_valid(void);

bool system_coredump_get(system_coredump_t *out);
esp_err_t system_coredump_clear(void);

bool system_in_safe_mode(void);
void system_enter_safe_mode(void);

void system_ota_start(void);

uint32_t system_boot_attempts(void);
void system_mark_clean_run(void);
