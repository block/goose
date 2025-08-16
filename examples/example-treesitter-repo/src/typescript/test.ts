// TypeScript example
function add(a: number, b: number): number {
    return a + b;
}

class Point {
    x: number;
    y: number;
    constructor(x: number, y: number) {
        this.x = x;
        this.y = y;
    }
    distanceTo(other: Point): number {
        const dx = this.x - other.x;
        const dy = this.y - other.y;
        return Math.hypot(dx, dy);
    }
    headingTo(other: Point): number {
        return Math.atan2(other.y - this.y, other.x - this.x);
    }
}

class Pose extends Point {
    heading: number;
    constructor(x: number, y: number, heading: number) {
        super(x, y);
        this.heading = heading;
    }
    headingTo(other: Point): number {
        return super.headingTo(other) - this.heading;
    }
}

enum Direction {
    North,
    East,
    South,
    West
}

class Animal {
    name: string;
    pose: Pose;
    constructor(name: string, pose: Pose) {
        this.name = name;
        this.pose = pose;
    }
    distanceAndHeadingTo(other: Animal): { dist: number; heading: number } {
        const dist = this.pose.distanceTo(new Point(other.pose.x, other.pose.y));
        const heading = this.pose.headingTo(new Point(other.pose.x, other.pose.y));
        return { dist, heading };
    }
}

class Dog extends Animal {
    constructor(name: string, pose: Pose) {
        super(name, pose);
    }
}

interface Named {
    getName(): string;
}

class Cat extends Animal implements Named {
    constructor(name: string, pose: Pose) {
        super(name, pose);
    }
    getName(): string {
        return this.name;
    }
}

class Box<T> {
    value: T;
    constructor(value: T) { this.value = value; }
    get(): T { return this.value; }
}
