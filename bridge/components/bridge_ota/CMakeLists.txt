idf_component_register(SRCS "bridge_ota.cpp"
                      INCLUDE_DIRS "."
                      PRIV_REQUIRES esp_https_ota esp_http_client app_update esp_app_format
)
