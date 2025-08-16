// JavaScript example
function add(a, b) {
    return a + b;
}

class Point {
    constructor(x, y) {
        this.x = x;
        this.y = y;
    }
    distanceTo(other) {
        const dx = this.x - other.x;
        const dy = this.y - other.y;
        return Math.hypot(dx, dy);
    }
    headingTo(other) {
        return Math.atan2(other.y - this.y, other.x - this.x);
    }
}

class Pose extends Point {
    constructor(x, y, heading) {
        super(x, y);
        this.heading = heading;
    }
    headingTo(other) {
        return super.headingTo(other) - this.heading;
    }
}

const Direction = Object.freeze({
    North: 0,
    East: 1,
    South: 2,
    West: 3
});

class Animal {
    constructor(name, pose) {
        this.name = name;
        this.pose = pose;
    }
    distanceAndHeadingTo(other) {
        const dist = this.pose.distanceTo(new Point(other.pose.x, other.pose.y));
        const heading = this.pose.headingTo(new Point(other.pose.x, other.pose.y));
        return { dist, heading };
    }
}

class Dog extends Animal {
    constructor(name, pose) {
        super(name, pose);
    }
}
