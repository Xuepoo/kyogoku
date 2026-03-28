#!/usr/bin/env python3
"""
Large file stress test for Kyogoku parser performance.
Tests parsing and serialization without LLM translation.
"""

import subprocess
import time
import os
import sys
import json

# Paths
PROJECT_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
STRESS_DIR = os.path.join(PROJECT_ROOT, "tests/perf-corpus/stress")
BINARY = os.path.join(PROJECT_ROOT, "target/release/kyogoku")

def generate_large_files():
    """Generate stress test files of various sizes."""
    os.makedirs(STRESS_DIR, exist_ok=True)
    
    sizes = [
        (1000, "1k"),
        (5000, "5k"),
        (10000, "10k"),
        (20000, "20k"),
    ]
    
    for num_lines, suffix in sizes:
        # TXT file
        txt_path = os.path.join(STRESS_DIR, f"stress_{suffix}.txt")
        with open(txt_path, 'w', encoding='utf-8') as f:
            for i in range(num_lines):
                f.write(f"Line {i}: 春眠不觉晓，处处闻啼鸟。\n")
        
        # SRT file
        srt_path = os.path.join(STRESS_DIR, f"stress_{suffix}.srt")
        with open(srt_path, 'w', encoding='utf-8') as f:
            for i in range(num_lines):
                h = i // 3600
                m = (i % 3600) // 60
                s = i % 60
                f.write(f"{i+1}\n")
                f.write(f"{h:02d}:{m:02d}:{s:02d},000 --> {h:02d}:{m:02d}:{s:02d},999\n")
                f.write(f"Subtitle line {i}: Content here.\n\n")
        
        print(f"Generated {suffix} files: TXT and SRT")

def test_parse_performance():
    """Test parsing performance on large files."""
    results = []
    
    # First, ensure release build
    print("Building release binary...")
    subprocess.run(
        ["cargo", "build", "--release", "-p", "kyogoku-cli"],
        cwd=PROJECT_ROOT,
        capture_output=True
    )
    
    # Test each file size
    for size in ["1k", "5k", "10k", "20k"]:
        for ext in ["txt", "srt"]:
            filepath = os.path.join(STRESS_DIR, f"stress_{size}.{ext}")
            if not os.path.exists(filepath):
                print(f"Skipping {filepath} (not found)")
                continue
            
            # Parse-only test using dry-run
            start = time.time()
            result = subprocess.run(
                [BINARY, "translate", filepath, "-o", "/tmp/kyogoku-stress-out", "--dry-run"],
                capture_output=True,
                text=True
            )
            elapsed = time.time() - start
            
            # Get file stats
            file_size = os.path.getsize(filepath)
            line_count = sum(1 for _ in open(filepath, encoding='utf-8'))
            
            results.append({
                "file": f"stress_{size}.{ext}",
                "lines": line_count,
                "size_kb": file_size / 1024,
                "parse_time_s": round(elapsed, 3),
                "lines_per_sec": round(line_count / elapsed, 0) if elapsed > 0 else 0,
                "success": result.returncode == 0
            })
            
            status = "✓" if result.returncode == 0 else "✗"
            print(f"{status} {filepath}: {elapsed:.3f}s ({line_count} lines)")

    return results

def main():
    print("=" * 60)
    print("Kyogoku Large File Stress Test")
    print("=" * 60)
    
    # Generate test files
    print("\n1. Generating stress test files...")
    generate_large_files()
    
    # Run parse performance test
    print("\n2. Testing parse performance...")
    results = test_parse_performance()
    
    # Summary
    print("\n" + "=" * 60)
    print("Results Summary")
    print("=" * 60)
    print(f"{'File':<25} {'Lines':>8} {'Size KB':>10} {'Time':>8} {'Lines/s':>10}")
    print("-" * 60)
    for r in results:
        print(f"{r['file']:<25} {r['lines']:>8} {r['size_kb']:>10.1f} {r['parse_time_s']:>7.3f}s {r['lines_per_sec']:>10.0f}")
    
    # Check all passed
    all_passed = all(r['success'] for r in results)
    print("\n" + ("✓ All tests passed!" if all_passed else "✗ Some tests failed!"))
    
    return 0 if all_passed else 1

if __name__ == "__main__":
    sys.exit(main())
