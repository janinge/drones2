use std::io::{self, Write};
use std::time::Instant;
use drones2::operators::{INSERTION_OPERATORS, REMOVAL_OPERATORS};
use drones2::operators::params::RemovalParams; // Import RemovalParams
use drones2::problem::Problem;
use drones2::search::pooled::Pooled;
use drones2::search::warmup::Warmup;
use drones2::solution::Solution;
use drones2::types::Cost;
use drones2::utils::{Args, Parser, enumerate_input_files};

fn main() -> io::Result<()> {
    let args = Args::parse();

    let instance_files = enumerate_input_files(&args)?;

    let runs = args.runs as usize;

    let operator_combinations: Vec<_> = REMOVAL_OPERATORS
        .iter()
        .flat_map(|&removal| {
            INSERTION_OPERATORS.iter().map(move |&insertion| (removal, insertion))
        })
        .collect();

    let removal_params = RemovalParams {
        selection_ratio: args.removal_selection_ratio,
        randomness: 0.0,
        cost_bias: 0.0,
        assignment_bias: args.removal_assignment_bias,
        min_removals: args.removal_min_removals,
        max_removals: args.removal_max_removals,
    };


    for path in instance_files {
        let instance_path = match path.to_str() {
            Some(p) => p,
            None => {
                eprintln!("Invalid input path: {:?}", path);
                continue;
            }
        };

        let setup_time = Instant::now();

        let problem = match Problem::load(instance_path) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Failed to load problem '{}': {}", instance_path, e);
                continue;
            }
        };

        let mut initial = Solution::new(&problem);
        let initial_cost = initial.cost(&problem);

        println!("------");

        println!("Instance: {:?}", instance_path);
        println!("Initial: {:?}", initial.feasible(&problem));
        println!("Initial cost: {:?}", initial_cost);

        let mut results = Vec::with_capacity(runs);

        let runs_start_time = Instant::now();

        for run in 1..=runs {
            let mut current_best_sol = initial.clone();
            let mut current_best_cost = initial_cost;

            let mut printed_cost: Option<(Cost, Instant)> = None;

            let t0 = args.t0.unwrap_or_else(|| {
                let warmup = Warmup::new(&operator_combinations);
                warmup.run(&problem, current_best_sol.clone(), 100, 0.8)
            });
            
            let mut temp = t0;

            let time_limit = args.time_limit.unwrap_or(1_800);

            let mut iterations_per_sec = 1_000;

            let mut search = Pooled::new(&operator_combinations, removal_params);

            let start_time = Instant::now();
            let mut iteration_end_time = start_time;

            loop {
                let alpha_per_sec = (args.t_final / t0).powf(1.0 / time_limit as f32);
                let alpha_per_iter = alpha_per_sec.powf(1.0 / iterations_per_sec as f32);

                let (best_cost, solution) = search.run(&problem, current_best_sol.clone(), iterations_per_sec, temp, alpha_per_iter);
                
                if best_cost < current_best_cost {
                    current_best_sol = solution.clone();
                    current_best_cost = best_cost;
                }
                
                let clock = Instant::now();
                
                let elapsed_time = clock.duration_since(start_time).as_secs();
                
                if elapsed_time > time_limit as u64 {
                    break;
                }
                
                let iteration_duration = clock.duration_since(iteration_end_time).as_secs_f64();
                if iteration_duration > 0.0 {
                    iterations_per_sec = (iterations_per_sec as f64 * 0.5 + (iterations_per_sec as f64 * 1.0 / iteration_duration) * 0.5) as usize;
                }

                if let Some(delay) = args.print_best_delay {
                    if let Some((last_cost, last_instant)) = printed_cost {
                        if current_best_cost < last_cost && clock.duration_since(last_instant).as_secs() > delay as u64 {
                            println!("\rBest after {:.2} seconds ({:?}): {:?}                                ",
                                     clock.duration_since(last_instant).as_secs_f64() - delay as f64, current_best_cost, current_best_sol.to_pylist(true));

                            printed_cost = Some((current_best_cost, clock))
                        }
                    } else {
                        printed_cost = Some((i32::MAX, clock))
                    }
                }

                print!("\rRun: {}/{}. Elapsed time: {}/{} seconds. Speed: {}/{:.4} iter/sec. Temperature: {:.4}. Best cost: {:?}.                                ",
                         run, runs, elapsed_time, time_limit, iterations_per_sec, iteration_duration, temp, best_cost);
                io::stdout().flush()?;

                iteration_end_time = clock;
                temp *= alpha_per_sec;
            }

            results.push((current_best_cost, current_best_sol.to_pylist(true)));
        }

        let duration = runs_start_time.elapsed();

        results.sort_by_key(|(cost, _)| *cost);
        
        println!();

        println!("Time computing: {:?} ({:?} setup)",
                 (duration / runs as u32) + (runs_start_time - setup_time),
                 runs_start_time - setup_time);

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
    }

    Ok(())
}
