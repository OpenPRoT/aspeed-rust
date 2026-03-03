# Binary Size Analysis

This project includes automated binary size analysis to help maintain optimal resource usage for embedded targets.

## Quick Start

### Analyze current build
```bash
# Install cargo-bloat if not already installed
cargo install cargo-bloat

# Basic size analysis
cargo xtask bloat --release

# Generate detailed reports
cargo xtask bloat --release --report
```

### Reading the Reports

Reports are generated in `target/bloat-reports/`:

- `bloat_functions.txt` - Largest functions by size
- `bloat_crates.txt` - Size breakdown by crate
- `size_comparison.md` - Comparison with baselines

## Size Targets

### Current Constraints
- **Flash**: Target < 256KB for production builds
- **RAM**: Target < 64KB total usage  
- **Critical path**: Boot code should be < 32KB

### Optimization Guidelines

#### 🟢 Good practices:
- Use `#[inline(never)]` for large, rarely-called functions
- Prefer static dispatch over dynamic dispatch
- Use const generics instead of runtime configuration
- Enable LTO and optimize for size in release builds

#### 🔴 Watch out for:
- Large generic monomorphizations
- Unused features being compiled in
- Debug formatting code in release builds
- Accidental std library usage

## Integration with CI/CD

### Automated Analysis
- **Every PR**: Size comparison against main branch
- **Precommit**: Local size analysis in reports
- **Releases**: Historical size tracking

### Size Regression Detection
PRs that increase binary size by more than 10KB will be flagged for review.

## Manual Investigation

### Finding size regressions:
```bash
# Compare with specific commit
git checkout <baseline-commit>
cargo xtask bloat --release --output-dir target/baseline-reports

git checkout <current-branch>  
cargo xtask bloat --release --output-dir target/current-reports

# Manual comparison of the reports
```

### Investigating specific functions:
```bash
# Find largest functions
cargo bloat --release --crates

# Find compilation time hogs (often correlates with code size)
cargo bloat --release --time
```

## Size Budgets by Feature

| Component | Target Size | Current | Notes |
|-----------|-------------|---------|--------|
| Core HAL | < 32KB | TBD | Hardware abstraction |  
| Crypto (RSA) | < 64KB | TBD | RSA operations |
| Crypto (ECDSA) | < 32KB | TBD | ECDSA operations |
| Hash functions | < 16KB | TBD | SHA family |
| Main application | < 32KB | TBD | Application logic |
| **Total** | **< 176KB** | **TBD** | Leaves 80KB buffer |

## Historical Tracking

Size data is tracked in CI artifacts and can be used to generate long-term size trend reports.
