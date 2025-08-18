// Swift example
import Foundation

class Point {
    var x: Double
    var y: Double
    init(x: Double, y: Double) {
        self.x = x
        self.y = y
    }
    func distanceTo(_ other: Point) -> Double {
        let dx = x - other.x
        let dy = y - other.y
        return (dx * dx + dy * dy).squareRoot()
    }
    func headingTo(_ other: Point) -> Double {
        return atan2(other.y - y, other.x - x)
    }
}

class Pose: Point {
    var heading: Double
    init(x: Double, y: Double, heading: Double) {
        self.heading = heading
        super.init(x: x, y: y)
    }
    override func headingTo(_ other: Point) -> Double {
        return super.headingTo(other) - heading
    }
}

func add(a: Int, b: Int) -> Int {
    return a + b
}

enum Direction {
    case north
    case east
    case south
    case west
}

class Animal {
    var name: String
    var pose: Pose
    init(name: String, pose: Pose) {
        self.name = name
        self.pose = pose
    }
    func distanceAndHeadingTo(_ other: Animal) -> (Double, Double) {
        let dist = pose.distanceTo(Point(x: other.pose.x, y: other.pose.y))
        let heading = pose.headingTo(Point(x: other.pose.x, y: other.pose.y))
        return (dist, heading)
    }
}

class Dog: Animal {
    override init(name: String, pose: Pose) {
        super.init(name: name, pose: pose)
    }
}
