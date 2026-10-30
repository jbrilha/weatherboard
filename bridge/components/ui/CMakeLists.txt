idf_component_register(SRCS "ui.cpp" "lvgl_demo_ui.cpp"
                    INCLUDE_DIRS "."
                    REQUIRES lvgl display
                    PRIV_REQUIRES esp_timer
)
