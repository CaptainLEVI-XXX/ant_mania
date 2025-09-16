use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use fastrand;

const MAX_MOVES: u32 = 10000;

/// Represents a colony ID (0-based index)
type ColonyId = usize;
type AntId = usize;

/// Main simulation state 
pub struct AntSimulation {
    /// Number of ants currently at each colony
    ant_count: Vec<u16>,  
    
    /// Is a colony destroyed
    destroyed: Vec<bool>,
    
    /// Colony names for final output (only used at start/end)
    colony_names: Vec<String>,
    
    /// Adjacency List (compressed)
    adjacency_list: Vec<ColonyId>,
    
    /// Starting index in adjacency_list for each colony's connections
    start_index: Vec<usize>,
    
    /// Number of connections for each colony
    connection_count: Vec<u8>,  // u8 since max is 4 connections
    
    /// Ant Tracking
    ant_position: Vec<ColonyId>,
    move_count: Vec<u32>,
    ant_alive: Vec<bool>,
    ants_at_colony: Vec<Vec<AntId>>,
    
    // Metadata
    total_colonies: usize,
    total_ants: usize,
    alive_ants: usize,
    active_ants_under_max_moves: usize, // counter to avoid O(n) scan
}

impl AntSimulation {
    /// Create a new simulation from a map file
    pub fn from_file(filename: &str, num_ants: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let file = File::open(filename)?;
        let reader = BufReader::new(file);
        
        // First pass: collect all colony names and build name->ID mapping
        let mut name_to_id: HashMap<String, ColonyId> = HashMap::new();
        let mut raw_connections: Vec<Vec<(String, ColonyId)>> = Vec::new();
        let mut colony_names: Vec<String> = Vec::new();
        
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }
            
            // First part is colony name
            let colony_name = parts[0].to_string();
            
            // Assign ID if new colony
            if !name_to_id.contains_key(&colony_name) {
                let id = colony_names.len();
                name_to_id.insert(colony_name.clone(), id);
                colony_names.push(colony_name.clone());
                raw_connections.push(Vec::new());
            }
            
            let colony_id = name_to_id[&colony_name];
            
            // Parse connections
            for i in 1..parts.len() {
                let connection_parts: Vec<&str> = parts[i].split('=').collect();
                if connection_parts.len() == 2 {
                    let target_name = connection_parts[1].to_string();
                    raw_connections[colony_id].push((target_name, colony_id));
                }
            }
        }
        
        let total_colonies = colony_names.len();
        
        // Build adjacency list
        let mut adjacency_list = Vec::new();
        let mut start_index = vec![0; total_colonies];
        let mut connection_count = vec![0u8; total_colonies];
        
        for (colony_id, connections) in raw_connections.iter().enumerate() {
            start_index[colony_id] = adjacency_list.len();
            
            for (target_name, _) in connections {
                if let Some(&target_id) = name_to_id.get(target_name) {
                    adjacency_list.push(target_id);
                    connection_count[colony_id] += 1;
                }
            }
        }
        
        // Initialize simulation state
        let mut sim = AntSimulation {
            ant_count: vec![0; total_colonies],
            destroyed: vec![false; total_colonies],
            colony_names,
            
            adjacency_list,
            start_index,
            connection_count,
            
            ant_position: vec![0; num_ants],
            move_count: vec![0; num_ants],
            ant_alive: vec![true; num_ants],
            ants_at_colony: vec![Vec::new(); total_colonies],
            
            total_colonies,
            total_ants: num_ants,
            alive_ants: num_ants,
            active_ants_under_max_moves: num_ants,
        };
        
        // Place ants at random colonies
        sim.initialize_ants();
        
        Ok(sim)
    }
    
    /// Place ants randomly across colonies
    fn initialize_ants(&mut self) {
        for ant_id in 0..self.total_ants {
            let mut colony_id;
            loop {
                colony_id = fastrand::usize(..self.total_colonies);
                if !self.destroyed[colony_id] {
                    break;
                }
            }
            
            self.ant_position[ant_id] = colony_id;
            self.ant_count[colony_id] += 1;
            self.ants_at_colony[colony_id].push(ant_id);
        }
    }
    
    /// Get valid moves from a colony
    #[inline]
    pub fn get_valid_moves(&self, colony_id: ColonyId, buffer: &mut Vec<ColonyId>) {
        buffer.clear();
        let start = self.start_index[colony_id];
        let count = self.connection_count[colony_id] as usize;
        
        for i in start..start + count {
            let neighbor = self.adjacency_list[i];
            if !self.destroyed[neighbor] {
                buffer.push(neighbor);
            }
        }
    }
    
    /// Remove ant from colony 
    #[inline]
    fn remove_ant_from_colony(&mut self, colony: ColonyId, ant: AntId) {
        if let Some(pos) = self.ants_at_colony[colony].iter().position(|&x| x == ant) {
            self.ants_at_colony[colony].swap_remove(pos);
        }
    }
    
    /// Move an ant once
    #[inline]
    pub fn move_ant(&mut self, ant_id: AntId, buffer: &mut Vec<ColonyId>) -> Option<(ColonyId, ColonyId)> {
        if !self.ant_alive[ant_id] {
            return None;
        }
        
        let current_colony = self.ant_position[ant_id];
        self.get_valid_moves(current_colony, buffer);
        
        if buffer.is_empty() {
            return None;
        }
        
        let next_colony = buffer[fastrand::usize(..buffer.len())];
        
        self.ant_position[ant_id] = next_colony;
        self.move_count[ant_id] += 1;
        
        if self.move_count[ant_id] == MAX_MOVES {
            self.active_ants_under_max_moves -= 1; // stop scanning in should_continue
        }
        
        self.ant_count[current_colony] -= 1;
        self.ant_count[next_colony] += 1;
        
        self.remove_ant_from_colony(current_colony, ant_id);
        self.ants_at_colony[next_colony].push(ant_id);
        
        Some((current_colony, next_colony))
    }
    
    #[inline]
    pub fn check_collision(&mut self, colony_id: ColonyId) -> Option<(AntId, AntId)> {
        if self.ant_count[colony_id] == 2 {
            let ant1 = self.ants_at_colony[colony_id][0];
            let ant2 = self.ants_at_colony[colony_id][1];
            
            self.destroy_colony(colony_id);
            self.kill_ant(ant1);
            self.kill_ant(ant2);
            
            return Some((ant1, ant2));
        }
        None
    }
    
    #[inline]
    fn destroy_colony(&mut self, colony_id: ColonyId) {
        self.destroyed[colony_id] = true;
        self.ant_count[colony_id] = 0;
        self.ants_at_colony[colony_id].clear();
    }
    
    #[inline]
    fn kill_ant(&mut self, ant_id: AntId) {
        if self.ant_alive[ant_id] {
            self.ant_alive[ant_id] = false;
            self.alive_ants -= 1;
            if self.move_count[ant_id] < MAX_MOVES {
                self.active_ants_under_max_moves -= 1;
            }
        }
    }
    
    /// check if simulation should continue
    #[inline]
    pub fn should_continue(&self) -> bool {
        self.alive_ants > 0 && self.active_ants_under_max_moves > 0
    }
    
    /// Run one iteration of the simulation
    pub fn run_iteration(&mut self) {
        let mut buffer = Vec::with_capacity(4);
        let mut colonies_to_check = Vec::new();
        
        for ant_id in 0..self.total_ants {
            if let Some((_, next_colony)) = self.move_ant(ant_id, &mut buffer) {
                if self.ant_count[next_colony] == 2 {
                    // avoid pushing duplicates
                    if colonies_to_check.last() != Some(&next_colony) {
                        colonies_to_check.push(next_colony);
                    }
                }
            }
        }
        
        for colony_id in colonies_to_check {
            self.check_collision(colony_id);
        }
    }
    
    /// Print the remaining map
    pub fn print_remaining_world(&self) {
        println!("\n=== Remaining World ===");
        
        for colony_id in 0..self.total_colonies {
            if self.destroyed[colony_id] {
                continue;
            }
            
            print!("{}", self.colony_names[colony_id]);
            
            let start = self.start_index[colony_id];
            let count = self.connection_count[colony_id] as usize;
            
            for i in start..start + count {
                let neighbor_id = self.adjacency_list[i];
                if !self.destroyed[neighbor_id] {
                    print!(" north={}", self.colony_names[neighbor_id]);
                }
            }
            
            println!();
        }
        
        println!("\nAlive ants: {}/{}", self.alive_ants, self.total_ants);
    }
    
    /// Get statistics
    pub fn stats(&self) -> (usize, usize, usize) {
        let active_colonies = self.destroyed.iter().filter(|&&d| !d).count();
        (self.alive_ants, active_colonies, self.total_colonies)
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <map_file> <num_ants>", args[0]);
        std::process::exit(1);
    }
    
    let filename = &args[1];
    let num_ants: usize = args[2].parse().expect("Number of ants must be a valid number");
    
    let mut sim = AntSimulation::from_file(filename, num_ants)
        .expect("Failed to load map file");
    
    let (ants, colonies, total) = sim.stats();
    println!("Starting simulation: {} ants, {}/{} active colonies", ants, colonies, total);
    
    let mut iterations = 0;
    let start = std::time::Instant::now();
    
    while sim.should_continue() && iterations < MAX_MOVES {
        sim.run_iteration();
        iterations += 1;
    }
    
    println!("\nSimulation ended after {} iterations", iterations);
    let duration = start.elapsed();
    println!("\nSimulation completed in {:?}", duration);
    sim.print_remaining_world();
}
