# whisper-rs-sys exposes WHISPER_* and CMAKE_* variables, but not GGML_*
# options. Setting this in the toolchain file applies before ggml declares its
# native-CPU default, avoiding binaries tied to the CI runner's CPU features.
set(GGML_NATIVE OFF CACHE BOOL "Build a portable ggml CPU backend" FORCE)

# whisper-rs-sys' Windows ARM build uses the Ninja generator and its CMake
# wrapper does not expose generator toolsets. Keep the compiler selection
# explicit in the toolchain while still letting the CMake crate receive flags
# from cc-rs.
if (DEFINED ENV{CC_aarch64_pc_windows_msvc})
    set(CMAKE_C_COMPILER "$ENV{CC_aarch64_pc_windows_msvc}" CACHE FILEPATH "" FORCE)
endif()
if (DEFINED ENV{CXX_aarch64_pc_windows_msvc})
    set(CMAKE_CXX_COMPILER "$ENV{CXX_aarch64_pc_windows_msvc}" CACHE FILEPATH "" FORCE)
endif()
