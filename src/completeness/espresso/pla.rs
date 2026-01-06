//! PLA file format parser
//!
//! Parses the Berkeley PLA format used by Espresso.
//!
//! Example PLA file:
//! ```text
//! .i 4
//! .o 1
//! .ilb A B C D
//! .ob F
//! 0000 1
//! 0001 0
//! 001- 1
//! .e
//! ```

use super::cover::Cover;
use super::cube::Cube;
use super::error::EspressoError;
use std::io::{BufRead, BufReader, Read, Write};

/// PLA file type specification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlaType {
    /// ON-set only (f)
    #[default]
    F,
    /// OFF-set only (r)
    R,
    /// DC-set only (d)
    D,
    /// ON-set and DC-set (fd)
    Fd,
    /// ON-set and OFF-set (fr)
    Fr,
    /// DC-set and OFF-set (dr)
    Dr,
    /// All three sets (fdr)
    Fdr,
}

/// Represents a parsed PLA file
#[derive(Debug, Clone)]
pub struct Pla {
    /// Number of inputs
    pub num_inputs: usize,
    /// Number of outputs
    pub num_outputs: usize,
    /// Number of product terms
    pub num_products: usize,
    /// Input labels
    pub input_labels: Vec<String>,
    /// Output labels
    pub output_labels: Vec<String>,
    /// PLA type
    pub pla_type: PlaType,
    /// ON-set cover
    pub on_set: Cover,
    /// OFF-set cover (complement)
    pub off_set: Cover,
    /// DC-set cover (don't cares)
    pub dc_set: Cover,
    /// Comments
    pub comments: Vec<String>,
}

impl Pla {
    /// Create a new empty PLA
    pub fn new(num_inputs: usize, num_outputs: usize) -> Self {
        Pla {
            num_inputs,
            num_outputs,
            num_products: 0,
            input_labels: (0..num_inputs).map(|i| format!("x{}", i)).collect(),
            output_labels: (0..num_outputs).map(|i| format!("y{}", i)).collect(),
            pla_type: PlaType::F,
            on_set: Cover::new(num_inputs, num_outputs),
            off_set: Cover::new(num_inputs, num_outputs),
            dc_set: Cover::new(num_inputs, num_outputs),
            comments: Vec::new(),
        }
    }

    /// Parse a PLA file from a reader
    pub fn parse<R: Read>(reader: R) -> Result<Self, EspressoError> {
        let buf_reader = BufReader::new(reader);
        let mut num_inputs: Option<usize> = None;
        let mut num_outputs: Option<usize> = None;
        let mut num_products: Option<usize> = None;
        let mut input_labels: Vec<String> = Vec::new();
        let mut output_labels: Vec<String> = Vec::new();
        let mut pla_type = PlaType::F;
        let mut cubes: Vec<(String, String)> = Vec::new();
        let mut comments: Vec<String> = Vec::new();

        for (line_num, line_result) in buf_reader.lines().enumerate() {
            let line = line_result?;
            let line_num = line_num + 1; // 1-based line numbers

            let line = line.trim();

            // Skip empty lines
            if line.is_empty() {
                continue;
            }

            // Handle comments
            if let Some(comment) = line.strip_prefix('#') {
                comments.push(comment.trim().to_string());
                continue;
            }

            // Handle keywords
            if line.starts_with('.') {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.is_empty() {
                    continue;
                }

                match parts[0] {
                    ".i" => {
                        if parts.len() < 2 {
                            return Err(EspressoError::InvalidPla {
                                line: line_num,
                                message: "Missing number of inputs".to_string(),
                            });
                        }
                        num_inputs =
                            Some(parts[1].parse().map_err(|_| EspressoError::InvalidPla {
                                line: line_num,
                                message: "Invalid number of inputs".to_string(),
                            })?);
                    }
                    ".o" => {
                        if parts.len() < 2 {
                            return Err(EspressoError::InvalidPla {
                                line: line_num,
                                message: "Missing number of outputs".to_string(),
                            });
                        }
                        num_outputs =
                            Some(parts[1].parse().map_err(|_| EspressoError::InvalidPla {
                                line: line_num,
                                message: "Invalid number of outputs".to_string(),
                            })?);
                    }
                    ".p" => {
                        if parts.len() < 2 {
                            return Err(EspressoError::InvalidPla {
                                line: line_num,
                                message: "Missing number of products".to_string(),
                            });
                        }
                        num_products =
                            Some(parts[1].parse().map_err(|_| EspressoError::InvalidPla {
                                line: line_num,
                                message: "Invalid number of products".to_string(),
                            })?);
                    }
                    ".ilb" => {
                        input_labels = parts[1..].iter().map(|s| s.to_string()).collect();
                    }
                    ".ob" => {
                        output_labels = parts[1..].iter().map(|s| s.to_string()).collect();
                    }
                    ".type" => {
                        if parts.len() >= 2 {
                            pla_type = match parts[1] {
                                "f" => PlaType::F,
                                "r" => PlaType::R,
                                "d" => PlaType::D,
                                "fd" => PlaType::Fd,
                                "fr" => PlaType::Fr,
                                "dr" => PlaType::Dr,
                                "fdr" => PlaType::Fdr,
                                _ => PlaType::F,
                            };
                        }
                    }
                    ".e" | ".end" => {
                        break; // End of file
                    }
                    ".phase" | ".pair" | ".symbolic" | ".mv" => {
                        // Skip these advanced directives
                    }
                    _ => {
                        // Unknown directive, skip
                    }
                }
                continue;
            }

            // Parse cube line
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            // Remove inline comments
            let input_part = parts[0].split('#').next().unwrap_or("");
            let output_part = if parts.len() > 1 {
                parts[1].split('#').next().unwrap_or("")
            } else {
                ""
            };

            if !input_part.is_empty() {
                cubes.push((input_part.to_string(), output_part.to_string()));
            }
        }

        // Determine dimensions
        let ni = num_inputs.unwrap_or_else(|| cubes.first().map(|(i, _)| i.len()).unwrap_or(0));
        let no = num_outputs.unwrap_or_else(|| {
            cubes
                .first()
                .map(|(_, o)| if o.is_empty() { 1 } else { o.len() })
                .unwrap_or(1)
        });

        // Set default labels if not provided
        if input_labels.is_empty() {
            input_labels = (0..ni).map(|i| format!("x{}", i)).collect();
        }
        if output_labels.is_empty() {
            output_labels = (0..no).map(|i| format!("y{}", i)).collect();
        }

        // Create covers
        let mut on_set = Cover::new(ni, no);
        let mut off_set = Cover::new(ni, no);
        let mut dc_set = Cover::new(ni, no);

        // Parse cubes
        for (input_str, output_str) in cubes {
            let output_str = if output_str.is_empty() {
                "1".repeat(no)
            } else {
                output_str
            };

            // Adjust output string length if needed
            let output_str = if output_str.len() < no {
                format!("{:0<width$}", output_str, width = no)
            } else {
                output_str
            };

            let cube = Cube::parse_line(&format!("{} {}", input_str, output_str), ni, no)?;

            // Add to appropriate set based on pla_type and output values
            match pla_type {
                PlaType::F | PlaType::Fd | PlaType::Fr | PlaType::Fdr => {
                    on_set.add(cube);
                }
                PlaType::R => {
                    off_set.add(cube);
                }
                PlaType::D => {
                    dc_set.add(cube);
                }
                PlaType::Dr => {
                    // Ambiguous, treat as DC
                    dc_set.add(cube);
                }
            }
        }

        Ok(Pla {
            num_inputs: ni,
            num_outputs: no,
            num_products: num_products.unwrap_or(on_set.len()),
            input_labels,
            output_labels,
            pla_type,
            on_set,
            off_set,
            dc_set,
            comments,
        })
    }

    /// Parse PLA from a string
    pub fn parse_str(s: &str) -> Result<Self, EspressoError> {
        Self::parse(s.as_bytes())
    }

    /// Write PLA to a writer
    pub fn write<W: Write>(&self, mut writer: W) -> Result<(), EspressoError> {
        writeln!(writer, ".i {}", self.num_inputs)?;
        writeln!(writer, ".o {}", self.num_outputs)?;

        if !self.input_labels.iter().all(|l| l.starts_with('x')) {
            write!(writer, ".ilb")?;
            for label in &self.input_labels {
                write!(writer, " {}", label)?;
            }
            writeln!(writer)?;
        }

        if !self.output_labels.iter().all(|l| l.starts_with('y')) {
            write!(writer, ".ob")?;
            for label in &self.output_labels {
                write!(writer, " {}", label)?;
            }
            writeln!(writer)?;
        }

        writeln!(writer, ".p {}", self.on_set.len())?;

        for cube in self.on_set.iter() {
            writeln!(writer, "{}", cube)?;
        }

        writeln!(writer, ".e")?;
        Ok(())
    }

    /// Convert to PLA format string
    pub fn format_pla(&self) -> String {
        let mut buf = Vec::new();
        self.write(&mut buf).unwrap();
        String::from_utf8(buf).unwrap()
    }

    /// Write in equation format (eqntott style)
    pub fn write_equations<W: Write>(&self, mut writer: W) -> Result<(), EspressoError> {
        for (out_idx, out_label) in self.output_labels.iter().enumerate() {
            let mut terms: Vec<String> = Vec::new();

            for cube in self.on_set.iter() {
                if cube.output(out_idx) == super::cube::CubeValue::One {
                    let mut literals: Vec<String> = Vec::new();
                    for (in_idx, in_label) in self.input_labels.iter().enumerate() {
                        match cube.input(in_idx) {
                            super::cube::CubeValue::One => {
                                literals.push(in_label.clone());
                            }
                            super::cube::CubeValue::Zero => {
                                literals.push(format!("!{}", in_label));
                            }
                            super::cube::CubeValue::DontCare => {}
                        }
                    }
                    if literals.is_empty() {
                        terms.push("1".to_string()); // Tautology term
                    } else {
                        terms.push(literals.join(" & "));
                    }
                }
            }

            if terms.is_empty() {
                writeln!(writer, "{} = 0;", out_label)?;
            } else {
                writeln!(writer, "{} = {};", out_label, terms.join(" | "))?;
            }
        }
        Ok(())
    }
}

impl std::fmt::Display for Pla {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.format_pla())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_pla() {
        let pla_str = r#"
.i 4
.o 1
.ilb A B C D
.ob F
0000 1
0001 1
001- 1
.e
"#;
        let pla = Pla::parse_str(pla_str).unwrap();
        assert_eq!(pla.num_inputs, 4);
        assert_eq!(pla.num_outputs, 1);
        assert_eq!(pla.on_set.len(), 3);
        assert_eq!(pla.input_labels, vec!["A", "B", "C", "D"]);
        assert_eq!(pla.output_labels, vec!["F"]);
    }

    #[test]
    fn test_parse_multi_output() {
        let pla_str = r#"
.i 2
.o 2
0- 10
-1 01
.e
"#;
        let pla = Pla::parse_str(pla_str).unwrap();
        assert_eq!(pla.num_inputs, 2);
        assert_eq!(pla.num_outputs, 2);
        assert_eq!(pla.on_set.len(), 2);
    }

    #[test]
    fn test_roundtrip() {
        let pla_str = r#".i 3
.o 1
.p 2
10- 1
-11 1
.e
"#;
        let pla = Pla::parse_str(pla_str).unwrap();
        let output = pla.to_string();
        let pla2 = Pla::parse_str(&output).unwrap();
        assert_eq!(pla.num_inputs, pla2.num_inputs);
        assert_eq!(pla.num_outputs, pla2.num_outputs);
        assert_eq!(pla.on_set.len(), pla2.on_set.len());
    }
}
