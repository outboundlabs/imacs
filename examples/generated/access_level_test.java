// GENERATED TESTS FROM: access_level.yaml
// SPEC HASH: sha256:61f180f99fb26ed2
// GENERATED: 2026-01-05T17:28:46.798278230+00:00
// DO NOT EDIT — regenerate from spec

import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;

public class AccessLevelTest {
    @Test
    public void testR1() {
        // R1: role == 'admin' → 100
        var input = new AccessLevel.Input("admin", false);
        assertEquals(100L, AccessLevel.evaluate(input));
    }

    @Test
    public void testR2() {
        // R2: role == 'member' && verified → 50
        var input = new AccessLevel.Input("member", true);
        assertEquals(50L, AccessLevel.evaluate(input));
    }

    @Test
    public void testR3() {
        // R3: role == 'member' && !verified → 25
        var input = new AccessLevel.Input("member", false);
        assertEquals(25L, AccessLevel.evaluate(input));
    }

    @Test
    public void testR4() {
        // R4: role == 'guest' → 10
        var input = new AccessLevel.Input("guest", false);
        assertEquals(10L, AccessLevel.evaluate(input));
    }

}
