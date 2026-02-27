using Dvc.Core;

namespace Dvc.Core.Tests;

public sealed class CoreEngineTests
{
    [Fact]
    public void SetFormulaAndRecalculate_ComputesValue()
    {
        var engine = new DvcEngineCore();
        Assert.Equal(DvcStatus.Ok, engine.SetCellNumber(new DvcCellAddr(1, 1), 10));
        Assert.Equal(DvcStatus.Ok, engine.SetCellFormula(new DvcCellAddr(2, 1), "=A1*2"));
        Assert.Equal(DvcStatus.Ok, engine.GetCellState(new DvcCellAddr(2, 1), out var state));
        Assert.Equal(DvcValueType.Number, state.Value.Type);
        Assert.Equal(20, state.Value.Number);
    }

    [Fact]
    public void ManualMode_ShowsStaleUntilRecalculate()
    {
        var engine = new DvcEngineCore();
        Assert.Equal(DvcStatus.Ok, engine.SetRecalcMode(DvcRecalcMode.Manual));
        Assert.Equal(DvcStatus.Ok, engine.SetCellNumber(new DvcCellAddr(1, 1), 1));
        Assert.Equal(DvcStatus.Ok, engine.SetCellFormula(new DvcCellAddr(2, 1), "=A1+1"));
        Assert.Equal(DvcStatus.Ok, engine.GetCellState(new DvcCellAddr(2, 1), out var stale));
        Assert.Equal(1, stale.Stale);
        Assert.Equal(DvcStatus.Ok, engine.Recalculate());
        Assert.Equal(DvcStatus.Ok, engine.GetCellState(new DvcCellAddr(2, 1), out var fresh));
        Assert.Equal(2, fresh.Value.Number);
        Assert.Equal(0, fresh.Stale);
    }

    [Fact]
    public void StructuralInsertRow_RewritesA1References()
    {
        var engine = new DvcEngineCore();
        engine.SetCellNumber(new DvcCellAddr(1, 1), 3);
        engine.SetCellFormula(new DvcCellAddr(1, 2), "=A1+1");
        Assert.Equal(DvcStatus.Ok, engine.InsertRow(1));
        Assert.Equal(DvcStatus.Ok, engine.GetCellInputText(new DvcCellAddr(1, 3), out var formula));
        Assert.Equal("=A2+1", formula);
    }

    [Fact]
    public void SequenceSpill_ReportsAnchorAndMember()
    {
        var engine = new DvcEngineCore();
        engine.SetCellFormula(new DvcCellAddr(1, 1), "=SEQUENCE(2,2,1,1)");
        Assert.Equal(DvcStatus.Ok, engine.GetSpillRole(new DvcCellAddr(1, 1), out var anchorRole));
        Assert.Equal(DvcSpillRole.Anchor, anchorRole);
        Assert.Equal(DvcStatus.Ok, engine.GetSpillRole(new DvcCellAddr(2, 1), out var memberRole));
        Assert.Equal(DvcSpillRole.Member, memberRole);
        Assert.Equal(DvcStatus.Ok, engine.GetSpillAnchor(new DvcCellAddr(2, 1), out var anchor, out var found));
        Assert.Equal(1, found);
        Assert.Equal((ushort)1, anchor.Col);
        Assert.Equal((ushort)1, anchor.Row);
    }

    [Fact]
    public void IterationConfig_ValidatesInputs()
    {
        var engine = new DvcEngineCore();
        var bad = new DvcIterationConfig { Enabled = 1, MaxIterations = 0, ConvergenceTolerance = 0.1 };
        Assert.Equal(DvcStatus.ErrInvalidArgument, engine.SetIterationConfig(bad));
        var ok = new DvcIterationConfig { Enabled = 1, MaxIterations = 10, ConvergenceTolerance = 0.01 };
        Assert.Equal(DvcStatus.Ok, engine.SetIterationConfig(ok));
    }
}
