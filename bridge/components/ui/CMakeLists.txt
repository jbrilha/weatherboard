set(UI_SRCS "ui.cpp" "pages/demo.cpp")

if(ESP_PLATFORM)
    idf_component_register(SRCS ${UI_SRCS}
                        INCLUDE_DIRS "."
                        REQUIRES lvgl
    )
else()
    add_library(ui ${UI_SRCS})
    target_include_directories(ui PUBLIC .)
    target_link_libraries(ui PUBLIC lvgl)
endif()
