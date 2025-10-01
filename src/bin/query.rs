use anyhow::Result;
use clap::Parser;
use project_profile_finder::application::find_route;
use project_profile_finder::domain::{Profile, Query};
use project_profile_finder::infrastructure::load_data;
use std::io::{self, BufRead};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about = "Query preprocessed graph for routes")]
struct Args {
    #[arg(short, long)]
    input: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let data = load_data(&args.input)?;
    let stdin = io::stdin();
    let mut lines = stdin.lines();

    let q: usize = lines.next().unwrap()?.trim().parse()?;
    for _ in 0..q {
        let line = lines.next().unwrap()?;
        let parts: Vec<f64> = line.split_whitespace().map(|s| s.parse().unwrap()).collect();
        if parts.len() < 3 || (parts.len() - 3) % 2 != 0 {
            println!("Invalid query");
            continue;
        }
        let cx = parts[0];
        let cy = parts[1];
        let d = parts[2];
        let mut p_points = Vec::new();
        for i in (3..parts.len()).step_by(2) {
            p_points.push((parts[i], parts[i + 1]));
        }
        let p = Profile::new(p_points);
        let query = Query { c: (cx, cy), d, p };
        match find_route(&data, &query)? {
            Some(route) => {
                print!("{:.6} {:.6}", route.si, route.ti);
                for id in route.edge_ids {
                    print!(" {}", id);
                }
                println!();
            }
            None => println!("no feasible path within tolerance"),
        }
    }
    Ok(())
}