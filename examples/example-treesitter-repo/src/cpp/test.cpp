#include "point.hpp"
#include "pose.hpp"
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

template <typename T>
class Box {
public:
    T value;
    Box(const T& value) : value(value) {}
    T get() const { return value; }
};

class Named {
public:
    virtual std::string getName() const = 0;
};

class Cat : public Animal, public Named {
public:
    Cat(const std::string& name, const Pose& pose) : Animal(name, pose) {}
    std::string getName() const override { return name; }
};
