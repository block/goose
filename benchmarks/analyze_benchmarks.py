#!/usr/bin/env python3
"""
Benchmark Analysis and Comparison Script
Loads results from CPU and GPU benchmarks and generates comprehensive comparison report
"""

import json
import numpy as np
import matplotlib.pyplot as plt
import matplotlib.patches as mpatches
from datetime import datetime
import os

class BenchmarkAnalyzer:
    def __init__(self):
        self.cpu_results = None
        self.gpu_results = None
        self.comparison = {}
        
    def load_results(self):
        """Load benchmark results from JSON files"""
        try:
            with open("cpu_benchmark_results.json", "r") as f:
                self.cpu_results = json.load(f)
            print("✅ Loaded CPU benchmark results")
        except FileNotFoundError:
            print("❌ CPU benchmark results not found. Please run cpu_bench.py first.")
            return False
        
        try:
            with open("gpu_benchmark_results.json", "r") as f:
                self.gpu_results = json.load(f)
            print("✅ Loaded GPU benchmark results")
        except FileNotFoundError:
            print("❌ GPU benchmark results not found. Please run gpu_bench.py first.")
            return False
        
        return True
    
    def analyze_performance(self):
        """Analyze and compare performance between CPU and GPU"""
        
        # Mandelbrot comparison
        if "mandelbrot" in self.cpu_results["tests"] and "mandelbrot" in self.gpu_results["tests"]:
            cpu_mandel = self.cpu_results["tests"]["mandelbrot"]
            gpu_mandel = self.gpu_results["tests"]["mandelbrot"]
            
            self.comparison["mandelbrot"] = {
                "cpu_time": cpu_mandel["time_seconds"],
                "gpu_time": gpu_mandel["time_seconds"],
                "speedup": cpu_mandel["time_seconds"] / gpu_mandel["time_seconds"],
                "cpu_pixels_per_sec": cpu_mandel["pixels_per_second"],
                "gpu_pixels_per_sec": gpu_mandel["pixels_per_second"],
                "performance_ratio": gpu_mandel["pixels_per_second"] / cpu_mandel["pixels_per_second"]
            }
        
        # Matrix operations comparison
        if "matrix_ops" in self.cpu_results["tests"] and "matrix_multiply" in self.gpu_results["tests"]:
            cpu_matrix = self.cpu_results["tests"]["matrix_ops"]
            gpu_matrix = self.gpu_results["tests"]["matrix_multiply"]
            
            self.comparison["matrix_operations"] = {
                "cpu_time": cpu_matrix["avg_time_seconds"],
                "gpu_time": gpu_matrix["avg_time_seconds"],
                "speedup": cpu_matrix["avg_time_seconds"] / gpu_matrix["avg_time_seconds"],
                "cpu_flops": cpu_matrix["flops_estimate"],
                "gpu_tflops": gpu_matrix["tflops"],
                "performance_ratio": (gpu_matrix["tflops"] * 1e12) / cpu_matrix["flops_estimate"]
            }
        
        # Overall scores
        self.comparison["overall_scores"] = {
            "cpu_score": self.cpu_results["overall_score"]["average"],
            "gpu_score": self.gpu_results["overall_score"]["average"],
            "score_ratio": self.gpu_results["overall_score"]["average"] / self.cpu_results["overall_score"]["average"]
        }
    
    def create_visualizations(self):
        """Create comparison charts"""
        
        # Set up the figure with subplots
        fig = plt.figure(figsize=(16, 12))
        fig.suptitle("CPU vs GPU Benchmark Comparison", fontsize=16, fontweight='bold')
        
        # 1. Time Comparison Bar Chart
        ax1 = plt.subplot(2, 3, 1)
        if "mandelbrot" in self.comparison:
            tests = ['Mandelbrot', 'Matrix Ops']
            cpu_times = [
                self.comparison["mandelbrot"]["cpu_time"],
                self.comparison["matrix_operations"]["cpu_time"] if "matrix_operations" in self.comparison else 0
            ]
            gpu_times = [
                self.comparison["mandelbrot"]["gpu_time"],
                self.comparison["matrix_operations"]["gpu_time"] if "matrix_operations" in self.comparison else 0
            ]
            
            x = np.arange(len(tests))
            width = 0.35
            
            bars1 = ax1.bar(x - width/2, cpu_times, width, label='CPU', color='#3498db')
            bars2 = ax1.bar(x + width/2, gpu_times, width, label='GPU (Simulated)', color='#e74c3c')
            
            ax1.set_xlabel('Test')
            ax1.set_ylabel('Time (seconds)')
            ax1.set_title('Execution Time Comparison (Lower is Better)')
            ax1.set_xticks(x)
            ax1.set_xticklabels(tests)
            ax1.legend()
            ax1.grid(True, alpha=0.3)
            
            # Add value labels on bars
            for bar in bars1:
                height = bar.get_height()
                ax1.text(bar.get_x() + bar.get_width()/2., height,
                        f'{height:.2f}s', ha='center', va='bottom', fontsize=9)
            for bar in bars2:
                height = bar.get_height()
                ax1.text(bar.get_x() + bar.get_width()/2., height,
                        f'{height:.2f}s', ha='center', va='bottom', fontsize=9)
        
        # 2. Speedup Chart
        ax2 = plt.subplot(2, 3, 2)
        if "mandelbrot" in self.comparison:
            speedups = []
            labels = []
            
            if "mandelbrot" in self.comparison:
                speedups.append(self.comparison["mandelbrot"]["speedup"])
                labels.append("Mandelbrot")
            
            if "matrix_operations" in self.comparison:
                speedups.append(self.comparison["matrix_operations"]["speedup"])
                labels.append("Matrix Ops")
            
            colors = ['#2ecc71' if s > 1 else '#e67e22' for s in speedups]
            bars = ax2.bar(labels, speedups, color=colors)
            ax2.axhline(y=1, color='black', linestyle='--', alpha=0.5, label='Equal Performance')
            ax2.set_ylabel('Speedup Factor')
            ax2.set_title('GPU Speedup over CPU (Higher is Better)')
            ax2.legend()
            ax2.grid(True, alpha=0.3)
            
            # Add value labels
            for bar, speedup in zip(bars, speedups):
                height = bar.get_height()
                ax2.text(bar.get_x() + bar.get_width()/2., height,
                        f'{speedup:.2f}x', ha='center', va='bottom', fontsize=10, fontweight='bold')
        
        # 3. Throughput Comparison
        ax3 = plt.subplot(2, 3, 3)
        if "mandelbrot" in self.comparison:
            metrics = ['Pixels/sec\n(Mandelbrot)', 'FLOPS\n(Matrix)']
            cpu_throughput = [
                self.comparison["mandelbrot"]["cpu_pixels_per_sec"] / 1e6,  # Convert to millions
                self.cpu_results["tests"]["matrix_ops"]["flops_estimate"] / 1e9 if "matrix_ops" in self.cpu_results["tests"] else 0  # Convert to GFLOPS
            ]
            gpu_throughput = [
                self.comparison["mandelbrot"]["gpu_pixels_per_sec"] / 1e6,  # Convert to millions
                self.gpu_results["tests"]["matrix_multiply"]["tflops"] * 1000 if "matrix_multiply" in self.gpu_results["tests"] else 0  # Convert to GFLOPS
            ]
            
            x = np.arange(len(metrics))
            width = 0.35
            
            bars1 = ax3.bar(x - width/2, cpu_throughput, width, label='CPU', color='#9b59b6')
            bars2 = ax3.bar(x + width/2, gpu_throughput, width, label='GPU (Simulated)', color='#f39c12')
            
            ax3.set_xlabel('Metric')
            ax3.set_ylabel('Throughput (Millions/GFLOPS)')
            ax3.set_title('Throughput Comparison (Higher is Better)')
            ax3.set_xticks(x)
            ax3.set_xticklabels(metrics)
            ax3.legend()
            ax3.grid(True, alpha=0.3)
            
            # Add value labels
            for bar, val in zip(bars1, cpu_throughput):
                height = bar.get_height()
                ax3.text(bar.get_x() + bar.get_width()/2., height,
                        f'{val:.1f}', ha='center', va='bottom', fontsize=9)
            for bar, val in zip(bars2, gpu_throughput):
                height = bar.get_height()
                ax3.text(bar.get_x() + bar.get_width()/2., height,
                        f'{val:.1f}', ha='center', va='bottom', fontsize=9)
        
        # 4. Overall Score Comparison
        ax4 = plt.subplot(2, 3, 4)
        scores = [
            self.comparison["overall_scores"]["cpu_score"],
            self.comparison["overall_scores"]["gpu_score"]
        ]
        labels = ['CPU', 'GPU (Simulated)']
        colors = ['#3498db', '#e74c3c']
        
        bars = ax4.bar(labels, scores, color=colors)
        ax4.set_ylabel('Score')
        ax4.set_title('Overall Performance Score (Higher is Better)')
        ax4.set_ylim(0, max(scores) * 1.2)
        ax4.grid(True, alpha=0.3)
        
        # Add value labels
        for bar, score in zip(bars, scores):
            height = bar.get_height()
            ax4.text(bar.get_x() + bar.get_width()/2., height,
                    f'{score:.1f}', ha='center', va='bottom', fontsize=12, fontweight='bold')
        
        # 5. Performance Ratio Pie Chart
        ax5 = plt.subplot(2, 3, 5)
        if self.comparison["overall_scores"]["score_ratio"] > 1:
            sizes = [1, self.comparison["overall_scores"]["score_ratio"] - 1]
            labels_pie = ['CPU Baseline', 'GPU Advantage']
            colors_pie = ['#95a5a6', '#27ae60']
        else:
            sizes = [self.comparison["overall_scores"]["score_ratio"], 1 - self.comparison["overall_scores"]["score_ratio"]]
            labels_pie = ['GPU Performance', 'CPU Advantage']
            colors_pie = ['#e74c3c', '#3498db']
        
        wedges, texts, autotexts = ax5.pie(sizes, labels=labels_pie, colors=colors_pie, 
                                            autopct='%1.1f%%', startangle=90)
        ax5.set_title(f'Relative Performance\n(GPU/CPU Ratio: {self.comparison["overall_scores"]["score_ratio"]:.2f}x)')
        
        # 6. Test Details Table
        ax6 = plt.subplot(2, 3, 6)
        ax6.axis('tight')
        ax6.axis('off')
        
        # Create table data
        table_data = [
            ['Metric', 'CPU', 'GPU', 'Ratio'],
            ['Overall Score', f'{self.comparison["overall_scores"]["cpu_score"]:.1f}', 
             f'{self.comparison["overall_scores"]["gpu_score"]:.1f}',
             f'{self.comparison["overall_scores"]["score_ratio"]:.2f}x'],
        ]
        
        if "mandelbrot" in self.comparison:
            table_data.append(['Mandelbrot (px/s)', 
                             f'{self.comparison["mandelbrot"]["cpu_pixels_per_sec"]:.0f}',
                             f'{self.comparison["mandelbrot"]["gpu_pixels_per_sec"]:.0f}',
                             f'{self.comparison["mandelbrot"]["performance_ratio"]:.2f}x'])
        
        if "matrix_operations" in self.comparison:
            table_data.append(['Matrix FLOPS', 
                             f'{self.comparison["matrix_operations"]["cpu_flops"]:.2e}',
                             f'{self.comparison["matrix_operations"]["gpu_tflops"]:.3f} TF',
                             f'{self.comparison["matrix_operations"]["performance_ratio"]:.2f}x'])
        
        table = ax6.table(cellText=table_data, loc='center', cellLoc='center')
        table.auto_set_font_size(False)
        table.set_fontsize(10)
        table.scale(1.2, 1.5)
        
        # Style the header row
        for i in range(4):
            table[(0, i)].set_facecolor('#34495e')
            table[(0, i)].set_text_props(weight='bold', color='white')
        
        ax6.set_title('Performance Summary', fontweight='bold', pad=20)
        
        plt.tight_layout()
        plt.savefig('benchmark_comparison.png', dpi=150, bbox_inches='tight')
        print("📊 Saved visualization to benchmark_comparison.png")
        
        return fig
    
    def generate_report(self):
        """Generate comprehensive text report"""
        
        report = []
        report.append("=" * 80)
        report.append("BENCHMARK ANALYSIS REPORT")
        report.append("=" * 80)
        report.append(f"Generated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
        report.append("")
        
        # System Information
        report.append("SYSTEM INFORMATION")
        report.append("-" * 40)
        report.append(f"CPU Cores: {self.cpu_results['system_info']['cpu_count']}")
        report.append(f"Memory: {self.cpu_results['system_info']['memory_total_gb']:.2f} GB")
        if self.cpu_results['system_info']['cpu_freq']:
            report.append(f"CPU Frequency: {self.cpu_results['system_info']['cpu_freq']['current']:.2f} MHz")
        report.append("")
        
        # Overall Performance
        report.append("OVERALL PERFORMANCE")
        report.append("-" * 40)
        report.append(f"CPU Score: {self.comparison['overall_scores']['cpu_score']:.2f}/100")
        report.append(f"GPU Score (Simulated): {self.comparison['overall_scores']['gpu_score']:.2f}/100")
        report.append(f"GPU/CPU Performance Ratio: {self.comparison['overall_scores']['score_ratio']:.2f}x")
        
        if self.comparison['overall_scores']['score_ratio'] > 1:
            report.append(f"✅ GPU shows {(self.comparison['overall_scores']['score_ratio'] - 1) * 100:.1f}% better overall performance")
        else:
            report.append(f"⚠️  CPU shows {(1 - self.comparison['overall_scores']['score_ratio']) * 100:.1f}% better overall performance")
        report.append("")
        
        # Detailed Test Results
        report.append("DETAILED TEST COMPARISONS")
        report.append("-" * 40)
        
        # Mandelbrot
        if "mandelbrot" in self.comparison:
            report.append("\n1. MANDELBROT SET GENERATION")
            report.append(f"   CPU Time: {self.comparison['mandelbrot']['cpu_time']:.2f} seconds")
            report.append(f"   GPU Time: {self.comparison['mandelbrot']['gpu_time']:.2f} seconds")
            report.append(f"   Speedup: {self.comparison['mandelbrot']['speedup']:.2f}x")
            report.append(f"   CPU Throughput: {self.comparison['mandelbrot']['cpu_pixels_per_sec']:,.0f} pixels/sec")
            report.append(f"   GPU Throughput: {self.comparison['mandelbrot']['gpu_pixels_per_sec']:,.0f} pixels/sec")
            report.append(f"   Performance Gain: {(self.comparison['mandelbrot']['performance_ratio'] - 1) * 100:.1f}%")
        
        # Matrix Operations
        if "matrix_operations" in self.comparison:
            report.append("\n2. MATRIX OPERATIONS")
            report.append(f"   CPU Time: {self.comparison['matrix_operations']['cpu_time']:.2f} seconds")
            report.append(f"   GPU Time: {self.comparison['matrix_operations']['gpu_time']:.2f} seconds")
            report.append(f"   Speedup: {self.comparison['matrix_operations']['speedup']:.2f}x")
            report.append(f"   CPU FLOPS: {self.comparison['matrix_operations']['cpu_flops']:.2e}")
            report.append(f"   GPU TFLOPS: {self.comparison['matrix_operations']['gpu_tflops']:.3f}")
            report.append(f"   Performance Gain: {(self.comparison['matrix_operations']['performance_ratio'] - 1) * 100:.1f}%")
        
        # CPU-specific tests
        report.append("\n3. CPU-SPECIFIC TESTS")
        if "ray_tracing" in self.cpu_results["tests"]:
            rt = self.cpu_results["tests"]["ray_tracing"]
            report.append(f"   Ray Tracing: {rt['time_seconds']:.2f}s, {rt['rays_per_second']:,.0f} rays/sec")
        
        if "prime_sieve" in self.cpu_results["tests"]:
            ps = self.cpu_results["tests"]["prime_sieve"]
            report.append(f"   Prime Sieve: {ps['time_seconds']:.2f}s, {ps['primes_found']:,} primes found")
        
        # GPU-specific tests
        report.append("\n4. GPU-SPECIFIC TESTS (Simulated)")
        if "particle_sim" in self.gpu_results["tests"]:
            ps = self.gpu_results["tests"]["particle_sim"]
            report.append(f"   Particle Simulation: {ps['time_seconds']:.2f}s, {ps['interactions_per_second']:.2e} interactions/sec")
        
        if "convolution" in self.gpu_results["tests"]:
            conv = self.gpu_results["tests"]["convolution"]
            report.append(f"   Convolution Filters: {conv['time_seconds']:.2f}s, {conv['gflops']:.2f} GFLOPS")
        
        if "vector_field" in self.gpu_results["tests"]:
            vf = self.gpu_results["tests"]["vector_field"]
            report.append(f"   Vector Field: {vf['time_seconds']:.2f}s, {vf['pixels_per_second']:,.0f} pixels/sec")
        
        report.append("")
        report.append("ANALYSIS SUMMARY")
        report.append("-" * 40)
        
        # Determine strengths
        report.append("\n💪 STRENGTHS:")
        if self.comparison['overall_scores']['score_ratio'] > 1:
            report.append("• GPU (Simulated) shows superior parallel processing capabilities")
            report.append("• Excellent for highly parallelizable workloads")
            report.append("• Better throughput on large-scale computations")
        else:
            report.append("• CPU shows strong single-threaded performance")
            report.append("• Better for sequential and complex branching operations")
            report.append("• More versatile for general-purpose computing")
        
        report.append("\n📊 RECOMMENDATIONS:")
        report.append("• Use GPU for: Image processing, matrix operations, simulations, ML training")
        report.append("• Use CPU for: Complex algorithms, small datasets, sequential processing")
        report.append("• Consider hybrid approaches for optimal performance")
        
        report.append("\n📝 NOTE:")
        report.append("GPU benchmarks are simulated using vectorized NumPy operations.")
        report.append("Actual GPU performance would typically be significantly higher with")
        report.append("proper CUDA/OpenCL/Metal implementations.")
        
        report.append("")
        report.append("=" * 80)
        report.append("END OF REPORT")
        report.append("=" * 80)
        
        # Save report
        report_text = "\n".join(report)
        with open("benchmark_report.txt", "w") as f:
            f.write(report_text)
        
        print("\n" + report_text)
        print("\n📄 Report saved to benchmark_report.txt")
        
        return report_text
    
    def run_analysis(self):
        """Main analysis pipeline"""
        print("=" * 60)
        print("🔍 BENCHMARK ANALYZER")
        print("=" * 60)
        
        if not self.load_results():
            return False
        
        print("\n📊 Analyzing performance data...")
        self.analyze_performance()
        
        print("📈 Creating visualizations...")
        self.create_visualizations()
        
        print("\n📝 Generating report...")
        self.generate_report()
        
        print("\n✅ Analysis complete!")
        print("\nGenerated files:")
        print("  • benchmark_comparison.png - Visual comparison charts")
        print("  • benchmark_report.txt - Detailed text report")
        
        return True

if __name__ == "__main__":
    analyzer = BenchmarkAnalyzer()
    analyzer.run_analysis()
