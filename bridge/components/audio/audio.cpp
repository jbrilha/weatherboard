#include "audio.hpp"
#include "driver/i2c_types.h"
#include "driver/i2s_std.h"
#include "driver/i2s_types.h"
#include "es8311_codec.h"
#include "esp_check.h"
#include "esp_codec_dev.h"
#include "esp_codec_dev_defaults.h"
#include "esp_err.h"
#include "freertos/idf_additions.h"
#include "freertos/projdefs.h"
#include "hal/i2c_types.h"
#include "hal/i2s_types.h"
#include "soc/clk_tree_defs.h"

static constexpr char TAG[] = "Audio";

extern const uint8_t music_pcm_start[] asm("_binary_canon_pcm_start");
extern const uint8_t music_pcm_end[] asm("_binary_canon_pcm_end");

static constexpr gpio_num_t BSP_I2S_SCLK = GPIO_NUM_12;
static constexpr gpio_num_t BSP_I2S_MCLK = GPIO_NUM_13;
static constexpr gpio_num_t BSP_I2S_LCLK = GPIO_NUM_10;
static constexpr gpio_num_t BSP_I2S_DOUT = GPIO_NUM_9;
static constexpr gpio_num_t BSP_I2S_DSIN = GPIO_NUM_11;
static constexpr gpio_num_t PA_PIN = GPIO_NUM_53;

static constexpr float INPUT_GAIN = 24.0;
static constexpr uint8_t VOLUME = 100;
static constexpr uint32_t SAMPLE_RATE = 16000;

Audio::Audio(I2C &i2c)
    : i2c(i2c), speaker(nullptr), speaker_in_use(false), microphone(nullptr),
      microphone_in_use(false), i2s_data_if(nullptr), i2s_tx_chan(nullptr),
      i2s_rx_chan(nullptr), volume(VOLUME), sample_rate(SAMPLE_RATE),
      music_task(nullptr), echo_task(nullptr) {}

Audio::~Audio() {
    esp_codec_dev_close(speaker);
    esp_codec_dev_close(microphone);
}

esp_err_t Audio::init(void) {
    ESP_ERROR_CHECK(i2s_init());
    assert(i2s_data_if);

    speaker = speaker_codec_init();
    ESP_RETURN_ON_FALSE(speaker, ESP_FAIL, TAG, "init speaker codec failed");
    microphone = microphone_codec_init();
    ESP_RETURN_ON_FALSE(microphone, ESP_FAIL, TAG,
                        "init microphone codec failed");
    ESP_RETURN_ON_ERROR(open_with_format(sample_rate, 16, I2S_SLOT_MODE_STEREO),
                        TAG, "open codecs with format failed");
    ESP_RETURN_ON_ERROR(set_volume(volume), TAG, "set codec volume failed");
    return ESP_OK;
}

esp_codec_dev_handle_t Audio::speaker_codec_init(void) {
    es8311_codec_cfg_t es8311_cfg = get_es8311_cfg(ESP_CODEC_DEV_WORK_MODE_DAC);

    const audio_codec_if_t *es8311_dev = es8311_codec_new(&es8311_cfg);
    assert(es8311_dev);

    esp_codec_dev_cfg_t codec_dev_cfg = {
        .dev_type = ESP_CODEC_DEV_TYPE_IN_OUT,
        .codec_if = es8311_dev,
        .data_if = i2s_data_if,
    };

    return esp_codec_dev_new(&codec_dev_cfg);
}

esp_codec_dev_handle_t Audio::microphone_codec_init(void) {
    es8311_codec_cfg_t es8311_cfg =
        get_es8311_cfg(ESP_CODEC_DEV_WORK_MODE_BOTH);

    const audio_codec_if_t *es8311_dev = es8311_codec_new(&es8311_cfg);
    assert(es8311_dev);

    esp_codec_dev_cfg_t codec_es8311_dev_cfg = {
        .dev_type = ESP_CODEC_DEV_TYPE_IN,
        .codec_if = es8311_dev,
        .data_if = i2s_data_if,
    };

    return esp_codec_dev_new(&codec_es8311_dev_cfg);
}

es8311_codec_cfg_t Audio::get_es8311_cfg(esp_codec_dec_work_mode_t codec_mode) {
    const audio_codec_gpio_if_t *gpio_if = audio_codec_new_gpio();

    audio_codec_i2c_cfg_t i2c_cfg = {
        .port = I2C_NUM_1,
        .addr = ES8311_CODEC_DEFAULT_ADDR,
        .bus_handle = i2c.get_bus(),
        .clock_speed_hz = 0,
    };
    const audio_codec_ctrl_if_t *i2c_ctrl_if =
        audio_codec_new_i2c_ctrl(&i2c_cfg);
    assert(i2c_ctrl_if);

    esp_codec_dev_hw_gain_t gain = {
        .pa_voltage = 5.0,
        .codec_dac_voltage = 3.3,
        .pa_gain = 0.0,
    };

    es8311_codec_cfg_t es8311_cfg = {
        .ctrl_if = i2c_ctrl_if,
        .gpio_if = gpio_if,
        .codec_mode = codec_mode,
        .pa_pin = PA_PIN,
        .pa_reverted = false,
        .master_mode = false,
        .use_mclk = true,
        .digital_mic = false,
        .invert_mclk = false,
        .invert_sclk = false,
        .hw_gain = gain,
        .no_dac_ref = false,
        .mclk_div = 256,
    };

    return es8311_cfg;
}

esp_err_t Audio::i2s_init(void) {
    if ((i2s_tx_chan != nullptr) && (i2s_rx_chan != nullptr)) {
        return ESP_OK;
    }

    i2s_chan_config_t chan_cfg =
        I2S_CHANNEL_DEFAULT_CONFIG(I2S_NUM_1, I2S_ROLE_MASTER);
    chan_cfg.auto_clear = true;
    ESP_ERROR_CHECK(i2s_new_channel(&chan_cfg, &i2s_tx_chan, &i2s_rx_chan));

    const i2s_std_config_t i2s_std_cfg = {
        .clk_cfg = I2S_STD_CLK_DEFAULT_CONFIG(sample_rate),
        .slot_cfg = I2S_STD_PHILIP_SLOT_DEFAULT_CONFIG(I2S_DATA_BIT_WIDTH_16BIT,
                                                       I2S_SLOT_MODE_STEREO),
        .gpio_cfg =
            {
                .mclk = BSP_I2S_MCLK,
                .bclk = BSP_I2S_SCLK,
                .ws = BSP_I2S_LCLK,
                .dout = BSP_I2S_DOUT,
                .din = BSP_I2S_DSIN,
                .invert_flags =
                    {
                        .mclk_inv = false,
                        .bclk_inv = false,
                        .ws_inv = false,
                    },
            },
    };
    if (i2s_tx_chan != nullptr) {
        ESP_ERROR_CHECK(i2s_channel_init_std_mode(i2s_tx_chan, &i2s_std_cfg));
        ESP_ERROR_CHECK(i2s_channel_enable(i2s_tx_chan));
    }

    if (i2s_rx_chan != nullptr) {
        ESP_ERROR_CHECK(i2s_channel_init_std_mode(i2s_rx_chan, &i2s_std_cfg));
        ESP_ERROR_CHECK(i2s_channel_enable(i2s_rx_chan));
    }

    audio_codec_i2s_cfg_t i2s_cfg = {
        .port = I2S_NUM_1,
        .rx_handle = i2s_rx_chan,
        .tx_handle = i2s_tx_chan,
        .clk_src = I2S_CLK_SRC_DEFAULT,
    };

    i2s_data_if = audio_codec_new_i2s_data(&i2s_cfg);

    return ESP_OK;
}

esp_err_t Audio::open_with_format(uint32_t rate, uint8_t bits_cfg,
                                  i2s_slot_mode_t ch) {
    ESP_RETURN_ON_FALSE(speaker || microphone, ESP_ERR_INVALID_STATE, TAG,
                        "codec is not initialized");

    this->sample_rate = rate;

    esp_err_t ret = ESP_OK;
    esp_codec_dev_sample_info_t fs = {};
    fs.bits_per_sample = bits_cfg;
    fs.channel = static_cast<uint8_t>(ch);
    fs.sample_rate = rate;

    if (speaker_in_use) {
        ret |= esp_codec_dev_close(speaker);
        speaker_in_use = false;
    }
    if (microphone_in_use) {
        ret |= esp_codec_dev_close(microphone);
        microphone_in_use = false;
    }
    if (speaker != nullptr) {
        ret |= esp_codec_dev_open(speaker, &fs);
        if (ret == ESP_OK) {
            speaker_in_use = true;
        }
    }
    if (microphone != nullptr) {
        ret |= esp_codec_dev_set_in_gain(microphone, input_gain);
        ret |= esp_codec_dev_open(microphone, &fs);
        if (ret == ESP_OK) {
            microphone_in_use = true;
        }
    }
    return ret;
}
esp_err_t Audio::set_volume(uint8_t vol) {
    ESP_RETURN_ON_FALSE(speaker, ESP_ERR_INVALID_STATE, TAG,
                        "speaker codec is not initialized");
    this->volume = vol;
    return esp_codec_dev_set_out_vol(speaker, volume);
}

static const char err_reason[][30] = {
    "input param is invalid",
    "operation timeout",
};

void Audio::i2s_music(void *args) {
    Audio *audio = static_cast<Audio *>(args);

    esp_err_t ret = ESP_OK;
    size_t bytes_write = 0;
    uint8_t *data_ptr = (uint8_t *)music_pcm_start;
    const size_t music_size = music_pcm_end - music_pcm_start;

    while (true) {
        data_ptr = (uint8_t *)music_pcm_start;
        ret = esp_codec_dev_write(audio->speaker, data_ptr, music_size);
        bytes_write = (ret == ESP_OK) ? music_size : 0;
        if (ret != ESP_OK) {
            ESP_LOGE(TAG, "[music] i2s write failed, %s",
                     err_reason[ret == ESP_ERR_TIMEOUT]);
            abort();
        }
        if (bytes_write > 0) {
            ESP_LOGI(TAG, "[music] i2s music played, %u bytes are written.",
                     (unsigned int)bytes_write);
        } else {
            ESP_LOGE(TAG, "[music] i2s music play failed.");
            abort();
        }
        data_ptr = (uint8_t *)music_pcm_start;
        vTaskDelay(1000 / portTICK_PERIOD_MS);
    }
}

void Audio::i2s_echo(void *args) {
    Audio *audio = static_cast<Audio *>(args);

    int rcv_buf_size = 2400;
    int *mic_data = (int *)malloc(rcv_buf_size);
    if (mic_data == nullptr) {
        ESP_LOGE(TAG, "[echo] No memory for read data buffer");
        abort();
    }

    esp_err_t ret = ESP_OK;
    size_t bytes_read = 0;
    size_t bytes_write = 0;
    ESP_LOGI(TAG, "[echo] Echo start");

    while (true) {
        memset(mic_data, 0, rcv_buf_size);
        ret = esp_codec_dev_read(audio->microphone, mic_data, rcv_buf_size);
        bytes_read = (ret == ESP_OK) ? rcv_buf_size : 0;
        if (ret != ESP_OK) {
            ESP_LOGE(TAG, "[echo] i2s read failed, %s",
                     err_reason[ret == ESP_ERR_TIMEOUT]);
            abort();
        }
        ret = esp_codec_dev_write(audio->speaker, mic_data, rcv_buf_size);
        bytes_write = (ret == ESP_OK) ? rcv_buf_size : 0;
        if (ret != ESP_OK) {
            ESP_LOGE(TAG, "[echo] i2s write failed, %s",
                     err_reason[ret == ESP_ERR_TIMEOUT]);
            abort();
        }
        if (bytes_read != bytes_write) {
            ESP_LOGW(TAG, "[echo] %u bytes read but only %u bytes are written",
                     (unsigned int)bytes_read, (unsigned int)bytes_write);
        }
    }
}

esp_err_t Audio::play_music(void) {
    if (xTaskCreate(i2s_music, "i2s_music", 4096, this, 5, &music_task) !=
        pdTRUE) {
        return ESP_ERR_INVALID_STATE;
    }

    return ESP_OK;
}

void Audio::stop_music(void) { vTaskDelete(music_task); }

esp_err_t Audio::play_echo(void) {
    if (xTaskCreate(i2s_echo, "i2s_echo", 8192, this, 5, &echo_task) !=
        pdTRUE) {
        return ESP_ERR_INVALID_STATE;
    }

    return ESP_OK;
}

void Audio::stop_echo(void) { vTaskDelete(echo_task); }
