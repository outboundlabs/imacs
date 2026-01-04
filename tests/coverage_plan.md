# Test Coverage Plan

## Public API Coverage Checklist

### Core Completeness Analysis
- [x] `analyze_completeness()` - Basic tests exist
- [ ] `analyze_completeness()` - Edge cases (empty, single rule, no predicates)
- [ ] `IncompletenessReport.to_report()` - All output formats

### Predicate Extraction
- [x] `extract_predicates()` - Basic types
- [ ] `extract_predicates()` - Complex CEL (OR, ternary, nested)
- [ ] `Predicate.to_cel_string()` - Round-trip correctness
- [ ] `Predicate.negated()` - All predicate types
- [ ] `PredicateSet` - Add/get/index operations

### Adapter (CEL ↔ Cubes)
- [x] `expression_to_cube()` - Basic
- [ ] `expression_to_cube()` - Complex expressions
- [ ] `cube_to_cel()` - All cube value combinations
- [ ] `rules_to_cover()` - Multiple rules
- [ ] `minimize_rules()` - End-to-end minimization

### Suite Analysis
- [x] `analyze_suite()` - Basic
- [ ] `analyze_suite()` - Empty suite, single spec
- [ ] `analyze_suite()` - Full vs incremental modes

### Collision Detection
- [x] `detect_collisions()` - Basic
- [ ] `detect_collisions()` - All collision types
- [ ] `detect_collisions()` - No collisions case

### Duplicate Detection
- [x] `detect_duplicates()` - Basic
- [ ] `detect_duplicates()` - Partial overlaps
- [ ] `detect_duplicates()` - No duplicates case

### Relationship Detection
- [x] `detect_relationships()` - Chain, merge
- [ ] `detect_relationships()` - No relationships
- [ ] `detect_relationships()` - Multiple chains

### Variable Matching
- [x] `match_variables()` - Basic
- [ ] `match_variables()` - Similarity scoring edge cases
- [ ] `match_variables()` - No matches

### Suggestions
- [x] `generate_suggestions()` - Basic
- [ ] `generate_suggestions()` - All suggestion types
- [ ] `generate_suggestions()` - Empty inputs

### Orchestrator Suite
- [x] `analyze_orchestrator_suite()` - Basic
- [ ] `analyze_orchestrator_suite()` - Missing specs
- [ ] `analyze_orchestrator_suite()` - Mapping issues
- [ ] `analyze_directory_with_orchestrators()` - Full directory scan

### Espresso Minimization
- [ ] `espresso()` - Known minimization cases
- [ ] `espresso()` - No minimization possible
- [ ] `Cover` - All operations
- [ ] `Cube` - All operations

### Refactoring APIs
- [ ] `minimize()` - All transformation types
- [ ] `decompose()` - Independent groups
- [ ] `compose()` - Chain composition
- [ ] `extract_spec_from_orchestrator()` - Branch/gate extraction

