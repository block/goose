# Python example
import math
from enum import Enum

def add(a, b):
    """
    Adds two numbers and returns the result.
    
    Args:
        a: First number.
        b: Second number.
    Returns:
        Sum of a and b.
    """
    return a + b

class Point:
    """
    Represents a 2D point.
    """
    def __init__(self, x, y):
        """
        Initializes a Point with x and y coordinates.
        """
        self.x = x
        self.y = y

    def distance_to(self, other):
        """
        Calculates the Euclidean distance to another Point.
        """
        dx = self.x - other.x
        dy = self.y - other.y
        return math.hypot(dx, dy)

    def heading_to(self, other):
        """
        Returns the angle (in radians) from this point to another Point.
        """
        return math.atan2(other.y - self.y, other.x - self.x)

class Pose(Point):
    """
    Represents a pose with position and heading.
    """
    def __init__(self, x, y, heading):
        """
        Initializes a Pose with x, y, and heading.
        """
        super().__init__(x, y)
        self.heading = heading

    def heading_to(self, other):
        """
        Returns the relative heading to another Point, adjusted by this pose's heading.
        """
        return super().heading_to(other) - self.heading

class Direction(Enum):
    """
    Cardinal directions.
    """
    NORTH = 1
    EAST = 2
    SOUTH = 3
    WEST = 4

class Animal:
    """
    Represents an animal with a name and pose.
    """
    def __init__(self, name, pose):
        """
        Initializes an Animal with a name and pose.
        """
        self.name = name
        self.pose = pose
    def distance_and_heading_to(self, other):
        """
        Returns the distance and heading to another Animal.
        """
        dist = self.pose.distance_to(Point(other.pose.x, other.pose.y))
        heading = self.pose.heading_to(Point(other.pose.x, other.pose.y))
        return dist, heading

class Dog(Animal):
    """
    Represents a dog, which is a type of Animal.
    """
    def __init__(self, name, pose):
        """
        Initializes a Dog with a name and pose.
        """
        super().__init__(name, pose)
