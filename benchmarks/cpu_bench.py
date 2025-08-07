#!/usr/bin/env python3
"""
CPU Benchmark Script
Performs CPU-intensive rendering tasks to measure performance
"""

import numpy as np
import time
import json
import psutil
from tqdm import tqdm
from PIL import Image
import multiprocessing as mp
from datetime import datetime

class CPUBenchmark:
    def __init__(self):
        self.results = {
            "timestamp": datetime.now().isoformat(),
            "system_info": self.get_system_info(),
            "tests": {}
        }
    
    def get_system_info(self):
        """Gather system information"""
        return {
            "cpu_count": mp.cpu_count(),
            "cpu_freq": psutil.cpu_freq()._asdict() if psutil.cpu_freq() else None,
            "memory_total_gb": psutil.virtual_memory().total / (1024**3),
            "platform": {
                "processor": psutil.cpu_count(logical=False),
                "logical_cores": psutil.cpu_count(logical=True)
            }
        }
    
    def mandelbrot_set(self, width=2000, height=2000, max_iter=256):
        """Generate Mandelbrot set - CPU intensive calculation"""
        print(f"\n🔢 Running Mandelbrot Set Generation ({width}x{height}, {max_iter} iterations)...")
        
        start_time = time.perf_counter()
        
        # Create coordinate arrays
        x = np.linspace(-2.5, 1.5, width)
        y = np.linspace(-2.0, 2.0, height)
        X, Y = np.meshgrid(x, y)
        C = X + 1j * Y
        
        # Initialize result array
        Z = np.zeros_like(C)
        M = np.zeros(C.shape, dtype=int)
        
        # Compute Mandelbrot set
        for i in tqdm(range(max_iter), desc="Computing iterations"):
            mask = np.abs(Z) <= 2
            Z[mask] = Z[mask]**2 + C[mask]
            M[mask] = i
        
        end_time = time.perf_counter()
        elapsed = end_time - start_time
        
        # Convert to image for visual verification
        img = Image.fromarray(np.uint8(M * 255 / max_iter))
        img.save("mandelbrot_cpu.png")
        
        return {
            "name": "Mandelbrot Set",
            "time_seconds": elapsed,
            "dimensions": f"{width}x{height}",
            "iterations": max_iter,
            "pixels_per_second": (width * height) / elapsed,
            "output_file": "mandelbrot_cpu.png"
        }
    
    def matrix_operations(self, size=2000, iterations=10):
        """Perform intensive matrix operations"""
        print(f"\n🔢 Running Matrix Operations ({size}x{size} matrices, {iterations} iterations)...")
        
        operations_times = []
        
        for i in tqdm(range(iterations), desc="Matrix operations"):
            # Generate random matrices
            A = np.random.rand(size, size)
            B = np.random.rand(size, size)
            
            start_time = time.perf_counter()
            
            # Matrix multiplication
            C = np.dot(A, B)
            
            # Eigenvalue decomposition
            eigenvalues = np.linalg.eigvals(C[:100, :100])  # Use smaller subset for eigenvalues
            
            # SVD on a subset
            U, S, Vt = np.linalg.svd(C[:200, :200], full_matrices=False)
            
            # Inverse of a subset
            inv = np.linalg.inv(C[:500, :500])
            
            end_time = time.perf_counter()
            operations_times.append(end_time - start_time)
        
        avg_time = np.mean(operations_times)
        std_time = np.std(operations_times)
        
        return {
            "name": "Matrix Operations",
            "avg_time_seconds": avg_time,
            "std_time_seconds": std_time,
            "matrix_size": f"{size}x{size}",
            "iterations": iterations,
            "operations_per_second": 1 / avg_time,
            "flops_estimate": (2 * size**3) / avg_time  # Rough FLOPS estimate for matrix multiplication
        }
    
    def ray_tracing_scene(self, width=800, height=600, samples=50):
        """Simple ray tracing scene - CPU intensive per-pixel calculations"""
        print(f"\n🔢 Running Ray Tracing Scene ({width}x{height}, {samples} samples/pixel)...")
        
        start_time = time.perf_counter()
        
        # Simple sphere ray tracing
        image = np.zeros((height, width, 3))
        
        # Camera setup
        camera_pos = np.array([0, 0, -20])
        
        # Sphere properties
        sphere_center = np.array([0, 0, 0])
        sphere_radius = 5
        
        # Light source
        light_pos = np.array([10, 10, -10])
        
        for y in tqdm(range(height), desc="Rendering rows"):
            for x in range(width):
                # Calculate ray direction
                ray_dir = np.array([
                    (x - width/2) / width,
                    (y - height/2) / height,
                    1
                ])
                ray_dir = ray_dir / np.linalg.norm(ray_dir)
                
                # Ray-sphere intersection
                oc = camera_pos - sphere_center
                a = np.dot(ray_dir, ray_dir)
                b = 2.0 * np.dot(oc, ray_dir)
                c = np.dot(oc, oc) - sphere_radius**2
                discriminant = b**2 - 4*a*c
                
                if discriminant > 0:
                    # Hit the sphere
                    t = (-b - np.sqrt(discriminant)) / (2*a)
                    hit_point = camera_pos + t * ray_dir
                    normal = (hit_point - sphere_center) / sphere_radius
                    
                    # Simple lighting
                    light_dir = light_pos - hit_point
                    light_dir = light_dir / np.linalg.norm(light_dir)
                    
                    # Diffuse lighting
                    diffuse = max(0, np.dot(normal, light_dir))
                    
                    # Multiple samples for anti-aliasing
                    color = np.array([diffuse, diffuse * 0.8, diffuse * 0.6])
                    
                    for _ in range(samples):
                        # Add some noise for sampling effect
                        noise = np.random.normal(0, 0.01, 3)
                        color += np.clip(color + noise, 0, 1) / samples
                    
                    image[y, x] = np.clip(color * 255, 0, 255)
        
        end_time = time.perf_counter()
        elapsed = end_time - start_time
        
        # Save the rendered image
        img = Image.fromarray(np.uint8(image))
        img.save("raytraced_cpu.png")
        
        return {
            "name": "Ray Tracing",
            "time_seconds": elapsed,
            "resolution": f"{width}x{height}",
            "samples_per_pixel": samples,
            "pixels_per_second": (width * height) / elapsed,
            "rays_per_second": (width * height * samples) / elapsed,
            "output_file": "raytraced_cpu.png"
        }
    
    def prime_sieve(self, limit=10_000_000):
        """Sieve of Eratosthenes - CPU intensive algorithm"""
        print(f"\n🔢 Running Prime Number Sieve (up to {limit:,})...")
        
        start_time = time.perf_counter()
        
        # Create boolean array "is_prime" and initialize all entries as true
        is_prime = np.ones(limit + 1, dtype=bool)
        is_prime[0] = is_prime[1] = False
        
        for i in tqdm(range(2, int(np.sqrt(limit)) + 1), desc="Sieving"):
            if is_prime[i]:
                # Update all multiples of i
                is_prime[i*i:limit+1:i] = False
        
        # Count primes
        prime_count = np.sum(is_prime)
        
        end_time = time.perf_counter()
        elapsed = end_time - start_time
        
        return {
            "name": "Prime Sieve",
            "time_seconds": elapsed,
            "limit": limit,
            "primes_found": int(prime_count),
            "numbers_per_second": limit / elapsed
        }
    
    def run_all_benchmarks(self):
        """Run all CPU benchmarks"""
        print("=" * 60)
        print("🖥️  CPU BENCHMARK SUITE")
        print("=" * 60)
        
        # Run benchmarks
        self.results["tests"]["mandelbrot"] = self.mandelbrot_set()
        self.results["tests"]["matrix_ops"] = self.matrix_operations()
        self.results["tests"]["ray_tracing"] = self.ray_tracing_scene()
        self.results["tests"]["prime_sieve"] = self.prime_sieve()
        
        # Calculate overall score
        self.calculate_score()
        
        # Save results
        self.save_results()
        
        return self.results
    
    def calculate_score(self):
        """Calculate an overall performance score"""
        scores = []
        
        # Normalize each test to a score out of 100
        if "mandelbrot" in self.results["tests"]:
            # Base: 1M pixels/second = 100 points
            score = (self.results["tests"]["mandelbrot"]["pixels_per_second"] / 1_000_000) * 100
            scores.append(min(score, 200))  # Cap at 200
        
        if "matrix_ops" in self.results["tests"]:
            # Base: 1 GFLOPS = 100 points
            score = (self.results["tests"]["matrix_ops"]["flops_estimate"] / 1_000_000_000) * 100
            scores.append(min(score, 200))
        
        if "ray_tracing" in self.results["tests"]:
            # Base: 100k rays/second = 100 points
            score = (self.results["tests"]["ray_tracing"]["rays_per_second"] / 100_000) * 100
            scores.append(min(score, 200))
        
        if "prime_sieve" in self.results["tests"]:
            # Base: 10M numbers/second = 100 points
            score = (self.results["tests"]["prime_sieve"]["numbers_per_second"] / 10_000_000) * 100
            scores.append(min(score, 200))
        
        self.results["overall_score"] = {
            "total": sum(scores),
            "average": sum(scores) / len(scores) if scores else 0,
            "individual_scores": scores
        }
    
    def save_results(self):
        """Save benchmark results to JSON file"""
        with open("cpu_benchmark_results.json", "w") as f:
            json.dump(self.results, f, indent=2)
        
        print("\n" + "=" * 60)
        print("📊 CPU Benchmark Complete!")
        print(f"Overall Score: {self.results['overall_score']['average']:.2f}/100")
        print(f"Results saved to: cpu_benchmark_results.json")
        print("=" * 60)

if __name__ == "__main__":
    benchmark = CPUBenchmark()
    benchmark.run_all_benchmarks()
