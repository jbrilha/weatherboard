if(ESP_PLATFORM)
    idf_component_register(SRCS "lvgl_demo_ui.cpp"
                        INCLUDE_DIRS "."
                        REQUIRES lvgl
    )
else()
    add_library(ui lvgl_demo_ui.cpp)
    target_include_directories(ui PUBLIC .)
    target_link_libraries(ui PUBLIC lvgl)
endif()
