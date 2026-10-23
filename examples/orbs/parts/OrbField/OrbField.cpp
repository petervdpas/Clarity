// Custom part OrbField: a field of orbs drawn with SFML in the native
// window Clarity gives it. An orb grows, then shrinks away; a click on it
// catches it. A round lasts thirty seconds, then tells how many were caught.

#include <SFML/Graphics.hpp>

#include <algorithm>
#include <cmath>
#include <cstdint>
#include <random>
#include <vector>

#include "clarity/OrbField.h"

namespace {

constexpr float ROUND_SECONDS = 30.f;
constexpr float ORB_SECONDS = 2.5f;

struct Orb {
    sf::Vector2f at;
    sf::Vector2f moving;
    float age = 0.f;
    sf::Color color;
};

const sf::Color COLORS[] = {sf::Color(244, 114, 94), sf::Color(250, 204, 21), sf::Color(74, 222, 128), sf::Color(96, 165, 250), sf::Color(192, 132, 252)};

}  // namespace

struct OrbFieldPart {
    OrbFieldHost* host;
    sf::RenderWindow window;
    bool enabled = true;
    int64_t speed = 3;
    int64_t points = 0;
    float played = 0.f;
    bool over = false;
    float until_next = 0.f;
    std::vector<Orb> orbs;
    std::mt19937 random{std::random_device{}()};
    sf::Clock clock;

    float between(float low, float high) { return std::uniform_real_distribution<float>(low, high)(random); }

    /// How big an orb is at its age: it grows for a fifth of its life, then shrinks.
    static float radius(const Orb& orb) {
        float life = orb.age / ORB_SECONDS;
        float size = life < 0.2f ? life / 0.2f : 1.f - (life - 0.2f) / 0.8f;
        return 34.f * std::max(0.f, size);
    }

    void restart() {
        points = 0;
        played = 0.f;
        over = false;
        orbs.clear();
        until_next = 0.f;
        clock.restart();
    }

    void click(sf::Vector2f where) {
        if (!enabled || over) return;
        for (auto it = orbs.begin(); it != orbs.end(); ++it) {
            sf::Vector2f d = where - it->at;
            if (d.x * d.x + d.y * d.y <= radius(*it) * radius(*it)) {
                orbs.erase(it);
                OrbField_caught(host, ++points);
                return;
            }
        }
    }

    void step(float dt) {
        if (over) return;
        played += dt;
        if (played >= ROUND_SECONDS) {
            over = true;
            orbs.clear();
            OrbField_finished(host, points);
            return;
        }
        sf::Vector2u size = window.getSize();
        until_next -= dt;
        if (until_next <= 0.f && size.x > 80 && size.y > 80) {
            float pace = 20.f + 12.f * static_cast<float>(speed);
            orbs.push_back({{between(40.f, size.x - 40.f), between(40.f, size.y - 40.f)}, {between(-pace, pace), between(-pace, pace)}, 0.f, COLORS[random() % 5]});
            until_next = 1.1f - 0.08f * static_cast<float>(speed);
        }
        for (auto& orb : orbs) {
            orb.age += dt;
            orb.at += orb.moving * dt;
            if (orb.at.x < 0.f || orb.at.x > size.x) orb.moving.x = -orb.moving.x;
            if (orb.at.y < 0.f || orb.at.y > size.y) orb.moving.y = -orb.moving.y;
        }
        orbs.erase(std::remove_if(orbs.begin(), orbs.end(), [](const Orb& o) { return o.age >= ORB_SECONDS; }), orbs.end());
    }

    void draw() {
        window.clear(sf::Color(17, 24, 39));
        sf::Vector2u size = window.getSize();
        // How much of the round is left, as a bar along the top.
        float left = over ? 0.f : 1.f - played / ROUND_SECONDS;
        sf::RectangleShape bar({left * static_cast<float>(size.x), 6.f});
        bar.setFillColor(sf::Color(96, 165, 250));
        window.draw(bar);
        for (const auto& orb : orbs) {
            float r = radius(orb);
            sf::CircleShape circle(r);
            circle.setOrigin(r, r);
            circle.setPosition(orb.at);
            circle.setFillColor(enabled ? orb.color : sf::Color(120, 120, 120));
            window.draw(circle);
        }
        window.display();
    }
};

OrbFieldPart* OrbField_create(void* parent, int width, int height, OrbFieldHost* host) {
    auto* part = new OrbFieldPart{host, sf::RenderWindow(static_cast<sf::WindowHandle>(reinterpret_cast<uintptr_t>(parent)))};
    OrbField_resize(part, width, height);
    part->restart();
    return part;
}

void OrbField_resize(OrbFieldPart* part, int width, int height) {
    part->window.setView(sf::View(sf::FloatRect(0.f, 0.f, static_cast<float>(width), static_cast<float>(height))));
}

void OrbField_frame(OrbFieldPart* part) {
    sf::Event event;
    while (part->window.pollEvent(event)) {
        if (event.type == sf::Event::MouseButtonPressed && event.mouseButton.button == sf::Mouse::Left) {
            part->click(part->window.mapPixelToCoords({event.mouseButton.x, event.mouseButton.y}));
        }
    }
    part->step(std::min(part->clock.restart().asSeconds(), 0.1f));
    part->draw();
}

void OrbField_destroy(OrbFieldPart* part) {
    delete part;
}

void OrbField_set_enabled(OrbFieldPart* part, bool enabled) {
    part->enabled = enabled;
}

void OrbField_set_speed(OrbFieldPart* part, int64_t speed) {
    part->speed = std::clamp<int64_t>(speed, 1, 10);
}

void OrbField_restart(OrbFieldPart* part) {
    part->restart();
}
