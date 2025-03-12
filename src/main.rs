use drones2::problem::Problem;
use drones2::solution::Solution;
use drones2::search::local::local_search;
use drones2::search::annealing::simulated_annealing;

use std::path::Path;
use std::time::Instant;
use drones2::metrics;

fn main() -> std::io::Result<()> {
    let data = [
        "Call_7_Vehicle_3.txt",
        "Call_18_Vehicle_5.txt",
        "Call_35_Vehicle_7.txt",
        "Call_80_Vehicle_20.txt",
        "Call_130_Vehicle_40.txt",
        "Call_300_Vehicle_90.txt",
    ];
    
    const MAX_ITERATIONS: usize = 1_000;
    const RUNS: usize = 10;

    for file in data {
        let path = Path::new("data").join(file);

        let setup_time = Instant::now();
        
        let problem = Problem::load(path.to_str().unwrap()).unwrap();

        let mut initial = Solution::new(&problem);
        let initial_cost = initial.cost(&problem);

        println!("------");

        println!("Instance: {:?}", path.to_str().unwrap());
        println!("Initial: {:?}", initial.feasible(&problem));
        println!("Cost: {:?}", initial_cost);

        let mut results = Vec::with_capacity(RUNS);
        let mut global_metrics = Vec::with_capacity(RUNS);
        
        let start_time = Instant::now();

        for _ in 0..RUNS {
            let mut metrics = Vec::with_capacity(MAX_ITERATIONS);
            
            let (best_cost, solution) = simulated_annealing(&problem, initial.clone(), MAX_ITERATIONS, 100, 0.1, Some(&mut metrics));
            //let (best_cost, solution) = local_search(&problem, initial.clone(), MAX_ITERATIONS);

            results.push((best_cost, solution.to_pylist(true)));
            global_metrics.push(metrics);
        }

        let duration = start_time.elapsed();

        results.sort_by_key(|(cost, _)| *cost);

        println!("Time computing: {:?} ({:?} setup)",
                 (duration / RUNS as u32) + (start_time - setup_time),
                 start_time - setup_time);

        if !results.is_empty() {
            println!(
                "Average cost: {:?}",
                results.iter().map(|(cost, _)| cost).sum::<i32>() / results.len() as i32
            );
        }

        println!("Best cost: {:?}", results.first().unwrap().0);
        println!("Best solution: {:?}", results.first().unwrap().1);

        println!(
            "Improvement over initial: {:?}",
            (initial_cost - results.first().unwrap().0) as f64 / initial_cost as f64 * 100.0
        );

        global_metrics
            .iter()
            .enumerate()
            .for_each(|(i, metric)| {
                let base_name = if let Some(dot_index) = file.rfind('.') {
                    &file[..dot_index]
                } else {
                    file
                };
                
                metrics::serialize_to_parquet(
                    metric, 
                    format!("instrumentation/annealing_{}_{:03}.parquet", base_name, i).as_str()
                ).unwrap();
            });
    }

    Ok(())
}
