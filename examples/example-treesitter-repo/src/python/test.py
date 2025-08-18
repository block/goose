# Python example
import math
from enum import Enum

def log_call(func):
    """
    A decorator that logs the function call with its arguments.
    """
    def wrapper(*args, **kwargs):
        print(f"Calling {func.__name__} with args={args}, kwargs={kwargs}")
        return func(*args, **kwargs)
    return wrapper

@log_call
def greet(name):
    """
    Greets a person by name.
    """
    print(f"Hello, {name}!")

def add(a, b):
    """
    Adds two numbers and returns the result.

    This function demonstrates a simple docstring with a summary line, a blank line,
    and a more detailed description. It also includes argument and return value documentation.

    Args:
        a (int or float): First number to add.
        b (int or float): Second number to add.

    Returns:
        int or float: The sum of a and b.
    """
    return a + b

class Point:
    """
    Represents a 2D point.

    This class demonstrates a class-level docstring with a summary and a longer description.
    """
    def __init__(self, x, y):
        """
        Initializes a Point with x and y coordinates.

        Args:
            x (float): The x coordinate.
            y (float): The y coordinate.
        """
        self.x = x
        self.y = y

    def distance_to(self, other):
        """
        Calculates the Euclidean distance to another Point.

        Args:
            other (Point): The other point.
        Returns:
            float: The Euclidean distance.
        """
        dx = self.x - other.x
        dy = self.y - other.y
        return math.hypot(dx, dy)

    def heading_to(self, other):
        """
        Returns the angle (in radians) from this point to another Point.

        Args:
            other (Point): The other point.
        Returns:
            float: The angle in radians.
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
