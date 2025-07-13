// Java example
public class Point {
    public double x, y;
    public Point(double x, double y) {
        this.x = x;
        this.y = y;
    }
    public double distanceTo(Point other) {
        double dx = x - other.x;
        double dy = y - other.y;
        return Math.hypot(dx, dy);
    }
    public double headingTo(Point other) {
        return Math.atan2(other.y - y, other.x - x);
    }
}

class Pose extends Point {
    public double heading;
    public Pose(double x, double y, double heading) {
        super(x, y);
        this.heading = heading;
    }
    @Override
    public double headingTo(Point other) {
        return super.headingTo(other) - heading;
    }
}

class Main {
    public static int add(int a, int b) {
        return a + b;
    }
}

enum Direction {
    NORTH,
    EAST,
    SOUTH,
    WEST
}

class Animal {
    public String name;
    public Pose pose;
    public Animal(String name, Pose pose) {
        this.name = name;
        this.pose = pose;
    }
    public double[] distanceAndHeadingTo(Animal other) {
        double dist = this.pose.distanceTo(new Point(other.pose.x, other.pose.y));
        double heading = this.pose.headingTo(new Point(other.pose.x, other.pose.y));
        return new double[] { dist, heading };
    }
}

class Dog extends Animal {
    public Dog(String name, Pose pose) {
        super(name, pose);
    }
}

interface Named {
    String getName();
}

class Cat extends Animal implements Named {
    public Cat(String name, Pose pose) {
        super(name, pose);
    }
    @Override
    public String getName() {
        return name;
    }
}

class Box<T> {
    private T value;
    public Box(T value) { this.value = value; }
    public T get() { return value; }
}
