// C# example
public class Point {
    public double x, y;
    public Point(double x, double y) {
        this.x = x;
        this.y = y;
    }
}

public class MainClass {
    public static int Add(int a, int b) {
        return a + b;
    }
}

public enum Direction {
    North,
    East,
    South,
    West
}

public class Animal {
    public string name;
    public Pose pose;
    public Animal(string name, Pose pose) {
        this.name = name;
        this.pose = pose;
    }
    public (double dist, double heading) DistanceAndHeadingTo(Animal other) {
        double dist = this.pose.DistanceTo(new Point { x = other.pose.x, y = other.pose.y });
        double heading = this.pose.HeadingTo(new Point { x = other.pose.x, y = other.pose.y });
        return (dist, heading);
    }
}

public class Dog {
    public Animal animal;
    public Dog(string name, Pose pose) {
        this.animal = new Animal(name, pose);
    }
}
