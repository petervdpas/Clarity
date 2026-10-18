// Runs every test; the exit status is the number of failures.

#include <exception>

#include "check.hpp"

int main() {
    for (const auto& test : check::tests()) {
        int before = check::failures();
        try {
            test.run();
        } catch (const check::Stop&) {
        } catch (const std::exception& e) {
            check::fail(test.name, 0, std::string("threw: ") + e.what());
        }
        std::cout << (check::failures() == before ? "ok   " : "FAIL ") << test.name << "\n";
    }
    std::cout << check::tests().size() << " tests, " << check::failures() << " failures\n";
    return check::failures() == 0 ? 0 : 1;
}
