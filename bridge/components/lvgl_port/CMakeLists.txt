idf_component_register(SRCS "lvgl_port.cpp"
                    INCLUDE_DIRS "."
                    REQUIRES lvgl display
                    PRIV_REQUIRES esp_timer ui
)
