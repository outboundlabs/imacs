//! Cube representation for Boolean functions
//!
//! A cube represents a product term in a Boolean function. Each variable
//! can be in one of three states: must be 0, must be 1, or don't care.

use super::error::EspressoError;
use std::fmt;

/// Value of a single variable in a cube
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CubeValue {
    /// Variable must be false (complemented)
    Zero,
    /// Variable must be true (uncomplemented)
    One,
    /// Variable can be either (don't care)
    DontCare,
}

impl CubeValue {
    /// Parse a character into a CubeValue
    pub fn from_char(c: char) -> Result<Self, EspressoError> {
        match c {
            '0' => Ok(CubeValue::Zero),
            '1' => Ok(CubeValue::One),
            '-' | '2' | 'x' | 'X' => Ok(CubeValue::DontCare),
            _ => Err(EspressoError::InvalidCube(format!(
                "Invalid character '{}' in cube",
                c
            ))),
        }
    }

    /// Convert to character representation
    pub fn to_char(self) -> char {
        match self {
            CubeValue::Zero => '0',
            CubeValue::One => '1',
            CubeValue::DontCare => '-',
        }
    }

    /// Check if this value is a literal (not don't care)
    pub fn is_literal(self) -> bool {
        matches!(self, CubeValue::Zero | CubeValue::One)
    }

    /// Complement this value (0 <-> 1, - stays -)
    pub fn complement(self) -> Self {
        match self {
            CubeValue::Zero => CubeValue::One,
            CubeValue::One => CubeValue::Zero,
            CubeValue::DontCare => CubeValue::DontCare,
        }
    }

    /// Intersection of two values (for cube intersection)
    /// Returns None if the values are incompatible (0 and 1)
    pub fn intersect(self, other: Self) -> Option<Self> {
        match (self, other) {
            (CubeValue::Zero, CubeValue::One) | (CubeValue::One, CubeValue::Zero) => None,
            (CubeValue::DontCare, x) | (x, CubeValue::DontCare) => Some(x),
            (x, _) => Some(x),
        }
    }

    /// Supercube (union) of two values
    pub fn supercube(self, other: Self) -> Self {
        match (self, other) {
            (CubeValue::Zero, CubeValue::Zero) => CubeValue::Zero,
            (CubeValue::One, CubeValue::One) => CubeValue::One,
            _ => CubeValue::DontCare,
        }
    }
}

/// A cube represents a product term in a Boolean function
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Cube {
    /// Input variables
    inputs: Vec<CubeValue>,
    /// Output values
    outputs: Vec<CubeValue>,
}

impl Cube {
    /// Create a new cube with all don't cares
    pub fn new(num_inputs: usize, num_outputs: usize) -> Self {
        Cube {
            inputs: vec![CubeValue::DontCare; num_inputs],
            outputs: vec![CubeValue::DontCare; num_outputs],
        }
    }

    /// Create a cube with specific input values and all outputs active
    pub fn from_inputs(inputs: Vec<CubeValue>, num_outputs: usize) -> Self {
        Cube {
            inputs,
            outputs: vec![CubeValue::One; num_outputs],
        }
    }

    /// Parse a cube from string representations
    pub fn from_str(input_str: &str, output_str: &str) -> Result<Self, EspressoError> {
        let inputs: Result<Vec<CubeValue>, _> =
            input_str.chars().map(CubeValue::from_char).collect();
        let outputs: Result<Vec<CubeValue>, _> =
            output_str.chars().map(CubeValue::from_char).collect();

        Ok(Cube {
            inputs: inputs?,
            outputs: outputs?,
        })
    }

    /// Parse from a single line (space-separated input and output)
    pub fn parse_line(line: &str, num_inputs: usize, num_outputs: usize) -> Result<Self, EspressoError> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        
        if parts.is_empty() {
            return Err(EspressoError::InvalidCube("Empty line".to_string()));
        }

        // Handle both space-separated and combined formats
        let (input_str, output_str) = if parts.len() >= 2 {
            (parts[0], parts[1])
        } else if parts[0].len() == num_inputs + num_outputs {
            // Combined format without space
            let s = parts[0];
            (&s[..num_inputs], &s[num_inputs..])
        } else if parts[0].len() == num_inputs {
            // Just inputs, assume all outputs are 1
            (parts[0], "")
        } else {
            return Err(EspressoError::InvalidCube(format!(
                "Cannot parse cube: '{}'",
                line
            )));
        };

        let inputs: Result<Vec<CubeValue>, _> =
            input_str.chars().map(CubeValue::from_char).collect();
        let inputs = inputs?;

        if inputs.len() != num_inputs {
            return Err(EspressoError::DimensionMismatch {
                expected: num_inputs,
                got: inputs.len(),
            });
        }

        let outputs = if output_str.is_empty() {
            vec![CubeValue::One; num_outputs]
        } else {
            let outputs: Result<Vec<CubeValue>, _> =
                output_str.chars().map(CubeValue::from_char).collect();
            outputs?
        };

        if outputs.len() != num_outputs {
            return Err(EspressoError::OutputMismatch {
                expected: num_outputs,
                got: outputs.len(),
            });
        }

        Ok(Cube { inputs, outputs })
    }

    /// Number of input variables
    pub fn num_inputs(&self) -> usize {
        self.inputs.len()
    }

    /// Number of output variables
    pub fn num_outputs(&self) -> usize {
        self.outputs.len()
    }

    /// Get input value at index
    pub fn input(&self, i: usize) -> CubeValue {
        self.inputs[i]
    }

    /// Get output value at index
    pub fn output(&self, i: usize) -> CubeValue {
        self.outputs[i]
    }

    /// Set input value at index
    pub fn set_input(&mut self, i: usize, val: CubeValue) {
        self.inputs[i] = val;
    }

    /// Set output value at index
    pub fn set_output(&mut self, i: usize, val: CubeValue) {
        self.outputs[i] = val;
    }

    /// Get all inputs
    pub fn inputs(&self) -> &[CubeValue] {
        &self.inputs
    }

    /// Get all outputs
    pub fn outputs(&self) -> &[CubeValue] {
        &self.outputs
    }

    /// Get mutable inputs
    pub fn inputs_mut(&mut self) -> &mut [CubeValue] {
        &mut self.inputs
    }

    /// Get mutable outputs
    pub fn outputs_mut(&mut self) -> &mut [CubeValue] {
        &mut self.outputs
    }

    /// Count the number of literals (non-don't-care values) in inputs
    pub fn literal_count(&self) -> usize {
        self.inputs.iter().filter(|v| v.is_literal()).count()
    }

    /// Count the total cost (literals * active outputs)
    pub fn cost(&self) -> usize {
        let literals = self.literal_count();
        let active_outputs = self.outputs.iter().filter(|v| **v == CubeValue::One).count();
        if active_outputs == 0 {
            0
        } else {
            literals
        }
    }

    /// Check if this cube is a tautology (all don't cares)
    pub fn is_tautology(&self) -> bool {
        self.inputs.iter().all(|v| *v == CubeValue::DontCare)
    }

    /// Check if this cube is empty (contradiction)
    pub fn is_empty(&self) -> bool {
        false // A valid cube is never empty; emptiness is checked via intersection
    }

    /// Check if this cube has any active output
    pub fn has_active_output(&self) -> bool {
        self.outputs.iter().any(|v| *v == CubeValue::One)
    }

    /// Check if two cubes can be merged (differ in exactly one variable)
    pub fn can_merge(&self, other: &Cube) -> Option<usize> {
        if self.outputs != other.outputs {
            return None;
        }

        let mut diff_pos = None;
        for i in 0..self.inputs.len() {
            if self.inputs[i] != other.inputs[i] {
                // Check if they're complements
                if (self.inputs[i] == CubeValue::Zero && other.inputs[i] == CubeValue::One)
                    || (self.inputs[i] == CubeValue::One && other.inputs[i] == CubeValue::Zero)
                {
                    if diff_pos.is_some() {
                        return None; // More than one difference
                    }
                    diff_pos = Some(i);
                } else {
                    return None; // Not a complement difference
                }
            }
        }
        diff_pos
    }

    /// Merge two cubes that differ in exactly one variable
    pub fn merge(&self, other: &Cube, diff_pos: usize) -> Cube {
        let mut result = self.clone();
        result.inputs[diff_pos] = CubeValue::DontCare;
        result
    }

    /// Compute the intersection of two cubes (for input part)
    /// Returns None if cubes are disjoint
    pub fn intersect(&self, other: &Cube) -> Option<Cube> {
        if self.inputs.len() != other.inputs.len() {
            return None;
        }

        let mut result = self.clone();
        for i in 0..self.inputs.len() {
            match self.inputs[i].intersect(other.inputs[i]) {
                Some(v) => result.inputs[i] = v,
                None => return None,
            }
        }

        // Intersect outputs
        for i in 0..self.outputs.len().min(other.outputs.len()) {
            match self.outputs[i].intersect(other.outputs[i]) {
                Some(v) => result.outputs[i] = v,
                None => result.outputs[i] = CubeValue::Zero,
            }
        }

        Some(result)
    }

    /// Compute the supercube (smallest cube containing both)
    pub fn supercube(&self, other: &Cube) -> Cube {
        let mut result = self.clone();
        for i in 0..self.inputs.len().min(other.inputs.len()) {
            result.inputs[i] = self.inputs[i].supercube(other.inputs[i]);
        }
        for i in 0..self.outputs.len().min(other.outputs.len()) {
            result.outputs[i] = self.outputs[i].supercube(other.outputs[i]);
        }
        result
    }

    /// Check if this cube contains (covers) another cube
    pub fn contains(&self, other: &Cube) -> bool {
        if self.inputs.len() != other.inputs.len() {
            return false;
        }

        for i in 0..self.inputs.len() {
            match self.inputs[i] {
                CubeValue::DontCare => continue,
                CubeValue::Zero => {
                    if other.inputs[i] == CubeValue::One {
                        return false;
                    }
                }
                CubeValue::One => {
                    if other.inputs[i] == CubeValue::Zero {
                        return false;
                    }
                }
            }
        }

        // Check outputs
        for i in 0..self.outputs.len().min(other.outputs.len()) {
            if other.outputs[i] == CubeValue::One && self.outputs[i] != CubeValue::One {
                return false;
            }
        }

        true
    }

    /// Check if this cube intersects with another (shares at least one minterm)
    pub fn intersects(&self, other: &Cube) -> bool {
        self.intersect(other).is_some()
    }

    /// Check if the input parts are disjoint
    pub fn inputs_disjoint(&self, other: &Cube) -> bool {
        for i in 0..self.inputs.len().min(other.inputs.len()) {
            if self.inputs[i].intersect(other.inputs[i]).is_none() {
                return true;
            }
        }
        false
    }

    /// Get distance between two cubes (number of positions where they differ)
    pub fn distance(&self, other: &Cube) -> usize {
        self.inputs
            .iter()
            .zip(other.inputs.iter())
            .filter(|(a, b)| a != b)
            .count()
    }

    /// Cofactor with respect to a variable being true
    pub fn cofactor_true(&self, var: usize) -> Option<Cube> {
        match self.inputs[var] {
            CubeValue::Zero => None, // Cube disappears
            _ => {
                let mut result = self.clone();
                result.inputs[var] = CubeValue::DontCare;
                Some(result)
            }
        }
    }

    /// Cofactor with respect to a variable being false
    pub fn cofactor_false(&self, var: usize) -> Option<Cube> {
        match self.inputs[var] {
            CubeValue::One => None, // Cube disappears
            _ => {
                let mut result = self.clone();
                result.inputs[var] = CubeValue::DontCare;
                Some(result)
            }
        }
    }

    /// Complement the cube (De Morgan's law)
    pub fn complement(&self) -> Vec<Cube> {
        let mut result = Vec::new();
        
        for i in 0..self.inputs.len() {
            if self.inputs[i].is_literal() {
                let mut new_cube = Cube::new(self.inputs.len(), self.outputs.len());
                new_cube.inputs[i] = self.inputs[i].complement();
                new_cube.outputs = self.outputs.clone();
                result.push(new_cube);
            }
        }
        
        if result.is_empty() {
            // Empty cube - no complement
        }
        
        result
    }
}

impl fmt::Display for Cube {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let inputs: String = self.inputs.iter().map(|v| v.to_char()).collect();
        let outputs: String = self.outputs.iter().map(|v| v.to_char()).collect();
        write!(f, "{} {}", inputs, outputs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cube_creation() {
        let cube = Cube::from_str("10-", "1").unwrap();
        assert_eq!(cube.num_inputs(), 3);
        assert_eq!(cube.num_outputs(), 1);
        assert_eq!(cube.input(0), CubeValue::One);
        assert_eq!(cube.input(1), CubeValue::Zero);
        assert_eq!(cube.input(2), CubeValue::DontCare);
    }

    #[test]
    fn test_cube_merge() {
        let c1 = Cube::from_str("10", "1").unwrap();
        let c2 = Cube::from_str("11", "1").unwrap();
        
        let diff = c1.can_merge(&c2);
        assert_eq!(diff, Some(1));
        
        let merged = c1.merge(&c2, 1);
        assert_eq!(merged.input(0), CubeValue::One);
        assert_eq!(merged.input(1), CubeValue::DontCare);
    }

    #[test]
    fn test_cube_contains() {
        let c1 = Cube::from_str("1-", "1").unwrap(); // Covers 10 and 11
        let c2 = Cube::from_str("10", "1").unwrap();
        let c3 = Cube::from_str("01", "1").unwrap();
        
        assert!(c1.contains(&c2));
        assert!(!c1.contains(&c3));
    }

    #[test]
    fn test_cube_intersection() {
        let c1 = Cube::from_str("1-", "1").unwrap();
        let c2 = Cube::from_str("-0", "1").unwrap();
        
        let inter = c1.intersect(&c2).unwrap();
        assert_eq!(inter.input(0), CubeValue::One);
        assert_eq!(inter.input(1), CubeValue::Zero);
    }

    #[test]
    fn test_disjoint_cubes() {
        let c1 = Cube::from_str("10", "1").unwrap();
        let c2 = Cube::from_str("01", "1").unwrap();
        
        assert!(c1.intersect(&c2).is_none());
    }

    #[test]
    fn test_literal_count() {
        let c1 = Cube::from_str("10-1", "1").unwrap();
        assert_eq!(c1.literal_count(), 3);
        
        let c2 = Cube::from_str("----", "1").unwrap();
        assert_eq!(c2.literal_count(), 0);
    }
}
