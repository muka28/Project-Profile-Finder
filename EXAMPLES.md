# Usage Examples

## Basic Workflow

### 1. Prepare Data
```bash
# Convert sample data to binary format
cargo run --bin preprocess -- -i sample_roadgraph_100m.json -o sample.bin
```

### 2. Interactive Mode (Recommended for beginners)
```bash
cargo run --bin interactive -- -i sample.bin
```

Follow the prompts:
- Enter center coordinates (e.g., `80 80`)
- Enter search radius (e.g., `50`)
- Enter elevation profile (e.g., `0 0 160 5`)

## Batch Queries

### Simple Queries
```bash
# Find flat 1km route near (100, 100)
echo "1
100.0 100.0 50.0 0.0 0.0 1000.0 0.0" | cargo run --bin query -- -i sample.bin

# Find 500m route with 25m climb
echo "1
80.0 80.0 75.0 0.0 0.0 500.0 25.0" | cargo run --bin query -- -i sample.bin
```

### Multiple Queries
```bash
# Test several different profiles
cat << EOF | cargo run --bin query -- -i sample.bin
3
80.0 80.0 50.0 0.0 0.0 160.0 5.0
80.0 80.0 100.0 0.0 0.0 240.0 10.0
100.0 100.0 75.0 0.0 0.0 320.0 0.0
EOF
```

## Profile Examples

### Flat Routes
```bash
# Flat 500m
echo "1
80.0 80.0 50.0 0.0 0.0 500.0 0.0" | cargo run --bin query -- -i sample.bin
```

### Steady Climbs
```bash
# 200m route with 15m steady climb
echo "1
80.0 80.0 75.0 0.0 0.0 200.0 15.0" | cargo run --bin query -- -i sample.bin

# Longer climb: 800m with 40m gain
echo "1
80.0 80.0 100.0 0.0 0.0 800.0 40.0" | cargo run --bin query -- -i sample.bin
```

### Hills (Up then Down)
```bash
# Classic hill: up to 250m (+20m), peak at 500m (+30m), down by 1000m (+10m)
echo "1
80.0 80.0 100.0 0.0 0.0 250.0 20.0 500.0 30.0 750.0 20.0 1000.0 10.0" | cargo run --bin query -- -i sample.bin

# Sharp hill: quick up and down
echo "1
80.0 80.0 75.0 0.0 0.0 100.0 15.0 200.0 15.0 300.0 0.0" | cargo run --bin query -- -i sample.bin
```

### Valleys (Down then Up)
```bash
# Valley: down 10m by 200m, bottom at 600m (-15m), back up by 1000m (-5m)
echo "1
80.0 80.0 100.0 0.0 0.0 200.0 -10.0 600.0 -15.0 800.0 -10.0 1000.0 -5.0" | cargo run --bin query -- -i sample.bin
```

### Complex Profiles
```bash
# Rolling hills: multiple ups and downs
echo "1
80.0 80.0 150.0 0.0 0.0 200.0 10.0 400.0 5.0 600.0 15.0 800.0 8.0 1000.0 12.0" | cargo run --bin query -- -i sample.bin

# Plateau: climb, flat, descend
echo "1
80.0 80.0 100.0 0.0 0.0 300.0 20.0 700.0 20.0 1000.0 5.0" | cargo run --bin query -- -i sample.bin
```

## Visualization Examples

### Basic Visualization
```bash
# Create map and elevation profile for a hill route
cargo run --bin visualize -- -i sample.bin \
  --cx 80.0 --cy 80.0 --distance 50.0 \
  --profile "0,0,200,15,400,20,600,10,800,0" \
  --map-output hill_route.png \
  --profile-output hill_elevation.png
```

### Custom Output Files
```bash
# Different output filenames
cargo run --bin visualize -- -i sample.bin \
  --cx 100.0 --cy 100.0 --distance 75.0 \
  --profile "0,0,500,25" \
  --map-output "climb_map_$(date +%Y%m%d).png" \
  --profile-output "climb_profile_$(date +%Y%m%d).png"
```

### Search Area Visualization (No Route Found)
```bash
# Impossible route (too steep/long for small network)
cargo run --bin visualize -- -i sample.bin \
  --cx 80.0 --cy 80.0 --distance 30.0 \
  --profile "0,0,2000,100" \
  --map-output impossible_search.png \
  --profile-output impossible_profile.png
```

## Troubleshooting Examples

### No Route Found - Solutions

**Problem: "no feasible path within tolerance"**

```bash
# 1. Increase search radius
echo "1
80.0 80.0 150.0 0.0 0.0 300.0 10.0" | cargo run --bin query -- -i sample.bin

# 2. Shorten target distance
echo "1
80.0 80.0 50.0 0.0 0.0 150.0 5.0" | cargo run --bin query -- -i sample.bin

# 3. Reduce elevation requirements
echo "1
80.0 80.0 50.0 0.0 0.0 200.0 3.0" | cargo run --bin query -- -i sample.bin

# 4. Try different center point
echo "1
120.0 120.0 75.0 0.0 0.0 200.0 8.0" | cargo run --bin query -- -i sample.bin
```

### Understanding the Sample Network

The `sample_roadgraph_100m.json` contains:
- **Grid**: 3×3 nodes spaced 160m apart
- **Coordinates**: (0,0) to (160,160)
- **Elevations**: 12m to 26m range
- **Good centers**: (80,80) is central, (40,40) or (120,120) for corners
- **Reasonable radius**: 50-100m works well
- **Route lengths**: 80-320m are achievable

### Test Different Strategies
```bash
# Conservative (more likely to find routes)
echo "1
80.0 80.0 100.0 0.0 0.0 160.0 3.0" | cargo run --bin query -- -i sample.bin

# Aggressive (might fail, tests tolerance)
echo "1
80.0 80.0 40.0 0.0 0.0 400.0 25.0" | cargo run --bin query -- -i sample.bin
```

## Building Custom Datasets

For testing with your own data, ensure JSONL format:

```bash
# Example node
{"type":"node","id":1,"x":0.0,"y":0.0,"elev":100.0}

# Example edge (directed)
{"type":"edge","id":1,"u":1,"v":2,"length_m":150.0,"climb_m":5.0,"slope":0.033}
```

Then preprocess and query as usual:
```bash
cargo run --bin preprocess -- -i your_data.json -o your_data.bin
cargo run --bin interactive -- -i your_data.bin
```