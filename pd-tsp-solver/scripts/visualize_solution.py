#!/usr/bin/env python3
"""Visualize a PD-TSP solution."""
import json
import sys
from pathlib import Path
import matplotlib.pyplot as plt
import matplotlib.patches as mpatches

def parse_tsp_file(filepath):
    """Parse TSP file and extract coordinates and demands."""
    with open(filepath, 'r') as f:
        lines = f.readlines()
    
    coords = {}
    demands = {}
    section = None
    
    for line in lines:
        line = line.strip()
        if not line or line == 'EOF':
            continue
        if 'NODE_COORD_SECTION' in line:
            section = 'coords'
            continue
        elif 'DEMAND_SECTION' in line:
            section = 'demands'
            continue
        elif 'DISPLAY_DATA_SECTION' in line:
            section = 'display'
            continue
        
        if section == 'coords':
            parts = line.split()
            if len(parts) >= 3:
                node_id = int(parts[0]) - 1  # 0-indexed
                x, y = float(parts[1]), float(parts[2])
                coords[node_id] = (x, y)
        elif section == 'demands':
            parts = line.split()
            if len(parts) >= 2:
                node_id = int(parts[0]) - 1  # 0-indexed
                demand = int(parts[1])
                demands[node_id] = demand
        elif section == 'display':
            parts = line.split()
            if len(parts) >= 3:
                node_id = int(parts[0]) - 1  # 0-indexed
                x, y = float(parts[1]), float(parts[2])
                coords[node_id] = (x, y)
    
    return coords, demands

def visualize_solution(instance_file, solution_file, output_file):
    """Generate visualization of the solution."""
    coords, demands = parse_tsp_file(instance_file)
    
    with open(solution_file, 'r') as f:
        solution = json.load(f)
    
    tour = solution['tour']
    
    # Calculate load profile if not provided
    if 'load_profile' in solution:
        load_profile = solution['load_profile']
    else:
        # Compute starting load
        n_nodes = max(demands.keys()) + 1
        depot_demand = demands.get(0, 0)
        return_depot_demand = demands.get(n_nodes - 1, 0) if n_nodes - 1 in demands else 0
        
        # Starting load calculation
        if depot_demand >= 0 and return_depot_demand < 0:
            starting_load = max(0, depot_demand + return_depot_demand)
        elif return_depot_demand >= 0 and depot_demand < 0:
            starting_load = max(0, return_depot_demand + depot_demand)
        else:
            starting_load = depot_demand + return_depot_demand
        
        load_profile = [starting_load]
        load = starting_load
        for node in tour[1:]:
            if node == 0:
                load = 0
            else:
                load += demands.get(node, 0)
            load_profile.append(load)
        load_profile.append(0)  # Return to depot

    
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 6))
    
    # Plot 1: Tour visualization
    # Draw edges
    for i in range(len(tour)):
        curr = tour[i]
        next_node = tour[(i + 1) % len(tour)] if i < len(tour) - 1 else 0
        x1, y1 = coords[curr]
        x2, y2 = coords[next_node]
        ax1.plot([x1, x2], [y1, y2], 'b-', alpha=0.5, linewidth=1)
    
    # Draw nodes
    for node_id, (x, y) in coords.items():
        if node_id == 0:
            # Depot
            ax1.plot(x, y, 'rs', markersize=12, label='Depot' if node_id == 0 else '')
        elif demands.get(node_id, 0) > 0:
            # Pickup node
            ax1.plot(x, y, 'g^', markersize=8, alpha=0.7)
        else:
            # Delivery node
            ax1.plot(x, y, 'bv', markersize=8, alpha=0.7)
    
    # Add legend
    depot_patch = mpatches.Patch(color='red', label='Depot')
    pickup_patch = mpatches.Patch(color='green', label='Pickup')
    delivery_patch = mpatches.Patch(color='blue', label='Delivery')
    ax1.legend(handles=[depot_patch, pickup_patch, delivery_patch])
    
    ax1.set_xlabel('X')
    ax1.set_ylabel('Y')
    ax1.set_title(f'Tour (Cost: {solution["cost"]:.2f})')
    ax1.grid(True, alpha=0.3)
    ax1.axis('equal')
    
    # Plot 2: Load profile
    ax2.plot(range(len(load_profile)), load_profile, 'b-o', linewidth=2, markersize=4)
    ax2.axhline(y=0, color='r', linestyle='--', alpha=0.5, label='Empty')
    ax2.fill_between(range(len(load_profile)), 0, load_profile, alpha=0.3)
    
    ax2.set_xlabel('Node Position in Tour')
    ax2.set_ylabel('Vehicle Load')
    ax2.set_title('Load Profile')
    ax2.grid(True, alpha=0.3)
    ax2.legend()
    
    # Add statistics
    max_load = max(load_profile) if load_profile else 0
    min_load = min(load_profile) if load_profile else 0
    
    stats_text = f"Algorithm: {solution['algorithm']}\n"
    stats_text += f"Cost: {solution['cost']:.2f}\n"
    stats_text += f"Profit: {solution['total_profit']}\n"
    stats_text += f"Objective: {solution['objective']:.2f}\n"
    stats_text += f"Time: {solution['computation_time']:.4f}s\n"
    stats_text += f"Max Load: {max_load}\n"
    stats_text += f"Min Load: {min_load}"
    
    fig.text(0.02, 0.98, stats_text, fontsize=9, verticalalignment='top',
             bbox=dict(boxstyle='round', facecolor='wheat', alpha=0.5))
    
    plt.tight_layout()
    plt.savefig(output_file, dpi=200, bbox_inches='tight')
    print(f'Visualization saved to {output_file}')

if __name__ == '__main__':
    if len(sys.argv) < 4:
        print('Usage: python visualize_solution.py <instance.tsp> <solution.json> <output.png>')
        sys.exit(1)
    
    visualize_solution(sys.argv[1], sys.argv[2], sys.argv[3])
