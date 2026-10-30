cmake_minimum_required(VERSION 3.22)
project(bridge_sim C CXX)

set(CMAKE_C_STANDARD 17)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

set(LVGL_DIR ${CMAKE_CURRENT_SOURCE_DIR}/../managed_components/lvgl__lvgl)
if(NOT EXISTS ${LVGL_DIR})
    message(FATAL_ERROR "LVGL not found at ${LVGL_DIR}. Run an IDF build first so the component manager fetches it.")
endif()

set(CONFIG_LV_BUILD_DEMOS OFF CACHE BOOL "" FORCE)
set(CONFIG_LV_BUILD_EXAMPLES OFF CACHE BOOL "" FORCE)
set(CONFIG_LV_USE_THORVG OFF CACHE BOOL "" FORCE)
set(LV_FETCH_DEPENDENCIES OFF CACHE BOOL "" FORCE)
set(LV_BUILD_INSTALL OFF CACHE BOOL "" FORCE)

find_package(SDL3 REQUIRED CONFIG COMPONENTS SDL3)

add_subdirectory(${LVGL_DIR} lvgl)
add_subdirectory(../components/ui ui)

add_executable(sim main.cpp)
target_link_libraries(sim PRIVATE ui lvgl SDL3::SDL3)
