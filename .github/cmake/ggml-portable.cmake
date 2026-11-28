# whisper-rs-sys exposes WHISPER_* and CMAKE_* variables, but not GGML_*
# options. Setting this in the toolchain file applies before ggml declares its
# native-CPU default, avoiding binaries tied to the CI runner's CPU features.
set(GGML_NATIVE OFF CACHE BOOL "Build a portable ggml CPU backend" FORCE)

# ggml 0.13's ARM NEON half-load intrinsics use __fp16 pointers, while its
# internal Windows ARM half representation is uint16_t. clang-cl therefore
# rejects the base NEON path even when FP16 vector arithmetic is disabled.
# Build the Windows ARM backend without SIMD until upstream fixes the pointer
# mismatch. This keeps inference portable at a performance cost on this target.
if (DEFINED ENV{CC_aarch64_pc_windows_msvc})
    set(CMAKE_SYSTEM_NAME Windows CACHE STRING "Windows ARM64 Whisper target" FORCE)
    set(CMAKE_SYSTEM_PROCESSOR ARM64 CACHE STRING "Windows ARM64 Whisper processor" FORCE)
    set(GGML_CPU_ARM_ARCH "armv8-a+nosimd" CACHE STRING "Use portable scalar ARMv8-A on Windows ARM" FORCE)
endif()

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
