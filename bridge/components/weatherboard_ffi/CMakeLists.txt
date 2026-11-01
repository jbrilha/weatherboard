idf_component_register(INCLUDE_DIRS ".")

set(RUST_TARGET riscv32imafc-unknown-none-elf)
set(FFI_DIR ${CMAKE_CURRENT_LIST_DIR}/../../../ffi)
set(FFI_LIB ${FFI_DIR}/target/${RUST_TARGET}/release/libweatherboard_ffi.a)

add_custom_target(weatherboard_ffi_build
    COMMAND cargo build --release --target ${RUST_TARGET}
    WORKING_DIRECTORY ${FFI_DIR}
    BYPRODUCTS ${FFI_LIB}
    VERBATIM)

add_library(weatherboard_ffi_lib STATIC IMPORTED GLOBAL)
set_target_properties(weatherboard_ffi_lib PROPERTIES IMPORTED_LOCATION ${FFI_LIB})
add_dependencies(weatherboard_ffi_lib weatherboard_ffi_build)

target_link_libraries(${COMPONENT_LIB} INTERFACE weatherboard_ffi_lib)
