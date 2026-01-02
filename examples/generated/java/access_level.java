// GENERATED FROM: access_level.yaml // SPEC HASH: sha256:61f180f99fb26ed2 // GENERATED: 2026-01-02T15:13:56.517833827+00:00

 public class AccessLevel {

 public static class AccessLevelInput { public String role;

public boolean verified; }

 public static long evaluate(AccessLevelInput input) { if ((input.role == "admin")) { // R1

 return 100L; }else if (((input.role == "member") && input.verified)) { // R2

 return 50L; }else if (((input.role == "member") && (!input.verified))) { // R3

 return 25L; }else if ((input.role == "guest")) { // R4

 return 10L; } } }

