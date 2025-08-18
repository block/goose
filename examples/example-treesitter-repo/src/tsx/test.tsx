// TypeScript React example
import React from 'react';

type PointProps = { x: number; y: number };

const Point: React.FC<PointProps> = ({ x, y }) => (
    <div>{`(${x}, ${y})`}</div>
);

export function add(a: number, b: number): number {
    return a + b;
}
