// Go example
package main

import (
    "math"
)

type Point struct {
    X float64
    Y float64
}

func NewPoint(x, y float64) *Point {
    return &Point{X: x, Y: y}
}

func (p *Point) DistanceTo(other *Point) float64 {
    dx := p.X - other.X
    dy := p.Y - other.Y
    return math.Hypot(dx, dy)
}

func (p *Point) HeadingTo(other *Point) float64 {
    return math.Atan2(other.Y-p.Y, other.X-p.X)
}

type Pose struct {
    Point
    Heading float64
}

func NewPose(x, y, heading float64) *Pose {
    return &Pose{Point: Point{X: x, Y: y}, Heading: heading}
}

func (p *Pose) HeadingTo(other *Point) float64 {
    return p.Point.HeadingTo(other) - p.Heading
}

type Direction int

const (
    North Direction = iota
    East
    South
    West
)

type Animal struct {
    Name string
    Pose Pose
}

func NewAnimal(name string, pose Pose) *Animal {
    return &Animal{Name: name, Pose: pose}
}

func (a *Animal) DistanceAndHeadingTo(other *Animal) (float64, float64) {
    dist := a.Pose.DistanceTo(&Point{X: other.Pose.X, Y: other.Pose.Y})
    heading := a.Pose.HeadingTo(&Point{X: other.Pose.X, Y: other.Pose.Y})
    return dist, heading
}

type Dog struct {
    Animal
}

func NewDog(name string, pose Pose) *Dog {
    return &Dog{Animal: Animal{Name: name, Pose: pose}}
}
