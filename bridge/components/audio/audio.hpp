#pragma once

#include "audio_codec_data_if.h"
#include "driver/i2s_std.h"
#include "driver/i2s_types.h"
#include "es8311_codec.h"
#include "esp_codec_dev.h"
#include "esp_err.h"
#include "freertos/idf_additions.h"
#include "hal/i2s_types.h"
#include "i2c.hpp"

class Audio {
  public:
    Audio(I2C &i2c);
    ~Audio();

    esp_err_t init(void);

    esp_err_t set_volume(uint8_t vol);
    esp_err_t open_with_format(uint32_t rate, uint8_t bits_cfg,
                               i2s_slot_mode_t ch);

    esp_err_t play_music(void);
    void stop_music(void);

    esp_err_t play_echo(void);
    void stop_echo(void);

  private:
    I2C &i2c;
    bool initialized;

    esp_codec_dev_handle_t speaker;
    bool speaker_in_use;

    esp_codec_dev_handle_t microphone;
    bool microphone_in_use;
    float input_gain;

    const audio_codec_data_if_t *i2s_data_if;
    i2s_chan_handle_t i2s_tx_chan;
    i2s_chan_handle_t i2s_rx_chan;

    esp_err_t i2s_init(void);
    esp_codec_dev_handle_t speaker_codec_init(void);
    esp_codec_dev_handle_t microphone_codec_init(void);

    uint8_t volume;
    uint32_t sample_rate;

    TaskHandle_t music_task;
    static void i2s_music(void *args);

    TaskHandle_t echo_task;
    static void i2s_echo(void *args);

    es8311_codec_cfg_t get_es8311_cfg(esp_codec_dec_work_mode_t codec_mode);
};
