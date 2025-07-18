// Rust example
use std::f64::consts::PI;

/// Represents a 2D point.
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    /// Creates a new Point.
    pub fn new(x: f64, y: f64) -> Self {
        Point { x, y }
    }
    /// Returns the Euclidean distance to another Point.
    pub fn distance_to(&self, other: &Point) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
    /// Returns the angle (in radians) from this point to another Point.
    pub fn heading_to(&self, other: &Point) -> f64 {
        (other.y - self.y).atan2(other.x - self.x)
    }
}

/// Represents a pose with position and heading.
pub struct Pose {
    pub point: Point,
    pub heading: f64,
}

impl Pose {
    /// Creates a new Pose with x, y, and heading.
    pub fn new(x: f64, y: f64, heading: f64) -> Self {
        Pose { point: Point { x, y }, heading }
    }
    /// Returns the distance to another Point.
    pub fn distance_to(&self, other: &Point) -> f64 {
        self.point.distance_to(other)
    }
    /// Returns the relative heading to another Point, adjusted by this pose's heading.
    pub fn heading_to(&self, other: &Point) -> f64 {
        self.point.heading_to(other) - self.heading
    }
}

/// Cardinal directions.
pub enum Direction {
    North,
    East,
    South,
    West,
}

/// Represents an animal with a name and pose.
pub struct Animal {
    pub name: String,
    pub pose: Pose,
}

impl Animal {
    /// Creates a new Animal with a name and pose.
    pub fn new(name: &str, pose: Pose) -> Self {
        Animal { name: name.to_string(), pose }
    }
    /// Returns the distance and heading to another Animal.
    pub fn distance_and_heading_to(&self, other: &Animal) -> (f64, f64) {
        let dist = self.pose.distance_to(&other.pose.point);
        let heading = self.pose.heading_to(&other.pose.point);
        (dist, heading)
    }
}

/// Represents a dog, which is a type of Animal.
pub struct Dog {
    pub animal: Animal,
}

impl Dog {
    /// Creates a new Dog with a name and pose.
    pub fn new(name: &str, pose: Pose) -> Self {
        Dog { animal: Animal::new(name, pose) }
    }
}

/// A trait for types that can be named.
pub trait Named {
    /// Returns the name of the object.
    fn name(&self) -> &str;
}

impl Named for Animal {
    fn name(&self) -> &str {
        &self.name
    }
}

/// A generic container for any type.
pub struct Container<T> {
    pub value: T,
}

impl<T> Container<T> {
    /// Creates a new Container.
    pub fn new(value: T) -> Self {
        Container { value }
    }
    /// Consumes the container and returns the value.
    pub fn into_inner(self) -> T {
        self.value
    }
}
