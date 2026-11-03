// Custom part OrbField: a field of orbs drawn on the canvas Clarity.UI
// gives. An orb grows, then shrinks away; a click on it catches it. A round
// lasts thirty seconds, then tells how many were caught.

#include <algorithm>
#include <chrono>
#include <cmath>
#include <cstdint>
#include <random>
#include <vector>

#include "clarity/OrbField.h"

namespace {

constexpr double ROUND_SECONDS = 30.0;
constexpr double ORB_SECONDS = 2.5;
/// How often the field moves on: about 60 times a second.
constexpr double FRAME_MS = 16.0;

struct Orb {
    double x, y;
    double dx, dy;
    double age = 0.0;
    uint32_t color;
};

constexpr uint32_t COLORS[] = {0xFFF4725E, 0xFFFACC15, 0xFF4ADE80, 0xFF60A5FA, 0xFFC084FC};

class OrbField final : public OrbFieldPart {
public:
    void start(clarity::ui::Cx& cx) override {
        restart();
        cx.wake_in(FRAME_MS);
    }

    void stop() override { stopped = true; }

    void show_enabled(bool on, clarity::ui::Cx& cx) override {
        enabled = on;
        cx.invalidate();
    }

    void show_speed(int64_t value, clarity::ui::Cx&) override { speed = std::clamp<int64_t>(value, 1, 10); }

    void on_restart(clarity::ui::Cx& cx) override {
        restart();
        cx.invalidate();
    }

    void wake(clarity::ui::Cx& cx) override {
        if (stopped) return;
        auto now = std::chrono::steady_clock::now();
        double dt = std::min(std::chrono::duration<double>(now - last).count(), 0.1);
        last = now;
        auto place = cx.bounds();
        step(dt, place.width, place.height, cx);
        cx.invalidate();
        cx.wake_in(FRAME_MS);
    }

    void pointer_pressed(const clarity::ui::Pointer& at, clarity::ui::Cx& cx) override {
        if (!enabled || over) return;
        auto place = cx.bounds();
        double x = at.x - place.x, y = at.y - place.y;
        for (auto it = orbs.begin(); it != orbs.end(); ++it) {
            double r = radius(*it);
            if ((x - it->x) * (x - it->x) + (y - it->y) * (y - it->y) <= r * r) {
                orbs.erase(it);
                caught(cx, ++points);
                cx.invalidate();
                return;
            }
        }
    }

    void draw(clarity::ui::Canvas& canvas, double x, double y, double width, double height) override {
        canvas.fill(x, y, width, height, 0xFF111827, 6);
        canvas.clip(x, y, width, height);
        // How much of the round is left, as a bar along the top.
        double left = over ? 0.0 : 1.0 - played / ROUND_SECONDS;
        canvas.fill(x, y, left * width, 6, 0xFF60A5FA);
        for (const auto& orb : orbs) {
            double r = radius(orb);
            canvas.oval(x + orb.x - r, y + orb.y - r, 2 * r, 2 * r, enabled ? orb.color : 0xFF787878);
        }
        canvas.restore();
    }

private:
    bool enabled = true;
    bool stopped = false;
    int64_t speed = 3;
    int64_t points = 0;
    double played = 0.0;
    bool over = false;
    double until_next = 0.0;
    std::vector<Orb> orbs;
    std::mt19937 random{std::random_device{}()};
    std::chrono::steady_clock::time_point last = std::chrono::steady_clock::now();

    double between(double low, double high) { return std::uniform_real_distribution<double>(low, high)(random); }

    /// How big an orb is at its age: it grows for a fifth of its life, then shrinks.
    static double radius(const Orb& orb) {
        double life = orb.age / ORB_SECONDS;
        double size = life < 0.2 ? life / 0.2 : 1.0 - (life - 0.2) / 0.8;
        return 34.0 * std::max(0.0, size);
    }

    void restart() {
        points = 0;
        played = 0.0;
        over = false;
        orbs.clear();
        until_next = 0.0;
        last = std::chrono::steady_clock::now();
    }

    void step(double dt, double width, double height, clarity::ui::Cx& cx) {
        if (over) return;
        played += dt;
        if (played >= ROUND_SECONDS) {
            over = true;
            orbs.clear();
            finished(cx, points);
            return;
        }
        until_next -= dt;
        if (until_next <= 0.0 && width > 80 && height > 80) {
            double pace = 20.0 + 12.0 * static_cast<double>(speed);
            orbs.push_back({between(40, width - 40), between(40, height - 40), between(-pace, pace), between(-pace, pace), 0.0, COLORS[random() % 5]});
            until_next = 1.1 - 0.08 * static_cast<double>(speed);
        }
        for (auto& orb : orbs) {
            orb.age += dt;
            orb.x += orb.dx * dt;
            orb.y += orb.dy * dt;
            if (orb.x < 0 || orb.x > width) orb.dx = -orb.dx;
            if (orb.y < 0 || orb.y > height) orb.dy = -orb.dy;
        }
        orbs.erase(std::remove_if(orbs.begin(), orbs.end(), [](const Orb& o) { return o.age >= ORB_SECONDS; }), orbs.end());
    }
};

}  // namespace

std::unique_ptr<OrbFieldPart> make_OrbField() {
    return std::make_unique<OrbField>();
}
