#ifndef POSE_HPP
#define POSE_HPP
#include "point.hpp"

class Pose : public Point {
public:
    double heading;
    Pose(double x, double y, double heading)
        : Point(x, y), heading(heading) {}
    double headingTo(const Point& other) const override {
        return Point::headingTo(other) - heading;
    }
};

#endif // POSE_HPP
