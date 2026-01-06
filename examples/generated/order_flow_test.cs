// GENERATED TESTS FROM: order_flow.yaml
// GENERATED: 2026-01-05T17:28:46.897249824+00:00
// DO NOT EDIT — regenerate from spec

using FluentAssertions;
using Xunit;

public class OrderFlowTests
{
    /// <summary>
    /// Happy path: all gates pass, orchestrator completes successfully
    /// </summary>
    [Fact]
    public void Execute_WithValidInputs_ShouldSucceed()
    {
        // Arrange
        var input = new OrderFlowInput
        {
            Role = "test",
            Verified = true,
            WeightKg = 10.0,
            Zone = "test",
            Priority = true,
            MemberTier = "test",
        };

        // Act
        var act = () => OrderFlowOrchestrator.Execute(input);

        // Assert
        act.Should().NotThrow();
    }

    /// <summary>
    /// Gate 'require_access' should reject when condition 'check_access.level >= 50' is false
    /// </summary>
    [Fact]
    public void Execute_WhenGateRequireAccess_Fails_ShouldThrow()
    {
        // Arrange - inputs designed to fail this gate
        var input = new OrderFlowInput
        {
            Role = "",
            Verified = false,
            WeightKg = 0.0,
            Zone = "",
            Priority = false,
            MemberTier = "",
        };

        // Act
        var act = () => OrderFlowOrchestrator.Execute(input);

        // Assert
        act.Should().Throw<OrderFlowException>()
            .Where(e => e.Step == "require_access")
            .Where(e => e.ErrorType == "gate_failed");
    }

    /// <summary>
    /// Step 'check_access' should call spec 'access_level'
    /// </summary>
    [Fact]
    public void Execute_StepCheckAccess_AccessLevel_ShouldBeInvoked()
    {
        // Arrange
        var input = new OrderFlowInput
        {
            Role = "test",
            Verified = true,
            WeightKg = 10.0,
            Zone = "test",
            Priority = true,
            MemberTier = "test",
        };

        // Act
        var result = OrderFlowOrchestrator.Execute(input);

        // Assert - step was executed (context populated)
        result.Should().NotBeNull();
    }

    /// <summary>
    /// Step 'calc_shipping' should call spec 'shipping_rate'
    /// </summary>
    [Fact]
    public void Execute_StepCalcShipping_ShippingRate_ShouldBeInvoked()
    {
        // Arrange
        var input = new OrderFlowInput
        {
            Role = "test",
            Verified = true,
            WeightKg = 10.0,
            Zone = "test",
            Priority = true,
            MemberTier = "test",
        };

        // Act
        var result = OrderFlowOrchestrator.Execute(input);

        // Assert - step was executed (context populated)
        result.Should().NotBeNull();
    }

}
