#ifndef POINT_HPP
#define POINT_HPP
#include <cmath>

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

#endif // POINT_HPP
