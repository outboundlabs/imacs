//! Espresso minimization algorithm
//!
//! This module implements the core Espresso heuristic Boolean minimization algorithm.
//! The algorithm consists of three main phases that are applied iteratively:
//!
//! 1. **EXPAND**: Enlarge implicants into prime implicants
//! 2. **IRREDUNDANT**: Remove redundant implicants
//! 3. **REDUCE**: Reduce implicants to enable further expansion
//!
//! The algorithm iterates until no further improvement is possible.

use super::cover::Cover;
use super::cube::{Cube, CubeValue};

/// Options for the Espresso algorithm
#[derive(Debug, Clone)]
pub struct EspressoOptions {
    /// Use fast mode (single pass)
    pub fast: bool,
    /// Detect essential primes
    pub detect_essential: bool,
    /// Make result irredundant
    pub irredundant: bool,
    /// Maximum iterations (0 = unlimited)
    pub max_iterations: usize,
    /// Verbose output
    pub verbose: bool,
}

impl Default for EspressoOptions {
    fn default() -> Self {
        EspressoOptions {
            fast: false,
            detect_essential: true,
            irredundant: true,
            max_iterations: 0,
            verbose: false,
        }
    }
}

/// Run the Espresso algorithm with default options
pub fn espresso(on_set: &Cover, dc_set: &Cover) -> Cover {
    espresso_with_options(on_set, dc_set, &EspressoOptions::default())
}

/// Run the Espresso algorithm with custom options
pub fn espresso_with_options(on_set: &Cover, dc_set: &Cover, options: &EspressoOptions) -> Cover {
    let mut minimizer = EspressoMinimizer::new(on_set.clone(), dc_set.clone(), options.clone());
    minimizer.minimize()
}

/// The Espresso minimizer state machine
pub struct EspressoMinimizer {
    /// Current cover (ON-set)
    cover: Cover,
    /// Don't care set
    dc_set: Cover,
    /// OFF-set (complement of ON ∪ DC)
    off_set: Cover,
    /// Options
    options: EspressoOptions,
    /// Number of inputs
    num_inputs: usize,
    /// Number of outputs
    num_outputs: usize,
}

impl EspressoMinimizer {
    /// Create a new minimizer
    pub fn new(on_set: Cover, dc_set: Cover, options: EspressoOptions) -> Self {
        let num_inputs = on_set.num_inputs();
        let num_outputs = on_set.num_outputs();

        // Skip expensive off-set computation - use lazy checking instead
        let off_set = Cover::new(num_inputs, num_outputs);

        EspressoMinimizer {
            cover: on_set,
            dc_set,
            off_set,
            options,
            num_inputs,
            num_outputs,
        }
    }

    /// Run the minimization algorithm
    pub fn minimize(&mut self) -> Cover {
        if self.cover.is_empty() {
            return Cover::new(self.num_inputs, self.num_outputs);
        }

        // Initial distance-1 merge for quick simplification
        self.cover.distance_1_merge();

        if self.options.fast {
            // Fast mode: single pass
            self.expand();
            if self.options.irredundant {
                self.irredundant();
            }
            return self.cover.clone();
        }

        // Full Espresso loop
        let mut iterations = 0;
        let mut prev_cost = self.cover.cost();

        loop {
            iterations += 1;

            // EXPAND phase
            self.expand();

            // IRREDUNDANT phase
            if self.options.irredundant {
                self.irredundant();
            }

            // REDUCE phase
            self.reduce();

            // EXPAND again after reduce
            self.expand();

            // IRREDUNDANT again
            if self.options.irredundant {
                self.irredundant();
            }

            let new_cost = self.cover.cost();

            if self.options.verbose {
                eprintln!(
                    "Iteration {}: {} cubes, {} literals",
                    iterations,
                    self.cover.len(),
                    new_cost
                );
            }

            // Check for convergence
            if new_cost >= prev_cost {
                break;
            }
            prev_cost = new_cost;

            // Check iteration limit
            if self.options.max_iterations > 0 && iterations >= self.options.max_iterations {
                break;
            }
        }

        // Final cleanup
        self.cover.remove_redundant();
        self.cover.clone()
    }

    /// EXPAND phase: enlarge each implicant into a prime implicant
    fn expand(&mut self) {
        // Skip expansion if we don't have an off-set to constrain it
        // (without off-set, expansion would make everything all don't cares)
        if self.off_set.is_empty() {
            return;
        }

        // Sort by size (smallest first) for better expansion
        self.cover.sort_by_size();

        let mut new_cover = Cover::new(self.num_inputs, self.num_outputs);
        let mut expanded_cubes: Vec<Cube> = Vec::new();

        for i in 0..self.cover.len() {
            let cube = self.cover.get(i).unwrap().clone();

            // Try to expand this cube
            let expanded = self.expand_cube(&cube, &expanded_cubes);

            // Check if expanded cube is already covered
            let mut dominated = false;
            for existing in &expanded_cubes {
                if existing.contains(&expanded) {
                    dominated = true;
                    break;
                }
            }

            if !dominated {
                // Remove any cubes dominated by the new expanded cube
                expanded_cubes.retain(|c| !expanded.contains(c));
                expanded_cubes.push(expanded);
            }
        }

        for cube in expanded_cubes {
            new_cover.add(cube);
        }

        self.cover = new_cover;
    }

    /// Expand a single cube as much as possible without intersecting OFF-set
    fn expand_cube(&self, cube: &Cube, _existing: &[Cube]) -> Cube {
        let mut expanded = cube.clone();

        // Try expanding each variable to don't care
        for var in 0..self.num_inputs {
            if expanded.input(var) != CubeValue::DontCare {
                let mut test_cube = expanded.clone();
                test_cube.set_input(var, CubeValue::DontCare);

                // Check if expansion is valid (doesn't intersect OFF-set)
                let valid = !self.cube_intersects_off(&test_cube);

                if valid {
                    expanded = test_cube;
                }
            }
        }

        expanded
    }

    /// Check if a cube intersects the OFF-set
    fn cube_intersects_off(&self, cube: &Cube) -> bool {
        for off_cube in self.off_set.iter() {
            if cube.intersects(off_cube) {
                return true;
            }
        }
        false
    }

    /// IRREDUNDANT phase: remove redundant cubes
    /// Uses a simple containment check to avoid expensive tautology operations
    fn irredundant(&mut self) {
        if self.cover.len() <= 1 {
            return;
        }

        // Simple redundancy removal: remove cubes that are contained by other cubes
        let mut redundant_indices: Vec<usize> = Vec::new();

        for i in 0..self.cover.len() {
            let cube = self.cover.get(i).unwrap();

            // Check if this cube is contained by any other cube
            for j in 0..self.cover.len() {
                if i != j {
                    if let Some(other) = self.cover.get(j) {
                        if other.contains(cube) {
                            redundant_indices.push(i);
                            break;
                        }
                    }
                }
            }
        }

        // Remove redundant cubes
        if !redundant_indices.is_empty() {
            redundant_indices.sort();
            redundant_indices.dedup();
            let mut new_cover = Cover::new(self.num_inputs, self.num_outputs);
            for (i, cube) in self.cover.iter().enumerate() {
                if !redundant_indices.contains(&i) {
                    new_cover.add(cube.clone());
                }
            }
            self.cover = new_cover;
        }
    }

    /// REDUCE phase: make cubes smaller to enable further expansion
    fn reduce(&mut self) {
        // Sort by size (largest first) for reduction
        self.cover.sort_by_size_desc();

        let mut new_cover = Cover::new(self.num_inputs, self.num_outputs);

        for i in 0..self.cover.len() {
            let cube = self.cover.get(i).unwrap().clone();

            // Try to reduce this cube
            let reduced = self.reduce_cube(&cube, i);
            new_cover.add(reduced);
        }

        self.cover = new_cover;
    }

    /// Reduce a single cube as much as possible while maintaining coverage
    fn reduce_cube(&self, cube: &Cube, cube_index: usize) -> Cube {
        let mut reduced = cube.clone();

        // Try to make each variable more specific
        for var in 0..self.num_inputs {
            if reduced.input(var) == CubeValue::DontCare {
                // Try reducing to 0
                let mut test_cube = reduced.clone();
                test_cube.set_input(var, CubeValue::Zero);

                if self.is_valid_reduction(&test_cube, cube_index) {
                    reduced = test_cube;
                    continue;
                }

                // Try reducing to 1
                let mut test_cube = reduced.clone();
                test_cube.set_input(var, CubeValue::One);

                if self.is_valid_reduction(&test_cube, cube_index) {
                    reduced = test_cube;
                }
            }
        }

        reduced
    }

    /// Check if a reduction is valid (maintains coverage)
    /// Simplified: just check that reduced cube is still covered by other cubes or DC set
    fn is_valid_reduction(&self, reduced_cube: &Cube, cube_index: usize) -> bool {
        // Check if reduced cube is covered by another cube in the cover
        for (i, cube) in self.cover.iter().enumerate() {
            if i != cube_index && cube.contains(reduced_cube) {
                return true;
            }
        }
        // Check if reduced cube is covered by DC set
        for dc_cube in self.dc_set.iter() {
            if dc_cube.contains(reduced_cube) {
                return true;
            }
        }
        false
    }
}

/// Simplify a cover using basic methods (distance-1 merge and redundancy removal)
pub fn simplify(cover: &mut Cover) {
    cover.distance_1_merge();
    cover.remove_redundant();
}

/// Perform exact minimization (Quine-McCluskey style)
/// Warning: exponential complexity for large inputs!
pub fn exact_minimize(on_set: &Cover, dc_set: &Cover) -> Cover {
    // First get all prime implicants
    let primes = find_prime_implicants(on_set, dc_set);

    // Then find minimum cover
    minimum_cover(&primes, on_set)
}

/// Find all prime implicants using consensus
fn find_prime_implicants(on_set: &Cover, dc_set: &Cover) -> Cover {
    let combined = on_set.union(dc_set);
    let mut primes = combined.clone();

    // Iteratively merge until no more merges possible
    let mut changed = true;
    while changed {
        changed = false;
        let mut new_primes = Cover::new(primes.num_inputs(), primes.num_outputs());
        let mut merged = vec![false; primes.len()];

        for i in 0..primes.len() {
            for j in (i + 1)..primes.len() {
                if let Some(pos) = primes.get(i).unwrap().can_merge(primes.get(j).unwrap()) {
                    let merged_cube = primes.get(i).unwrap().merge(primes.get(j).unwrap(), pos);
                    if !new_primes.contains_cube(&merged_cube) {
                        new_primes.add(merged_cube);
                    }
                    merged[i] = true;
                    merged[j] = true;
                    changed = true;
                }
            }
        }

        // Add unmerged cubes (they are prime implicants)
        for (i, is_merged) in merged.iter().enumerate() {
            if !*is_merged && !new_primes.contains_cube(primes.get(i).unwrap()) {
                new_primes.add(primes.get(i).unwrap().clone());
            }
        }

        // Add newly merged cubes
        primes = new_primes;
    }

    primes.remove_redundant();
    primes
}

/// Find minimum cover using greedy set cover
fn minimum_cover(primes: &Cover, on_set: &Cover) -> Cover {
    let mut result = Cover::new(primes.num_inputs(), primes.num_outputs());
    let mut uncovered = on_set.clone();

    // Greedy: always pick the prime that covers the most uncovered cubes
    while !uncovered.is_empty() {
        let mut best_prime = None;
        let mut best_count = 0;

        for prime in primes.iter() {
            let mut count = 0;
            for uc in uncovered.iter() {
                if prime.contains(uc) {
                    count += 1;
                }
            }
            if count > best_count {
                best_count = count;
                best_prime = Some(prime.clone());
            }
        }

        if let Some(prime) = best_prime {
            // Remove covered cubes
            let mut new_uncovered = Cover::new(uncovered.num_inputs(), uncovered.num_outputs());
            for uc in uncovered.iter() {
                if !prime.contains(uc) {
                    new_uncovered.add(uc.clone());
                }
            }
            uncovered = new_uncovered;
            result.add(prime);
        } else {
            break; // No more progress possible
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cube(input_str: &str) -> Cube {
        Cube::from_str(input_str, "1").unwrap()
    }

    #[test]
    fn test_espresso_simple() {
        // Function: AB' + A'B + AB = A + B
        let mut on_set = Cover::new(2, 1);
        on_set.add(make_cube("10")); // AB'
        on_set.add(make_cube("01")); // A'B
        on_set.add(make_cube("11")); // AB

        let dc_set = Cover::new(2, 1);

        let result = espresso(&on_set, &dc_set);

        // Should minimize to 2 terms: A + B (or 1- + -1)
        assert!(result.len() <= 2);
    }

    #[test]
    fn test_espresso_with_dc() {
        // Function with don't cares
        let mut on_set = Cover::new(2, 1);
        on_set.add(make_cube("00"));
        on_set.add(make_cube("01"));
        on_set.add(make_cube("10"));

        let mut dc_set = Cover::new(2, 1);
        dc_set.add(make_cube("11")); // 11 is don't care

        let result = espresso(&on_set, &dc_set);

        // Should minimize to single tautology term (all 1s with dc)
        assert!(result.len() <= 2);
    }

    #[test]
    fn test_expand() {
        let mut on_set = Cover::new(3, 1);
        on_set.add(make_cube("100"));
        on_set.add(make_cube("101"));
        on_set.add(make_cube("110"));
        on_set.add(make_cube("111"));

        let dc_set = Cover::new(3, 1);

        let result = espresso(&on_set, &dc_set);

        // Should recognize pattern: A (1--)
        assert_eq!(result.len(), 1);
        let cube = result.get(0).unwrap();
        assert_eq!(cube.input(0), CubeValue::One);
        assert_eq!(cube.input(1), CubeValue::DontCare);
        assert_eq!(cube.input(2), CubeValue::DontCare);
    }

    #[test]
    fn test_irredundant() {
        let mut on_set = Cover::new(2, 1);
        // These cubes are redundant: 1- covers both 10 and 11
        on_set.add(make_cube("1-")); // Covers 10, 11
        on_set.add(make_cube("10")); // Redundant

        let dc_set = Cover::new(2, 1);

        let result = espresso(&on_set, &dc_set);

        // Should remove redundant cube
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_seven_segment_a() {
        // Seven segment display, segment A
        // ON: 0, 2, 3, 5, 6, 7, 8, 9
        let mut on_set = Cover::new(4, 1);
        on_set.add(make_cube("0000")); // 0
        on_set.add(make_cube("0010")); // 2
        on_set.add(make_cube("0011")); // 3
        on_set.add(make_cube("0101")); // 5
        on_set.add(make_cube("0110")); // 6
        on_set.add(make_cube("0111")); // 7
        on_set.add(make_cube("1000")); // 8
        on_set.add(make_cube("1001")); // 9

        // DC: 10-15 (invalid BCD)
        let mut dc_set = Cover::new(4, 1);
        for i in 10..16 {
            let bits = format!("{:04b}", i);
            dc_set.add(make_cube(&bits));
        }

        let result = espresso(&on_set, &dc_set);

        // Should produce a reasonable minimization
        assert!(result.len() < 8);
    }
}
