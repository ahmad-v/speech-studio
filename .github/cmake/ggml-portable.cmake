# whisper-rs-sys exposes WHISPER_* and CMAKE_* variables, but not GGML_*
# options. Setting this in the toolchain file applies before ggml declares its
# native-CPU default, avoiding binaries tied to the CI runner's CPU features.
set(GGML_NATIVE OFF CACHE BOOL "Build a portable ggml CPU backend" FORCE)
