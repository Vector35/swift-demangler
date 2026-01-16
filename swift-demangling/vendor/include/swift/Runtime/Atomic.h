#ifndef SWIFT_RUNTIME_ATOMIC_H
#define SWIFT_RUNTIME_ATOMIC_H
#include <atomic>
namespace swift {
template<typename T> using Atomic = std::atomic<T>;
}
#endif
