# Project Profile Finder

A Rust-based system for finding bicycle routes that match specific elevation profiles. Given a center point, search radius, and desired elevation profile, this tool finds roads that best match your climbing/descending preferences.

## Features

- **Profile Matching**: Uses area-under-curve comparison with optional vertical offset adjustment
- **Spatial Indexing**: R-tree for efficient spatial queries
- **Route Visualization**: Generate maps and elevation profile comparisons
- **Interactive Interface**: User-friendly command-line interaction
- **Beam Search**: Efficient route discovery algorithm

## Quick Start

### 1. Preprocess Road Data
```bash
cargo run --bin preprocess -- -i sample_roadgraph_100m.json -o sample.bin
```

### 2. Find Routes (Interactive Mode)
```bash
cargo run --bin interactive -- -i sample.bin
```

### 3. Find Routes (Batch Mode)
```bash
echo "1
80.0 80.0 50.0 0.0 0.0 160.0 5.0" | cargo run --bin query -- -i sample.bin
```

### 4. Create Visualizations
```bash
cargo run --bin visualize -- -i sample.bin \
  --cx 80.0 --cy 80.0 --distance 50.0 \
  --profile "0,0,160,5" \
  --map-output route_map.png \
  --profile-output elevation_profile.png
```

## Input Format

### Road Graph (JSONL)
Each line contains one JSON record:

- **Meta**: `{"type":"meta","crs":"EPSG:3857","units":"meters","max_segment_m":100}`
- **Node**: `{"type":"node","id":int,"x":float,"y":float,"elev":float}`
- **Edge**: `{"type":"edge","id":int,"u":node_id,"v":node_id,"length_m":float,"climb_m":float,"slope":float}`

### Query Format
```
<center_x> <center_y> <max_distance> <profile_points...>
```

Profile points are pairs of `(distance, elevation)`:
- Distance: cumulative meters from route start
- Elevation: relative meters from starting elevation

### Example Queries

**Flat 1km route:**
```
100.0 100.0 50.0 0.0 0.0 1000.0 0.0
```

**Steady 25m climb over 500m:**
```
100.0 100.0 75.0 0.0 0.0 500.0 25.0
```

**Hill profile (up then down):**
```
100.0 100.0 100.0 0.0 0.0 250.0 20.0 500.0 40.0 750.0 20.0 1000.0 0.0
```

## Binaries

### `preprocess`
Converts JSONL road data to optimized binary format.

```bash
cargo run --bin preprocess -- --input roads.jsonl --output roads.bin
```

### `query`
Batch mode route finder. Reads queries from stdin, outputs routes.

```bash
cargo run --bin query -- --input roads.bin
```

**Input format:**
```
<number_of_queries>
<query1>
<query2>
...
```

**Output format:**
```
<start_fraction> <end_fraction> <edge_id1> <edge_id2> ...
```

### `interactive`
User-friendly interactive interface with guidance and validation.

```bash
cargo run --bin interactive -- --input roads.bin
```

### `visualize`
Generate route maps and elevation profile comparisons.

```bash
cargo run --bin visualize -- --input roads.bin \
  --cx 100 --cy 100 --distance 50 \
  --profile "0,0,500,25,1000,0" \
  --map-output route.png \
  --profile-output elevation.png
```

## Algorithm Details

### Profile Matching
Uses **area-under-curve comparison** between target and actual elevation profiles:

1. **Base Score**: ∫|actual(s) - target(s)|ds over route length
2. **With Offset** (optional): Find optimal vertical offset z₀ to minimize ∫|actual(s) + z₀ - target(s)|ds
3. **Handles varying sampling**: Linear interpolation between profile points

### Route Search
**Beam search algorithm**:

1. **Start Selection**: Find edges within search radius using R-tree spatial index
2. **Path Expansion**: Extend promising partial routes, tracking cumulative area difference
3. **Beam Pruning**: Keep top K candidates based on estimated final score
4. **Termination**: Accept routes within length tolerance, select best profile match

### Complexity
- **Preprocessing**: O(M log M) for R-tree construction
- **Query**: O(B × D × V) where B=beam width, D=max route length/avg_edge, V=avg vertex degree

## Sample Data

The included `sample_roadgraph_100m.json` contains a small grid network:
- 9 nodes in a 3×3 grid (160m spacing)
- Various elevation values (12m to 26m)
- Bidirectional edges with length/climb/slope data

## Dependencies

- `petgraph`: Graph data structure
- `rstar`: R-tree spatial indexing
- `plotters`: Visualization
- `clap`: Command-line parsing
- `serde`: Data serialization

## Building

```bash
cargo build --release
```

## Testing

Test with the provided sample data:

```bash
# Preprocess sample data
cargo run --bin preprocess -- -i sample_roadgraph_100m.json -o sample.bin

# Try interactive mode
cargo run --bin interactive -- -i sample.bin

# Test a simple query
echo "1
80.0 80.0 100.0 0.0 0.0 160.0 10.0" | cargo run --bin query -- -i sample.bin
```

## Output Files

- **route_map.png**: Shows road network, search area, and found route
- **elevation_profile.png**: Compares target vs actual elevation profiles

## Limitations

- Assumes planar Euclidean geometry
- Linear elevation interpolation along edges
- Routes must be within length tolerance (default: max(5m, 5% of target))
- Search radius limits starting positions

## Future Improvements

- Hierarchical road network preprocessing
- Multi-objective optimization (distance + profile + other factors)
- Support for one-way restrictions and turn penalties
- Real-time visualization during search
- Export routes to GPX format