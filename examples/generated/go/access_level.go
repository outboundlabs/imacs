// GENERATED FROM: access_level.yaml // SPEC HASH: sha256:61f180f99fb26ed2 // GENERATED: 2026-01-02T15:13:55.453265841+00:00

 package access_level

 type AccessLevelInput struct { Role string

Verified bool }

 func AccessLevel(input AccessLevelInput) int64 { if (input.Role == "admin") { // R1

 return 100 }else if ((input.Role == "member") && input.Verified) { // R2

 return 50 }else if ((input.Role == "member") && (!input.Verified)) { // R3

 return 25 }else if (input.Role == "guest") { // R4

 return 10 } }

