# whisper-rs-sys exposes WHISPER_* and CMAKE_* variables, but not GGML_*
# options. Setting this in the toolchain file applies before ggml declares its
# native-CPU default, avoiding binaries tied to the CI runner's CPU features.
set(GGML_NATIVE OFF CACHE BOOL "Build a portable ggml CPU backend" FORCE)

# The LLVM ARM64 Windows clang-cl toolchain advertises NEON half-float vector
# support, but ggml's current intrinsic code is incompatible with its __fp16
# pointer types. Disable this optional vector path for the Windows ARM build;
# scalar kernels remain available and portable.
if (DEFINED ENV{CC_aarch64_pc_windows_msvc})
    set(CMAKE_SYSTEM_NAME Windows CACHE STRING "Windows ARM64 Whisper target" FORCE)
    set(CMAKE_SYSTEM_PROCESSOR ARM64 CACHE STRING "Windows ARM64 Whisper processor" FORCE)
    set(GGML_CPU_ARM_ARCH "armv8-a" CACHE STRING "Use baseline ARMv8-A on Windows ARM" FORCE)
    # clang-cl reports support for -mfp16-format=ieee, but ggml's FP16
    # vector macros use incompatible __fp16 pointer types with its internal
    # uint16_t representation. Keep the safe scalar half conversion path.
    set(GGML_COMPILER_SUPPORTS_FP16_FORMAT_I3E "" CACHE INTERNAL "Disable incompatible clang-cl FP16 vectors" FORCE)
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
