import pandas as pd
import plotly.graph_objects as go
import plotly.subplots as sp
import glob
import plotly.io as pio

def plot_metrics(input_file_prefix: str):
    """
    Reads Parquet files, calculates statistics, and generates interactive Plotly plots.

    Args:
        input_file_prefix: The prefix of the input files (e.g., "Call_7_Vehicle_3").
                           This function will find all files matching
                           "instrumentation/annealing_{prefix}_*.parquet".
    """

    all_files = glob.glob(f"instrumentation/annealing_{input_file_prefix}_*.parquet")
    if not all_files:
        print(f"No files found for prefix: {input_file_prefix}")
        return

    all_data = []
    for file in all_files:
        df = pd.read_parquet(file)
        all_data.append(df)

    # Concatenate all runs into a single DataFrame
    combined_df = pd.concat(all_data, keys=range(len(all_files)), names=['run'])
    combined_df = combined_df.reset_index(level=0)

    # Calculate statistics
    grouped = combined_df.groupby('iteration')
    stats = grouped.agg({
        'current_cost': ['min', 'max', 'mean'],
        'best_cost': ['min', 'max', 'mean'],
        'evaluations': ['min', 'max', 'mean'],
        'infeasible_count': ['min', 'max', 'mean'],
        'time': ['min', 'max', 'mean'],
        'temperature': ['mean']
    })

    # Flatten the multi-level column index
    stats.columns = ['_'.join(col).strip() for col in stats.columns.values]
    stats = stats.reset_index()

    fig = sp.make_subplots(rows=4, cols=1,
                           subplot_titles=(f'Current and Best cost',
                                           f'Evaluations and Infeasible counts',
                                           f'Time',
                                           f'Temperature'),
                           vertical_spacing=0.1)


    # Current Cost
    fig.add_trace(go.Scatter(x=stats['iteration'], y=stats['current_cost_mean'],
                             mode='lines', name='Current Cost',
                             line=dict(color='blue'), legendgroup='current_cost',
                             hovertemplate='%{y:,.0f}<extra></extra>'), row=1, col=1)
    fig.add_trace(go.Scatter(x=stats['iteration'], y=stats['current_cost_min'],
                             mode='lines', name='Current Cost (Min)',
                             line=dict(width=0), showlegend=False, legendgroup='current_cost',
                             hovertemplate='%{y:,.0f}<extra></extra>'), row=1, col=1)
    fig.add_trace(go.Scatter(x=stats['iteration'], y=stats['current_cost_max'],
                             mode='lines', name='Current Cost (Max)',
                             line=dict(width=0), fill='tonexty', legendgroup='current_cost',
                             fillcolor='rgba(0,0,255,0.2)', showlegend=False,
                             hovertemplate='%{y:,.0f}<extra></extra>'), row=1, col=1)

    # Best Cost
    fig.add_trace(go.Scatter(x=stats['iteration'], y=stats['best_cost_mean'],
                             mode='lines', name='Best Cost',
                             line=dict(color='green'), legendgroup='best_cost',
                             hovertemplate='%{y:,.0f}<extra></extra>'), row=1, col=1)
    fig.add_trace(go.Scatter(x=stats['iteration'], y=stats['best_cost_min'],
                             mode='lines', name='Best Cost (Min)',
                             line=dict(width=0), showlegend=False, legendgroup='best_cost',
                             hovertemplate='%{y:,.0f}<extra></extra>'), row=1, col=1)
    fig.add_trace(go.Scatter(x=stats['iteration'], y=stats['best_cost_max'],
                             mode='lines', name='Best Cost (Max)',
                             line=dict(width=0), fill='tonexty', legendgroup='best_cost',
                             fillcolor='rgba(0,255,0,0.2)', showlegend=False,
                             hovertemplate='%{y:,.0f}<extra></extra>'), row=1, col=1)

    fig.update_yaxes(title_text="Cost", row=1, col=1, tickformat=",")



    # Evaluations
    fig.add_trace(go.Scatter(x=stats['iteration'], y=stats['evaluations_mean'],
                             mode='lines', name='Evaluations',
                             line=dict(color='blue'), legendgroup='evaluations',
                             hovertemplate='Ev. avg.: %{y:,.0f}<extra></extra>'), row=2, col=1)
    fig.add_trace(go.Scatter(x=stats['iteration'], y=stats['evaluations_min'],
                             mode='lines', name='Evaluations (Min)',
                             line=dict(width=0), showlegend=False, legendgroup='evaluations',
                             hovertemplate='Ev. min.: %{y:,.0f}<extra></extra>'), row=2, col=1)
    fig.add_trace(go.Scatter(x=stats['iteration'], y=stats['evaluations_max'],
                             mode='lines', name='Evaluations (Max)',
                             line=dict(width=0), fill='tonexty', legendgroup='evaluations',
                             fillcolor='rgba(0,0,255,0.2)', showlegend=False,
                             hovertemplate='Ev. max.: %{y:,.0f}<extra></extra>'), row=2, col=1)

    # Infeasible Count
    fig.add_trace(go.Scatter(x=stats['iteration'], y=stats['infeasible_count_mean'],
                             mode='lines', name='Infeasible Count',
                             line=dict(color='red'), legendgroup='infeasible_count',
                             hovertemplate='In. avg.: %{y:,.0f}<extra></extra>'), row=2, col=1)
    fig.add_trace(go.Scatter(x=stats['iteration'], y=stats['infeasible_count_min'],
                             mode='lines', name='Infeasible Count (Min)',
                             line=dict(width=0), showlegend=False, legendgroup='infeasible_count',
                             hovertemplate='In. min.: %{y:,.0f}<extra></extra>'), row=2, col=1)
    fig.add_trace(go.Scatter(x=stats['iteration'], y=stats['infeasible_count_max'],
                             mode='lines', name='Infeasible Count (Max)',
                             line=dict(width=0), fill='tonexty', legendgroup='infeasible_count',
                             fillcolor='rgba(255,0,0,0.2)', showlegend=False,
                             hovertemplate='In. max.: %{y:,.0f}<extra></extra>'), row=2, col=1)

    fig.update_yaxes(title_text="Count", row=2, col=1, tickformat=",")

    # Time
    fig.add_trace(go.Scatter(x=stats['iteration'], y=stats['time_mean'] * 1_000_000,
                             mode='lines', name='Time', legendgroup='time',
                             line=dict(color='purple'),
                             hovertemplate='Mean: %{y:,.0f}<extra></extra>'), row=3, col=1)
    fig.add_trace(go.Scatter(x=stats['iteration'], y=stats['time_min'] * 1_000_000,
                             mode='lines', name='Time (Min)', legendgroup='time',
                             line=dict(width=0), showlegend=False,
                             hovertemplate='Min: %{y:,.0f}<extra></extra>'), row=3, col=1)
    fig.add_trace(go.Scatter(x=stats['iteration'], y=stats['time_max'] * 1_000_000,
                             mode='lines', name='Time (Max)', legendgroup='time',
                             line=dict(width=0), fill='tonexty',
                             fillcolor='rgba(128,0,128,0.2)', showlegend=False,
                             hovertemplate='Max: %{y:,.0f}<extra></extra>'), row=3, col=1)

    fig.update_yaxes(title_text="Time (μs)", row=3, col=1, tickformat=",")

    # Temperature
    fig.add_trace(go.Scatter(x=stats['iteration'], y=stats['temperature_mean'],
                             mode='lines', name='Temperature',
                             line=dict(color='orange')), row=4, col=1)
    fig.update_yaxes(title_text="Temperature", row=4, col=1, tickformat=",")
    fig.update_xaxes(title_text="Iteration", row=4, col=1)

    fig.update_layout(height=2800,
                      width=1400,
                      title_text=f"Metrics for {input_file_prefix}",
                      hovermode='x unified')

    pio.write_html(fig, file=f'instrumentation/metrics_{input_file_prefix}.html', auto_open=True)

if __name__ == '__main__':
    plot_metrics("Call_300_Vehicle_90")
    # data = [
    #     "Call_7_Vehicle_3",
    #     "Call_18_Vehicle_5",
    #     "Call_35_Vehicle_7",
    #     "Call_80_Vehicle_20",
    #     "Call_130_Vehicle_40",
    #     "Call_300_Vehicle_90",
    # ]
    #