// GENERATED TESTS FROM: access_level.yaml
// SPEC HASH: sha256:61f180f99fb26ed2
// GENERATED: 2026-01-05T17:28:46.700239976+00:00
// DO NOT EDIT — regenerate from spec

using Xunit;

public class AccessLevelTests
{
    [Fact]
    public void Test_R1()
    {
        // R1: role == 'admin' → 100
        Assert.Equal(100L, AccessLevel.Evaluate(new AccessLevelInput { Role = "admin", Verified = false }));
    }

    [Fact]
    public void Test_R2()
    {
        // R2: role == 'member' && verified → 50
        Assert.Equal(50L, AccessLevel.Evaluate(new AccessLevelInput { Role = "member", Verified = true }));
    }

    [Fact]
    public void Test_R3()
    {
        // R3: role == 'member' && !verified → 25
        Assert.Equal(25L, AccessLevel.Evaluate(new AccessLevelInput { Role = "member", Verified = false }));
    }

    [Fact]
    public void Test_R4()
    {
        // R4: role == 'guest' → 10
        Assert.Equal(10L, AccessLevel.Evaluate(new AccessLevelInput { Role = "guest", Verified = false }));
    }

}
