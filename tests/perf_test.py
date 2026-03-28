#!/usr/bin/env python3
"""
Performance test runner for Kyogoku translation engine.
Tests various file formats and sizes, logs metrics to CSV.
"""

import subprocess
import time
import json
import csv
import os
import sys
from pathlib import Path
from typing import Dict, List, Optional
import psutil

# Token pricing per 1M tokens (input, output)
TOKEN_PRICING = {
    'google/gemini-2.5-flash': (0.075, 0.30),
    'gpt-4o': (2.50, 10.00),
    'gpt-4o-mini': (0.15, 0.60),
}

class PerfTestRunner:
    def __init__(self, corpus_dir: Path, output_dir: Path, csv_path: Path):
        self.corpus_dir = corpus_dir
        self.output_dir = output_dir
        self.csv_path = csv_path
        self.results: List[Dict] = []
        
    def get_file_size(self, path: Path) -> int:
        """Get file size in bytes."""
        return path.stat().st_size
    
    def count_dialogue_blocks(self, path: Path) -> int:
        """Estimate dialogue blocks based on format."""
        ext = path.suffix.lower()
        content = path.read_text(encoding='utf-8')
        
        if ext == '.txt':
            # Count non-empty lines
            return len([l for l in content.splitlines() if l.strip()])
        elif ext == '.srt':
            # Count subtitle entries (lines starting with digit)
            return len([l for l in content.splitlines() if l.strip() and l[0].isdigit() and '-->' not in l])
        elif ext == '.ass':
            # Count Dialogue: lines
            return len([l for l in content.splitlines() if l.startswith('Dialogue:')])
        elif ext == '.vtt':
            # Count timestamp lines
            return len([l for l in content.splitlines() if '-->' in l])
        elif ext == '.json':
            # Count messages
            try:
                data = json.loads(content)
                if isinstance(data, dict) and 'messages' in data:
                    return len(data['messages'])
                return len(data) if isinstance(data, list) else 1
            except:
                return 0
        elif ext == '.rpy':
            # Count dialogue lines (character + quoted text)
            return len([l for l in content.splitlines() if '"' in l and not l.strip().startswith('#')])
        return 0
    
    def run_translation(self, input_path: Path, model: str = 'google/gemini-2.5-flash') -> Optional[Dict]:
        """Run translation and collect metrics.
        Note: model parameter is for cost calculation only; actual model used is from config."""
        output_name = f"{input_path.stem}_translated{input_path.suffix}"
        output_path = self.output_dir / input_path.name  # CLI outputs to dir with same name
        
        # Ensure output directory exists
        self.output_dir.mkdir(parents=True, exist_ok=True)
        
        # Get initial process memory
        process = psutil.Process()
        mem_before = process.memory_info().rss / 1024 / 1024  # MB
        
        # Build command (note: model is read from config, not CLI arg)
        cmd = [
            './target/release/kyogoku',
            'translate',
            str(input_path),
            '-o', str(self.output_dir)
        ]
        
        print(f"Running: {input_path.name} ({model})...", flush=True)
        
        # Run translation
        start_time = time.time()
        try:
            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                timeout=600,  # 10 min timeout
                cwd=Path(__file__).parent.parent,
                env={**os.environ}  # Pass through env vars including API keys
            )
            elapsed = time.time() - start_time
            
            if result.returncode != 0:
                print(f"❌ Failed: {result.stderr}", flush=True)
                return None
            
            # Get final memory
            mem_after = process.memory_info().rss / 1024 / 1024  # MB
            mem_peak = mem_after  # Approximate (psutil doesn't track peak easily)
            
            # Parse output for token counts if available
            stdout = result.stdout
            input_tokens = 0
            output_tokens = 0
            
            # Try to extract token info from logs (if implemented)
            for line in stdout.splitlines():
                if 'tokens' in line.lower():
                    # Parse token counts if logged
                    pass
            
            # Estimate tokens if not available (very rough estimate)
            if input_tokens == 0:
                # Rough estimate: ~4 chars per token for Japanese
                input_text = input_path.read_text(encoding='utf-8')
                input_tokens = len(input_text) // 4
                
                if output_path.exists():
                    output_text = output_path.read_text(encoding='utf-8')
                    output_tokens = len(output_text) // 4
            
            # Calculate cost
            if model in TOKEN_PRICING:
                input_price, output_price = TOKEN_PRICING[model]
                cost_usd = (input_tokens / 1_000_000 * input_price + 
                           output_tokens / 1_000_000 * output_price)
            else:
                cost_usd = 0.0
            
            metrics = {
                'file': input_path.name,
                'format': input_path.suffix[1:],
                'size': input_path.stat().st_size,
                'dialogue_blocks': self.count_dialogue_blocks(input_path),
                'model': model,
                'elapsed_sec': round(elapsed, 2),
                'input_tokens': input_tokens,
                'output_tokens': output_tokens,
                'total_tokens': input_tokens + output_tokens,
                'cost_usd': round(cost_usd, 6),
                'mem_before_mb': round(mem_before, 1),
                'mem_after_mb': round(mem_after, 1),
                'mem_delta_mb': round(mem_after - mem_before, 1),
                'success': True,
                'timestamp': time.strftime('%Y-%m-%d %H:%M:%S')
            }
            
            print(f"✓ Completed in {elapsed:.1f}s | {metrics['dialogue_blocks']} blocks | ${cost_usd:.6f}", flush=True)
            return metrics
            
        except subprocess.TimeoutExpired:
            print(f"❌ Timeout after 10 minutes", flush=True)
            return None
        except Exception as e:
            print(f"❌ Error: {e}", flush=True)
            return None
    
    def run_all_tests(self, size_filter: Optional[str] = None, model: str = 'google/gemini-2.5-flash'):
        """Run all performance tests."""
        sizes = ['short', 'medium', 'long'] if not size_filter else [size_filter]
        
        for size in sizes:
            size_dir = self.corpus_dir / size
            if not size_dir.exists():
                continue
                
            print(f"\n{'='*60}")
            print(f"Testing {size.upper()} corpus")
            print(f"{'='*60}\n")
            
            for test_file in sorted(size_dir.glob('*.*')):
                if test_file.suffix in ['.txt', '.srt', '.ass', '.vtt', '.json', '.rpy']:
                    metrics = self.run_translation(test_file, model=model)
                    if metrics:
                        self.results.append(metrics)
                        self.save_results()  # Save after each test
    
    def save_results(self):
        """Save results to CSV."""
        if not self.results:
            return
        
        fieldnames = [
            'timestamp', 'file', 'format', 'size', 'dialogue_blocks',
            'model', 'elapsed_sec', 'input_tokens', 'output_tokens',
            'total_tokens', 'cost_usd', 'mem_before_mb', 'mem_after_mb',
            'mem_delta_mb', 'success'
        ]
        
        # Write/append to CSV
        write_header = not self.csv_path.exists()
        with open(self.csv_path, 'a', newline='', encoding='utf-8') as f:
            writer = csv.DictWriter(f, fieldnames=fieldnames)
            if write_header:
                writer.writeheader()
            for result in self.results[-1:]:  # Write only the last result (incremental)
                writer.writerow(result)
        
        print(f"\nResults saved to {self.csv_path}")
    
    def print_summary(self):
        """Print test summary."""
        if not self.results:
            print("\nNo results to summarize.")
            return
        
        print(f"\n{'='*60}")
        print("PERFORMANCE TEST SUMMARY")
        print(f"{'='*60}\n")
        
        total_cost = sum(r['cost_usd'] for r in self.results)
        total_time = sum(r['elapsed_sec'] for r in self.results)
        total_blocks = sum(r['dialogue_blocks'] for r in self.results)
        total_tokens = sum(r['total_tokens'] for r in self.results)
        
        print(f"Tests run: {len(self.results)}")
        print(f"Total time: {total_time:.1f}s ({total_time/60:.1f} min)")
        print(f"Total cost: ${total_cost:.6f}")
        print(f"Total blocks: {total_blocks}")
        print(f"Total tokens: {total_tokens:,}")
        print(f"Avg time/block: {total_time/total_blocks:.3f}s" if total_blocks > 0 else "")
        print(f"Avg cost/block: ${total_cost/total_blocks:.6f}" if total_blocks > 0 else "")
        print()
        
        # Per-format breakdown
        by_format = {}
        for r in self.results:
            fmt = r['format']
            if fmt not in by_format:
                by_format[fmt] = []
            by_format[fmt].append(r)
        
        print("By Format:")
        for fmt in sorted(by_format.keys()):
            results = by_format[fmt]
            avg_time = sum(r['elapsed_sec'] for r in results) / len(results)
            avg_cost = sum(r['cost_usd'] for r in results) / len(results)
            total = sum(r['dialogue_blocks'] for r in results)
            print(f"  {fmt:>5s}: {len(results)} files | {avg_time:>6.1f}s avg | ${avg_cost:.6f} avg | {total} blocks")
        
        print(f"\nDetailed results in: {self.csv_path}")


def main():
    """Main entry point."""
    import argparse
    
    parser = argparse.ArgumentParser(description='Run Kyogoku performance tests')
    parser.add_argument('--size', choices=['short', 'medium', 'long'], 
                       help='Test only specific size corpus')
    parser.add_argument('--model', default='google/gemini-2.5-flash',
                       help='Model to use for translation')
    parser.add_argument('--corpus-dir', type=Path, 
                       default=Path(__file__).parent / 'perf-corpus',
                       help='Path to test corpus directory')
    parser.add_argument('--output-dir', type=Path,
                       default=Path(__file__).parent / 'perf-output',
                       help='Path to output directory')
    parser.add_argument('--csv', type=Path,
                       default=Path(__file__).parent / 'perf-results.csv',
                       help='Path to CSV results file')
    
    args = parser.parse_args()
    
    # Validate corpus exists
    if not args.corpus_dir.exists():
        print(f"Error: Corpus directory not found: {args.corpus_dir}")
        sys.exit(1)
    
    # Create output directory
    args.output_dir.mkdir(parents=True, exist_ok=True)
    
    # Run tests
    runner = PerfTestRunner(args.corpus_dir, args.output_dir, args.csv)
    
    try:
        runner.run_all_tests(size_filter=args.size, model=args.model)
    except KeyboardInterrupt:
        print("\n\n⚠️  Tests interrupted by user")
    finally:
        runner.print_summary()


if __name__ == '__main__':
    main()
