# JSON Schema Files

This directory contains JSON Schema files for IMACS output types.

## Generating Schemas

To regenerate these schemas, run:

```bash
mkdir -p schemas
imacs schema spec > schemas/spec.schema.json
imacs schema verify > schemas/verify.schema.json
imacs schema analyze > schemas/analyze.schema.json
imacs schema extract > schemas/extract.schema.json
imacs schema drift > schemas/drift.schema.json
imacs schema completeness > schemas/completeness.schema.json
```

Or use the helper script:

```bash
./scripts/generate_schemas.sh
```

## Schema Files

- `spec.schema.json` - Input spec format (YAML)
- `verify.schema.json` - VerificationResult output
- `analyze.schema.json` - AnalysisReport output
- `extract.schema.json` - ExtractedSpec output
- `drift.schema.json` - DriftReport output
- `completeness.schema.json` - IncompletenessReport output

These schemas are included in the published crate and can be used by LLM tools and other integrations to understand the JSON output format.

