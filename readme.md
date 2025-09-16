### Preprocessing the Map Efficiently

- Parsed the input once and built a compressed adjacency list (adjacency_list + start_index + connection_count) for all colonies.

- Ant movement is always O(1) by directly indexing into neighbors and picking a random valid move.

- Avoids repeated string parsing or dynamic traversals during the simulation.

### Flat Data Structures for Cache Locality

- Stored colonies (destroyed, ant_count) and ants (position, alive, move_count) in contiguous vectors instead of hash maps.

- This improves memory locality and reduces pointer chasing, keeping per-tick updates very fast.

### Collision Detection

- Each colony tracks how many ants are present (ant_count) and a list of the specific ants (ants_at_colony).

- A colony is destroyed immediately once exactly two ants meet, so in practice no colony ever grows beyond two ants.

- This makes collision checks effectively O(1) per colony, though a short scan of the ant list is still performed.

### No Dynamic Allocations in the Main Loop

- All simulation vectors (ant_positions, ant_alive, ant_count, etc.) are pre-allocated once at startup.

- The per-tick move buffer for neighbors is reused, eliminating small heap allocations inside the loop.

- No growing vectors or hashing during simulation; only in-place updates.

### Randomness Optimized

Switched to fastrand for ant placement and movement.

This is faster than the standard rand::thread_rng() approach.

### Simulation Control

- The simulation runs until all ants are dead or every surviving ant has reached the maximum move limit.

- This check currently requires scanning all ants (O(N) per tick).

### Further optimization is possible by using specialized data structures for ex:(AtomicU32, AtomicU64, spatial hashing, and parallel movement with Rayon.) , but I'm not entirely sure of its feasibility as I never used these things before, I have very little exp. with system programming.

### Benchmark on MacBook Air M2

| Ants (N) | Colonies (C) | Steps (max 10,000)  Total Runtime |
| -------- | ------------ | ------------------ | -------------|
| 1000     | 7000         | 10,000             | \~150 ms     |
| 10       | 30           | 10,000             | \~0.047791 ms|
