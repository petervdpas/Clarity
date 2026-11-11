// Runs every test, or those whose name holds the word given
// (`clarity_tests signin`); the exit status is the number of failures.

#include <exception>
#include <string>

#include "check.hpp"

int main(int argc, char** argv) {
    std::string only = argc > 1 ? argv[1] : "";
    size_t ran = 0;
    for (const auto& test : check::tests()) {
        if (!only.empty() && std::string(test.name).find(only) == std::string::npos) continue;
        ++ran;
        int before = check::failures();
        try {
            test.run();
        } catch (const check::Stop&) {
        } catch (const std::exception& e) {
            check::fail(test.name, 0, std::string("threw: ") + e.what());
        }
        std::cout << (check::failures() == before ? "ok   " : "FAIL ") << test.name << "\n";
    }
    std::cout << ran << " tests, " << check::failures() << " failures\n";
    return check::failures() == 0 ? 0 : 1;
}
