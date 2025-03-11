import io

import pandas as pd
import plotly.graph_objects as go
import plotly.io as pio
import dash
from dash import dcc, html, Input, Output, State
import dash_bootstrap_components as dbc
import glob
import os

pio.templates.default = "plotly_white"

palette = [
    "rgba(242, 128, 137, 1)",
    "rgba(101, 182, 191, 1)",
    "rgba(21, 63, 43, 1)",
    "rgba(155, 191, 101, 1)",
    "rgba(242, 173, 113, 1)"
]

def load_data(input_file_prefix: str):
    """Loads and combines data from all runs for a given prefix."""
    all_files = glob.glob(f"instrumentation/annealing_{input_file_prefix}_*.parquet")
    if not all_files:
        print(f"No files found for prefix: {input_file_prefix}")
        return None

    all_data = []
    for file in all_files:
        df = pd.read_parquet(file)
        # Extract run number from filename
        run_number = int(os.path.splitext(os.path.basename(file))[0].split("_")[-1])
        df['run'] = run_number  # Add run number as a column
        all_data.append(df)

    combined_df = pd.concat(all_data, ignore_index=True)
    return combined_df

app = dash.Dash(__name__, external_stylesheets=[dbc.themes.LUX])

file_prefixes = [
    "Call_7_Vehicle_3",
    "Call_18_Vehicle_5",
    "Call_35_Vehicle_7",
    "Call_80_Vehicle_20",
    "Call_130_Vehicle_40",
    "Call_300_Vehicle_90"
]

app.layout = dbc.Container([
    dbc.NavbarSimple(
        brand="SA metrics",
        brand_href="#",
        color="primary",
        dark=True
    ),

    dbc.Card([
        dbc.CardBody([
            dbc.Row([
                dbc.Col([
                    html.Label("Instance:"),
                    dcc.Dropdown(
                        id='file-prefix-dropdown',
                        options=[{'label': prefix, 'value': prefix} for prefix in file_prefixes],
                        value=file_prefixes[0]
                    ),
                ], width=6),
                dbc.Col([
                    html.Label("Run:"),
                    dcc.Slider(
                        id='run-slider',
                        min=0,
                        max=9,
                        step=1,
                        value=0,
                        marks={i: str(i) for i in range(10)},
                        tooltip={"placement": "bottom", "always_visible": True}
                    ),
                ], width=6),
            ]),
            dbc.Row([
                dbc.Col([
                    dcc.Checklist(
                        id='show-bands-checklist',
                        options=[
                            {'label': 'Show Min/Max Bands', 'value': 'show'}
                        ],
                        value=['show'],
                        inline=True
                    )
                ], width=12)
            ]),
        ])
    ], className="mb-3"),

    dbc.Row([
        dbc.Col([
            dcc.Graph(id='metric-plot')
        ], width=12)
    ]),

    # Hidden div to store pre-calculated min/max data
    html.Div(id='min-max-data', style={'display': 'none'})
], fluid=True, className="p-0")

@app.callback(
    Output('run-slider', 'min'),
    Output('run-slider', 'max'),
    Output('run-slider', 'marks'),
    Input('file-prefix-dropdown', 'value')
)
def update_slider(selected_prefix):
    df = load_data(selected_prefix)
    if df is not None:
        runs = sorted(df['run'].unique())
        min_run = min(runs)
        max_run = max(runs)
        marks = {int(run): str(run) for run in runs}  # Ensure marks are integers
        return min_run, max_run, marks
    return 0, 9, {i: str(i) for i in range(10)}  # Default

# Callback to pre-calculate min/max data
@app.callback(
    Output('min-max-data', 'children'),
    Input('file-prefix-dropdown', 'value')
)
def calculate_min_max(selected_prefix):
    df = load_data(selected_prefix)
    if df is None:
        return None

    # Calculate min/max across ALL runs
    min_max_stats = df.groupby('iteration').agg({
        'incumbent_cost': ['min', 'max'],
        'candidate_cost': ['min', 'max'],
        'best_cost': ['min', 'max']
    })
    min_max_stats.columns = ['_'.join(col).strip() for col in min_max_stats.columns.values]
    min_max_stats = min_max_stats.reset_index()
    return min_max_stats.to_json(date_format='iso', orient='split')

# Callback to update the plot
@app.callback(
    Output('metric-plot', 'figure'),
    Input('file-prefix-dropdown', 'value'),
    Input('run-slider', 'value'),
    Input('show-bands-checklist', 'value'),
    Input('metric-plot', 'relayoutData'),
    State('min-max-data', 'children')
)
def update_plot(selected_prefix, selected_run, show_bands, relayout_data, min_max_data_json):
    df = load_data(selected_prefix)
    if df is None or min_max_data_json is None:
        return go.Figure()

    # Filter data for the selected run
    df_run = df[df['run'] == selected_run]
    fig = go.Figure()

    # Left y-axis
    fig.add_trace(go.Scatter(
        x=df_run['iteration'],
        y=df_run['candidate_cost'],
        mode='lines',
        name='Candidate Cost',
        legendgroup='candidate_cost',
        line=dict(color=palette[3]),
        opacity=0.2,
        hovertemplate='%{y:,.0f}<extra></extra>'
    ))

    if 'show' in show_bands:
        min_max_stats = pd.read_json(io.StringIO(min_max_data_json), orient='split')
        # Incumbent cost min/max bands
        fig.add_trace(go.Scatter(
            x=min_max_stats['iteration'],
            y=min_max_stats['incumbent_cost_min'],
            mode='lines',
            name='Incumbent Cost (Min)',
            line=dict(width=0),
            showlegend=False,
            legendgroup='incumbent_cost',
            hovertemplate='%{y:,.0f}<extra></extra>'
        ))
        fig.add_trace(go.Scatter(
            x=min_max_stats['iteration'],
            y=min_max_stats['incumbent_cost_max'],
            mode='lines',
            name='Incumbent Cost (Max)',
            line=dict(width=0),
            fill='tonexty',
            legendgroup='incumbent_cost',
            fillcolor='rgba(242, 128, 137, 0.5)',
            showlegend=False,
            hovertemplate='%{y:,.0f}<extra></extra>'
        ))

    fig.add_trace(go.Scatter(
        x=df_run['iteration'],
        y=df_run['incumbent_cost'],
        mode='lines',
        name='Incumbent Cost',
        legendgroup='incumbent_cost',
        line=dict(color=palette[0]),
        hovertemplate='%{y:,.0f}<extra></extra>'
    ))
    fig.add_trace(go.Scatter(
        x=df_run['iteration'],
        y=df_run['best_cost'],
        mode='lines',
        name='Best Cost',
        legendgroup='best_cost',
        line=dict(color=palette[2]),
        hovertemplate='%{y:,.0f}<extra></extra>'
    ))

    # Right y-axis
    if 'candidate_seen' in df_run.columns:
        fig.add_trace(go.Scatter(
            x=df_run['iteration'],
            y=df_run['candidate_seen'],
            mode='lines',
            name='Seen',
            yaxis='y2',
            line=dict(color=palette[1]),
            opacity=0.3,
            hovertemplate='%{y:,.0f}<extra></extra>'
        ))

    if 'evaluations' in df_run.columns:
        fig.add_trace(go.Scatter(
            x=df_run['iteration'],
            y=df_run['evaluations'],
            mode='lines',
            name='Evaluations',
            yaxis='y2',  # Secondary y-axis
            line=dict(color=palette[2]),
            opacity=0.3,
            hovertemplate='%{y:,.0f}<extra></extra>'
        ))

    # Define both primary and secondary y-axes
    yaxis2_config = dict(
        overlaying='y',  # overlay on the primary y-axis
        side='right',
        showgrid=False,
    )

    yaxis2_config['range'] = [0, max(300, max(df_run['evaluations']), max(df_run['candidate_seen']))]

    fig.update_layout(
        title_text=f"Metrics for {selected_prefix}, run {selected_run}",
        xaxis_title="Iteration",
        yaxis=dict(
            title="Cost"
        ),
        yaxis2=yaxis2_config,
        hovermode='x unified',
        legend=dict(
            orientation="h",
            yanchor="bottom",
            y=-0.3,
            xanchor="center",
            x=0.5
        )
    )

    # Apply any relayout settings if provided
    if relayout_data:
        if 'xaxis.range[0]' in relayout_data and 'xaxis.range[1]' in relayout_data:
            fig.update_layout(
                xaxis_range=[relayout_data['xaxis.range[0]'],
                             relayout_data['xaxis.range[1]']]
            )
        if 'yaxis.range[0]' in relayout_data and 'yaxis.range[1]' in relayout_data:
            fig.update_layout(
                yaxis_range=[relayout_data['yaxis.range[0]'],
                             relayout_data['yaxis.range[1]']]
            )

    return fig


if __name__ == '__main__':
    app.run(debug=True)
