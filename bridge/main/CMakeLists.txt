idf_component_register(SRCS "bridge.cpp"
                    INCLUDE_DIRS ".")
target_compile_options(${COMPONENT_LIB} PRIVATE -Wno-missing-field-initializers)
