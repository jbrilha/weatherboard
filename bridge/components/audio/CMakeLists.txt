idf_component_register(SRCS "audio.cpp"
                    INCLUDE_DIRS "."
                    REQUIRES esp_driver_i2s esp_driver_gpio esp_driver_i2c esp_codec_dev i2c
                    EMBED_FILES "canon.pcm"
)
