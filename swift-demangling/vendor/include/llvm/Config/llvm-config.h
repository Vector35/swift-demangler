#ifndef LLVM_CONFIG_H
#define LLVM_CONFIG_H
// Stub for generated LLVM config - only defines what Swift demangling needs
#if defined(__APPLE__)
#define LLVM_ON_UNIX 1
#elif defined(__linux__)
#define LLVM_ON_UNIX 1
#elif defined(_WIN32)
#define LLVM_ON_WIN32 1
#else
#define LLVM_ON_UNIX 1
#endif
#define LLVM_VERSION_MAJOR 18
#define LLVM_VERSION_MINOR 0
#define LLVM_VERSION_PATCH 0
#define LLVM_VERSION_STRING "18.0.0"
#define LLVM_ENABLE_ABI_BREAKING_CHECKS 0
#endif
