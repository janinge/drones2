use arrow::array::{BooleanArray, Float64Array, Int64Array};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use parquet::arrow::arrow_writer::ArrowWriter;
use std::fs::File;
use std::sync::Arc;
use crate::types::Cost;

#[derive(Debug)]
pub struct IterationRecord {
    pub iteration: usize,
    pub candidate_cost: Cost,
    pub candidate_seen: usize,
    pub incumbent_cost: Cost,
    pub best_cost: Cost,
    pub evaluations: usize,
    pub infeasible: usize,
    pub time: f64,
    pub temperature: Option<f32>,
}

pub fn serialize_to_parquet(
    iteration_data: &[IterationRecord],
    filename: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let iterations: Int64Array = iteration_data.iter().map(|d| d.iteration as i64).collect();
    let candidate_costs: Int64Array = iteration_data.iter().map(|d| d.candidate_cost as i64).collect();
    let candidate_observations: Int64Array = iteration_data.iter().map(|d| d.candidate_seen as i64).collect();
    let incumbent_costs: Int64Array = iteration_data.iter().map(|d| d.incumbent_cost as i64).collect();
    let best_costs: Int64Array = iteration_data.iter().map(|d| d.best_cost as i64).collect();
    let evaluations: Int64Array = iteration_data.iter().map(|d| d.evaluations as i64).collect();
    let infeasible_counts: Int64Array = iteration_data
        .iter()
        .map(|d| d.infeasible as i64)
        .collect();
    let times: Float64Array = iteration_data.iter().map(|d| d.time).collect();
    let temperatures: Float64Array = iteration_data
        .iter()
        .map(|d| d.temperature.unwrap_or(f32::NAN) as f64)
        .collect();

    // Arrow schema
    let schema = Schema::new(vec![
        Field::new("iteration", DataType::Int64, false),
        Field::new("candidate_cost", DataType::Int64, false),
        Field::new("candidate_seen", DataType::Int64, false),
        Field::new("incumbent_cost", DataType::Int64, false),
        Field::new("best_cost", DataType::Int64, false),
        Field::new("evaluations", DataType::Int64, false),
        Field::new("infeasible_count", DataType::Int64, false),
        Field::new("time", DataType::Float64, false),
        Field::new("temperature", DataType::Float64, false),
    ]);
    
    let batch = RecordBatch::try_new(
        Arc::new(schema),
        vec![
            Arc::new(iterations),
            Arc::new(candidate_costs),
            Arc::new(candidate_observations),
            Arc::new(incumbent_costs),
            Arc::new(best_costs),
            Arc::new(evaluations),
            Arc::new(infeasible_counts),
            Arc::new(times),
            Arc::new(temperatures),
        ],
    )?;
    
    let file = File::create(filename)?;
    let mut writer = ArrowWriter::try_new(file, batch.schema(), None)?;
    writer.write(&batch)?;
    writer.close()?;

    Ok(())
}

