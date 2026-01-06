


// GENERATED FROM: access_level.yaml
// SPEC HASH: sha256:61f180f99fb26ed2
// GENERATED: 2026-01-05T17:28:46.798176324+00:00
// DO NOT EDIT - regenerate from spec


import java.util.*;

public class AccessLevel {

    public static class Input {

        public String role;

        public boolean verified;


        public Input(String role, boolean verified) {

            this.role = role;

            this.verified = verified;

        }
    }


    public static long evaluate(Input input) {


        if ((input.role == "admin")) {

            // R1
            return 100L;


        } else if (((input.role == "member") && input.verified)) {

            // R2
            return 50L;


        } else if (((input.role == "member") && (!input.verified))) {

            // R3
            return 25L;


        } else if ((input.role == "guest")) {

            // R4
            return 10L;

        } else {

            throw new IllegalStateException("No rule matched");

        }
    }
}