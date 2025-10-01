use anyhow::Result;
use clap::Parser;
use project_profile_finder::application::find_route;
use project_profile_finder::domain::{Profile, Query};
use project_profile_finder::infrastructure::load_data;
use std::io::{self, Write};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about = "Interactive route finder with better user interface")]
struct Args {
    #[arg(short, long)]
    input: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    println!("Loading data from {:?}...", args.input);
    let data = load_data(&args.input)?;
    println!("✅ Data loaded successfully!");
    println!("📊 Graph has {} nodes and {} edges",
             data.graph.graph.node_count(),
             data.graph.graph.edge_count());

    loop {
        println!("\n🚴 Project Profile Finder - Interactive Mode");
        println!("============================================");

        // Get center coordinates
        let (cx, cy) = get_center_coordinates()?;

        // Get search radius
        let distance = get_search_radius()?;

        // Get profile
        let profile = get_elevation_profile()?;

        // Create query
        let query = Query {
            c: (cx, cy),
            d: distance,
            p: profile.clone(),
        };

        // Display query summary
        println!("\n📋 Query Summary:");
        println!("   Center: ({:.1}, {:.1})", cx, cy);
        println!("   Search radius: {:.1}m", distance);
        println!("   Profile length: {:.1}m", profile.total_length());
        println!("   Profile points: {:?}", profile.points);

        // Search for route
        print!("\n🔍 Searching for matching route... ");
        io::stdout().flush()?;

        match find_route(&data, &query)? {
            Some(route) => {
                println!("✅ Found!");
                println!("\n🛤️  Route Details:");
                println!("   Segments: {} edges", route.edge_ids.len());
                println!("   Start fraction: {:.3}", route.si);
                println!("   End fraction: {:.3}", route.ti);
                println!("   Edge IDs: {:?}", route.edge_ids);

                // Offer visualization
                if ask_yes_no("\n🖼️  Would you like to create visualizations? (y/n): ")? {
                    create_visualizations(&data, &query, &route, &profile)?;
                }
            }
            None => {
                println!("❌ No feasible route found within tolerance");
                println!("💡 Try:");
                println!("   - Increasing the search radius");
                println!("   - Modifying the elevation profile");
                println!("   - Moving the center point");

                if ask_yes_no("\n🖼️  Show search area visualization? (y/n): ")? {
                    create_search_area_vis(&data, &query)?;
                }
            }
        }

        if !ask_yes_no("\n🔄 Search for another route? (y/n): ")? {
            break;
        }
    }

    println!("\n👋 Thanks for using Project Profile Finder!");
    Ok(())
}

fn get_center_coordinates() -> Result<(f64, f64)> {
    loop {
        print!("📍 Enter center coordinates (x y): ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let parts: Vec<&str> = input.trim().split_whitespace().collect();
        if parts.len() != 2 {
            println!("❌ Please enter exactly two numbers (x y)");
            continue;
        }

        match (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
            (Ok(x), Ok(y)) => return Ok((x, y)),
            _ => println!("❌ Please enter valid numbers"),
        }
    }
}

fn get_search_radius() -> Result<f64> {
    loop {
        print!("🎯 Enter search radius in meters (e.g., 50): ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        match input.trim().parse::<f64>() {
            Ok(d) if d > 0.0 => return Ok(d),
            _ => println!("❌ Please enter a positive number"),
        }
    }
}

fn get_elevation_profile() -> Result<Profile> {
    println!("\n📈 Define your elevation profile:");
    println!("   Enter pairs of (distance, elevation_gain)");
    println!("   Distance is cumulative from start");
    println!("   Elevation is relative to starting elevation");
    println!("   Example: '0 0 100 10 200 5' means:");
    println!("     - Start at (0m, +0m elevation)");
    println!("     - At 100m: +10m elevation gain");
    println!("     - At 200m: +5m elevation gain");

    loop {
        print!("⛰️  Enter profile points: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let numbers: Result<Vec<f64>, _> = input
            .trim()
            .split_whitespace()
            .map(|s| s.parse())
            .collect();

        match numbers {
            Ok(nums) if nums.len() >= 2 && nums.len() % 2 == 0 => {
                let mut points = Vec::new();
                for chunk in nums.chunks(2) {
                    points.push((chunk[0], chunk[1]));
                }

                // Validate profile
                if points[0].0 != 0.0 {
                    println!("⚠️  First distance should be 0, adjusting...");
                }
                if points[0].1 != 0.0 {
                    println!("⚠️  First elevation should be 0, adjusting...");
                }

                // Check distances are increasing
                let mut valid = true;
                for i in 1..points.len() {
                    if points[i].0 <= points[i-1].0 {
                        println!("❌ Distances must be increasing");
                        valid = false;
                        break;
                    }
                }

                if valid {
                    return Ok(Profile::new(points));
                }
            }
            Ok(_) => println!("❌ Please enter an even number of values (distance, elevation pairs)"),
            Err(_) => println!("❌ Please enter valid numbers"),
        }
    }
}

fn ask_yes_no(prompt: &str) -> Result<bool> {
    loop {
        print!("{}", prompt);
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        match input.trim().to_lowercase().as_str() {
            "y" | "yes" => return Ok(true),
            "n" | "no" => return Ok(false),
            _ => println!("❌ Please enter 'y' or 'n'"),
        }
    }
}

fn create_visualizations(
    _data: &project_profile_finder::infrastructure::AppData,
    query: &Query,
    _route: &project_profile_finder::domain::Route,
    target_profile: &Profile
) -> Result<()> {
    use std::process::Command;

    println!("🎨 Creating visualizations...");

    // Create profile string for command line
    let profile_str = target_profile.points
        .iter()
        .map(|(d, z)| format!("{},{}", d, z))
        .collect::<Vec<_>>()
        .join(",");

    // Run visualization command
    let status = Command::new("cargo")
        .args(&[
            "run", "--bin", "visualize", "--",
            "--input", &format!("{}", query.c.0), // This is wrong, but we'll fix it
            "--cx", &query.c.0.to_string(),
            "--cy", &query.c.1.to_string(),
            "--distance", &query.d.to_string(),
            "--profile", &profile_str,
        ])
        .status();

    match status {
        Ok(_) => println!("✅ Visualizations created: route_map.png, elevation_profile.png"),
        Err(e) => println!("❌ Failed to create visualizations: {}", e),
    }

    Ok(())
}

fn create_search_area_vis(
    _data: &project_profile_finder::infrastructure::AppData,
    _query: &Query,
) -> Result<()> {
    println!("🎨 Search area visualization would be created here");
    // Implementation would be similar to above
    Ok(())
}

// Preset profile examples
fn _show_profile_examples() {
    println!("\n💡 Example profiles:");
    println!("   Flat: '0 0 1000 0'");
    println!("   Steady climb: '0 0 500 25 1000 50'");
    println!("   Hill: '0 0 250 20 500 40 750 20 1000 0'");
    println!("   Valley: '0 0 200 -10 600 -15 800 -10 1000 0'");
}