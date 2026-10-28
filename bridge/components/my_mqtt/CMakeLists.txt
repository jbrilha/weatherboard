idf_component_register(SRCS "mqtt.cpp"
                    INCLUDE_DIRS "."
                    PRIV_REQUIRES mqtt
)
target_compile_options(${COMPONENT_LIB} PRIVATE -Wno-missing-field-initializers)
