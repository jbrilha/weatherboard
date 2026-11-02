idf_component_register(SRCS "weather_mqtt.cpp"
                    INCLUDE_DIRS "."
                    REQUIRES mqtt weatherboard_ffi
)
target_compile_options(${COMPONENT_LIB} PRIVATE -Wno-missing-field-initializers)
