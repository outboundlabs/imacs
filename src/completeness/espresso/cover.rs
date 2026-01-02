//! Cover representation - a collection of cubes
//!
//! A cover represents a Boolean function as a set of product terms (cubes).
//! The function is the OR of all cubes in the cover.

use super::cube::{Cube, CubeValue};
use std::fmt;

/// A cover is a collection of cubes representing a Boolean function
#[derive(Debug, Clone)]
pub struct Cover {
    /// The cubes in this cover
    cubes: Vec<Cube>,
    /// Number of input variables
    num_inputs: usize,
    /// Number of output variables
    num_outputs: usize,
}

impl Cover {
    /// Create an empty cover
    pub fn new(num_inputs: usize, num_outputs: usize) -> Self {
        Cover {
            cubes: Vec::new(),
            num_inputs,
            num_outputs,
        }
    }

    /// Create a cover from a vector of cubes
    pub fn from_cubes(cubes: Vec<Cube>, num_inputs: usize, num_outputs: usize) -> Self {
        Cover {
            cubes,
            num_inputs,
            num_outputs,
        }
    }

    /// Number of cubes in the cover
    pub fn len(&self) -> usize {
        self.cubes.len()
    }

    /// Check if cover is empty
    pub fn is_empty(&self) -> bool {
        self.cubes.is_empty()
    }

    /// Number of input variables
    pub fn num_inputs(&self) -> usize {
        self.num_inputs
    }

    /// Number of output variables
    pub fn num_outputs(&self) -> usize {
        self.num_outputs
    }

    /// Add a cube to the cover
    pub fn add(&mut self, cube: Cube) {
        self.cubes.push(cube);
    }

    /// Remove a cube at index
    pub fn remove(&mut self, index: usize) -> Cube {
        self.cubes.remove(index)
    }

    /// Get a reference to a cube
    pub fn get(&self, index: usize) -> Option<&Cube> {
        self.cubes.get(index)
    }

    /// Get a mutable reference to a cube
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Cube> {
        self.cubes.get_mut(index)
    }

    /// Get all cubes
    pub fn cubes(&self) -> &[Cube] {
        &self.cubes
    }

    /// Get mutable access to all cubes
    pub fn cubes_mut(&mut self) -> &mut Vec<Cube> {
        &mut self.cubes
    }

    /// Iterate over cubes
    pub fn iter(&self) -> impl Iterator<Item = &Cube> {
        self.cubes.iter()
    }

    /// Iterate over cubes mutably
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Cube> {
        self.cubes.iter_mut()
    }

    /// Calculate the cost (total literals)
    pub fn cost(&self) -> usize {
        self.cubes.iter().map(|c| c.cost()).sum()
    }

    /// Calculate the total literal count
    pub fn literal_count(&self) -> usize {
        self.cubes.iter().map(|c| c.literal_count()).sum()
    }

    /// Check if this cover contains (covers) a cube
    pub fn contains_cube(&self, cube: &Cube) -> bool {
        self.cubes.iter().any(|c| c.contains(cube))
    }

    /// Check if this cover covers another cover
    pub fn covers(&self, other: &Cover) -> bool {
        other.cubes.iter().all(|c| self.contains_cube(c))
    }

    /// Remove cubes that are covered by other cubes
    pub fn remove_redundant(&mut self) {
        let mut i = 0;
        while i < self.cubes.len() {
            let mut is_redundant = false;
            for j in 0..self.cubes.len() {
                if i != j && self.cubes[j].contains(&self.cubes[i]) {
                    is_redundant = true;
                    break;
                }
            }
            if is_redundant {
                self.cubes.remove(i);
            } else {
                i += 1;
            }
        }
    }

    /// Perform distance-1 merge on the cover
    pub fn distance_1_merge(&mut self) {
        let mut changed = true;
        while changed {
            changed = false;
            let mut i = 0;
            while i < self.cubes.len() {
                let mut merged = false;
                for j in (i + 1)..self.cubes.len() {
                    if let Some(pos) = self.cubes[i].can_merge(&self.cubes[j]) {
                        let new_cube = self.cubes[i].merge(&self.cubes[j], pos);
                        self.cubes[i] = new_cube;
                        self.cubes.remove(j);
                        merged = true;
                        changed = true;
                        break;
                    }
                }
                if !merged {
                    i += 1;
                }
            }
        }
        self.remove_redundant();
    }

    /// Union of two covers
    pub fn union(&self, other: &Cover) -> Cover {
        let mut result = self.clone();
        for cube in &other.cubes {
            if !result.contains_cube(cube) {
                result.add(cube.clone());
            }
        }
        result
    }

    /// Intersection of two covers (all cubes that are in both)
    pub fn intersect(&self, other: &Cover) -> Cover {
        let mut result = Cover::new(self.num_inputs, self.num_outputs);
        for c1 in &self.cubes {
            for c2 in &other.cubes {
                if let Some(inter) = c1.intersect(c2) {
                    if inter.has_active_output() {
                        result.add(inter);
                    }
                }
            }
        }
        result.remove_redundant();
        result
    }

    /// Compute the complement of this cover using Shannon expansion
    pub fn complement(&self) -> Cover {
        self.complement_with_depth(0)
    }

    /// Internal complement with depth tracking to prevent stack overflow
    fn complement_with_depth(&self, depth: usize) -> Cover {
        // Prevent stack overflow with depth limit
        const MAX_DEPTH: usize = 30;
        if depth > MAX_DEPTH {
            // Return empty cover as fallback (conservative)
            return Cover::new(self.num_inputs, self.num_outputs);
        }

        if self.cubes.is_empty() {
            // Complement of empty = universe (all don't cares)
            let mut result = Cover::new(self.num_inputs, self.num_outputs);
            let mut tautology = Cube::new(self.num_inputs, self.num_outputs);
            for i in 0..self.num_outputs {
                tautology.set_output(i, CubeValue::One);
            }
            result.add(tautology);
            return result;
        }

        // Check for tautology
        if self.is_tautology_with_depth(depth) {
            return Cover::new(self.num_inputs, self.num_outputs);
        }

        // Find the best splitting variable (most binate)
        let split_var = self.find_splitting_variable();

        // Cofactor with respect to the splitting variable
        let cofactor_pos = self.cofactor(split_var, true);
        let cofactor_neg = self.cofactor(split_var, false);

        // Recursively complement
        let comp_pos = cofactor_pos.complement_with_depth(depth + 1);
        let comp_neg = cofactor_neg.complement_with_depth(depth + 1);

        // Combine results
        let mut result = Cover::new(self.num_inputs, self.num_outputs);

        // Add cubes from positive cofactor complement with variable = 1
        for cube in comp_pos.cubes {
            let mut new_cube = cube;
            if new_cube.input(split_var) == CubeValue::DontCare {
                new_cube.set_input(split_var, CubeValue::One);
            }
            result.add(new_cube);
        }

        // Add cubes from negative cofactor complement with variable = 0
        for cube in comp_neg.cubes {
            let mut new_cube = cube;
            if new_cube.input(split_var) == CubeValue::DontCare {
                new_cube.set_input(split_var, CubeValue::Zero);
            }
            result.add(new_cube);
        }

        result.remove_redundant();
        result
    }

    /// Check if the cover is a tautology (covers all minterms)
    pub fn is_tautology(&self) -> bool {
        self.is_tautology_with_depth(0)
    }

    /// Tautology check with depth tracking
    fn is_tautology_with_depth(&self, depth: usize) -> bool {
        // Prevent stack overflow
        const MAX_DEPTH: usize = 30;
        if depth > MAX_DEPTH {
            return false; // Conservative: assume not tautology
        }

        // Simple check: if any cube is a tautology, the cover is
        if self.cubes.iter().any(|c| c.is_tautology()) {
            return true;
        }

        if self.cubes.is_empty() {
            return false;
        }

        // Check if unate in all variables - unate covers can't be tautologies
        // unless they contain a tautology cube (already checked above)
        if self.is_unate() {
            return false;
        }

        // Find splitting variable
        let split_var = self.find_splitting_variable();

        // Check both cofactors
        let cofactor_pos = self.cofactor(split_var, true);
        let cofactor_neg = self.cofactor(split_var, false);

        cofactor_pos.is_tautology_with_depth(depth + 1)
            && cofactor_neg.is_tautology_with_depth(depth + 1)
    }

    /// Check if the cover is unate (monotone in each variable)
    pub fn is_unate(&self) -> bool {
        for var in 0..self.num_inputs {
            let mut has_pos = false;
            let mut has_neg = false;
            for cube in &self.cubes {
                match cube.input(var) {
                    CubeValue::One => has_pos = true,
                    CubeValue::Zero => has_neg = true,
                    CubeValue::DontCare => {}
                }
            }
            if has_pos && has_neg {
                return false;
            }
        }
        true
    }

    /// Find the best variable to split on (most binate)
    fn find_splitting_variable(&self) -> usize {
        let mut best_var = 0;
        let mut best_score = 0;
        let mut found_valid = false;

        for var in 0..self.num_inputs {
            let mut pos_count = 0;  // Count cubes with explicit 1
            let mut neg_count = 0;  // Count cubes with explicit 0
            let mut dc_count = 0;   // Count cubes with don't care

            for cube in &self.cubes {
                match cube.input(var) {
                    CubeValue::One => pos_count += 1,
                    CubeValue::Zero => neg_count += 1,
                    CubeValue::DontCare => dc_count += 1,
                }
            }

            // Skip variables that are all don't-care (cofactoring won't help)
            if dc_count == self.cubes.len() {
                continue;
            }

            // Score is min(pos + dc, neg + dc) - prefer balanced splits
            // Only count variables that have actual literals
            let effective_pos = pos_count + dc_count;
            let effective_neg = neg_count + dc_count;
            let score = effective_pos.min(effective_neg);

            // Prefer variables with actual literals (not all don't care)
            if score > best_score || !found_valid {
                best_score = score;
                best_var = var;
                found_valid = true;
            }
        }
        best_var
    }

    /// Compute cofactor with respect to a variable
    pub fn cofactor(&self, var: usize, positive: bool) -> Cover {
        let mut result = Cover::new(self.num_inputs, self.num_outputs);
        for cube in &self.cubes {
            let cofactored = if positive {
                cube.cofactor_true(var)
            } else {
                cube.cofactor_false(var)
            };
            if let Some(c) = cofactored {
                result.add(c);
            }
        }
        result
    }

    /// Check if a cube intersects with the OFF-set (represented by the complement)
    pub fn cube_intersects_off(&self, cube: &Cube, off_set: &Cover) -> bool {
        for off_cube in &off_set.cubes {
            if cube.intersects(off_cube) {
                return true;
            }
        }
        false
    }

    /// Clear the cover
    pub fn clear(&mut self) {
        self.cubes.clear();
    }

    /// Extract cubes for a single output
    pub fn extract_single_output(&self, output: usize) -> Cover {
        let mut result = Cover::new(self.num_inputs, 1);
        for cube in &self.cubes {
            if cube.output(output) == CubeValue::One {
                let mut new_cube = Cube::new(self.num_inputs, 1);
                for i in 0..self.num_inputs {
                    new_cube.set_input(i, cube.input(i));
                }
                new_cube.set_output(0, CubeValue::One);
                result.add(new_cube);
            }
        }
        result
    }

    /// Sort cubes by literal count (fewest literals first)
    pub fn sort_by_size(&mut self) {
        self.cubes.sort_by_key(|c| c.literal_count());
    }

    /// Sort cubes by number of literals (most literals first, for reduce)
    pub fn sort_by_size_desc(&mut self) {
        self.cubes.sort_by_key(|c| std::cmp::Reverse(c.literal_count()));
    }
}

impl fmt::Display for Cover {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for cube in &self.cubes {
            writeln!(f, "{}", cube)?;
        }
        Ok(())
    }
}

impl IntoIterator for Cover {
    type Item = Cube;
    type IntoIter = std::vec::IntoIter<Cube>;

    fn into_iter(self) -> Self::IntoIter {
        self.cubes.into_iter()
    }
}

impl<'a> IntoIterator for &'a Cover {
    type Item = &'a Cube;
    type IntoIter = std::slice::Iter<'a, Cube>;

    fn into_iter(self) -> Self::IntoIter {
        self.cubes.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cover_creation() {
        let mut cover = Cover::new(2, 1);
        cover.add(Cube::from_str("10", "1").unwrap());
        cover.add(Cube::from_str("01", "1").unwrap());
        
        assert_eq!(cover.len(), 2);
        assert_eq!(cover.num_inputs(), 2);
        assert_eq!(cover.num_outputs(), 1);
    }

    #[test]
    fn test_distance_1_merge() {
        let mut cover = Cover::new(2, 1);
        cover.add(Cube::from_str("10", "1").unwrap());
        cover.add(Cube::from_str("11", "1").unwrap());
        
        cover.distance_1_merge();
        
        assert_eq!(cover.len(), 1);
        assert_eq!(cover.get(0).unwrap().input(0), CubeValue::One);
        assert_eq!(cover.get(0).unwrap().input(1), CubeValue::DontCare);
    }

    #[test]
    fn test_remove_redundant() {
        let mut cover = Cover::new(2, 1);
        cover.add(Cube::from_str("1-", "1").unwrap()); // Covers 10 and 11
        cover.add(Cube::from_str("10", "1").unwrap()); // Redundant
        
        cover.remove_redundant();
        
        assert_eq!(cover.len(), 1);
    }

    #[test]
    fn test_cofactor() {
        let mut cover = Cover::new(2, 1);
        cover.add(Cube::from_str("1-", "1").unwrap());
        cover.add(Cube::from_str("01", "1").unwrap());
        
        let cofactor = cover.cofactor(0, true);
        assert_eq!(cofactor.len(), 1); // Only "1-" contributes
        
        let cofactor = cover.cofactor(0, false);
        assert_eq!(cofactor.len(), 1); // Only "01" contributes
    }

    #[test]
    fn test_tautology() {
        let mut cover = Cover::new(2, 1);
        cover.add(Cube::from_str("--", "1").unwrap());
        assert!(cover.is_tautology());

        let mut cover2 = Cover::new(2, 1);
        cover2.add(Cube::from_str("10", "1").unwrap());
        assert!(!cover2.is_tautology());
    }
}
