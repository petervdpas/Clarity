// A test harness of a few lines, so the runtime's tests need nothing but
// the compiler: TEST names a test, CHECK and CHECK_EQ report a failure and
// go on, REQUIRE stops the test.

#pragma once

#include <functional>
#include <iostream>
#include <sstream>
#include <string>
#include <vector>

namespace check {

struct Test {
    const char* name;
    std::function<void()> run;
};

inline std::vector<Test>& tests() {
    static std::vector<Test> all;
    return all;
}

inline int& failures() {
    static int count = 0;
    return count;
}

struct Stop {};

struct Registered {
    Registered(const char* name, std::function<void()> run) { tests().push_back({name, std::move(run)}); }
};

template <typename T>
std::string shown(const T& value) {
    std::ostringstream out;
    if constexpr (requires { out << value; }) {
        out << value;
    } else {
        out << "(a value)";
    }
    return out.str();
}

inline void fail(const char* file, int line, const std::string& what) {
    ++failures();
    std::cerr << file << ":" << line << ": " << what << "\n";
}

}  // namespace check

#define CHECK_CAT2(a, b) a##b
#define CHECK_CAT(a, b) CHECK_CAT2(a, b)
#define TEST(name)                                                                        \
    static void name();                                                                   \
    static check::Registered CHECK_CAT(registered_, name)(#name, name);                   \
    static void name()
#define CHECK(cond) \
    do { if (!(cond)) check::fail(__FILE__, __LINE__, "failed: " #cond); } while (0)
#define CHECK_EQ(a, b)                                                                              \
    do {                                                                                            \
        /* Copies: a reference into a temporary would not outlive this line. */                     \
        const auto check_a = (a);                                                                   \
        const auto check_b = (b);                                                                   \
        if (!(check_a == check_b))                                                                  \
            check::fail(__FILE__, __LINE__, #a " == " #b ": " + check::shown(check_a) + " != " + check::shown(check_b)); \
    } while (0)
#define REQUIRE(cond) \
    do { if (!(cond)) { check::fail(__FILE__, __LINE__, "required: " #cond); throw check::Stop{}; } } while (0)
#define CHECK_THROWS(type, expr)                                                    \
    do {                                                                            \
        bool check_thrown = false;                                                  \
        try { (void)(expr); } catch (const type&) { check_thrown = true; }          \
        if (!check_thrown) check::fail(__FILE__, __LINE__, "did not throw " #type ": " #expr); \
    } while (0)
