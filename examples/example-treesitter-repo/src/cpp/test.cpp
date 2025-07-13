// C++ example
#include <iostream>
#include <cmath>
#include <string>
#include <utility>

enum class Direction {
    North,
    East,
    South,
    West
};

int add(int a, int b) {
    return a + b;
}

class Point {
public:
    double x, y;
    Point(double x, double y) : x(x), y(y) {}

    double distanceTo(const Point& other) const {
        double dx = x - other.x;
        double dy = y - other.y;
        return std::sqrt(dx * dx + dy * dy);
    }

    double headingTo(const Point& other) const {
        return std::atan2(other.y - y, other.x - x);
    }
};

class Pose : public Point {
public:
    double heading;
    Pose(double x, double y, double heading)
        : Point(x, y), heading(heading) {}

    double headingTo(const Point& other) const override {
        // Adjust the heading by the pose's heading
        return Point::headingTo(other) - heading;
    }
};

class Animal {
public:
    std::string name;
    Pose pose;
    Animal(const std::string& name, const Pose& pose) : name(name), pose(pose) {}
    std::pair<double, double> distance_and_heading_to(const Animal& other) const {
        double dist = pose.distanceTo(Point(other.pose.x, other.pose.y));
        double heading = pose.headingTo(Point(other.pose.x, other.pose.y));
        return {dist, heading};
    }
};

class Dog : public Animal {
public:
    Dog(const std::string& name, const Pose& pose) : Animal(name, pose) {}
};
