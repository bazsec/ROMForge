# Match Rust's static CRT on Windows, including SDL's generated VS project.
# CMAKE_C_FLAGS alone is insufficient: CMake's RuntimeLibrary property wins.
if(CMAKE_HOST_WIN32)
  set(CMAKE_MSVC_RUNTIME_LIBRARY "MultiThreaded" CACHE STRING "MSVC runtime")
endif()
