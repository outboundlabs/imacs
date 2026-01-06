// GENERATED TESTS FROM: order_flow.yaml
// GENERATED: 2026-01-05T17:28:46.945003542+00:00
// DO NOT EDIT — regenerate from spec

import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;

class OrderFlowTests {

    @Test
    void execute_withValidInputs_shouldSucceed() {
        var input = new OrderFlowOrchestrator.Input(
            "test",
            true,
            10.0,
            "test",
            true,
            "test"
        );

        assertDoesNotThrow(() -> OrderFlowOrchestrator.execute(input));
    }

    @Test
    void execute_whenGateRequireAccess_fails_shouldThrow() {
        var input = new OrderFlowOrchestrator.Input(
            "",
            false,
            0.0,
            "",
            false,
            ""
        );

        var ex = assertThrows(OrderFlowOrchestrator.OrderFlowException.class, () -> {
            OrderFlowOrchestrator.execute(input);
        });
        assertEquals("require_access", ex.step);
        assertEquals("gate_failed", ex.type);
    }

}
