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

    [Fact]
    public void FormulaSurface_ComparisonConcatLogicalAndMathHelpers_Work()
    {
        var engine = new DvcEngineCore();
        Assert.Equal(DvcStatus.Ok, engine.SetCellNumber(new DvcCellAddr(1, 1), 2));
        Assert.Equal(DvcStatus.Ok, engine.SetCellFormula(new DvcCellAddr(2, 1), "=A1>1"));
        Assert.Equal(DvcStatus.Ok, engine.SetCellFormula(new DvcCellAddr(3, 1), "=\"A\"&\"B\""));
        Assert.Equal(DvcStatus.Ok, engine.SetCellFormula(new DvcCellAddr(4, 1), "=AND(TRUE,A1=2,NOT(FALSE))"));
        Assert.Equal(DvcStatus.Ok, engine.SetCellFormula(new DvcCellAddr(5, 1), "=MIN(3,1,2)"));
        Assert.Equal(DvcStatus.Ok, engine.SetCellFormula(new DvcCellAddr(6, 1), "=MAX(3,1,2)"));
        Assert.Equal(DvcStatus.Ok, engine.SetCellFormula(new DvcCellAddr(7, 1), "=ABS(-3)"));
        Assert.Equal(DvcStatus.Ok, engine.SetCellFormula(new DvcCellAddr(8, 1), "=ROUND(1.26,1)"));

        Assert.Equal(DvcStatus.Ok, engine.GetCellState(new DvcCellAddr(2, 1), out var cmp));
        Assert.Equal(DvcValueType.Bool, cmp.Value.Type);
        Assert.Equal(1, cmp.Value.BoolVal);

        Assert.Equal(DvcStatus.Ok, engine.GetCellState(new DvcCellAddr(3, 1), out var concat));
        Assert.Equal(DvcValueType.Text, concat.Value.Type);
        Assert.Equal(DvcStatus.Ok, engine.GetCellText(new DvcCellAddr(3, 1), out var concatText));
        Assert.Equal("AB", concatText);

        Assert.Equal(DvcStatus.Ok, engine.GetCellState(new DvcCellAddr(4, 1), out var logical));
        Assert.Equal(DvcValueType.Bool, logical.Value.Type);
        Assert.Equal(1, logical.Value.BoolVal);

        Assert.Equal(DvcStatus.Ok, engine.GetCellState(new DvcCellAddr(5, 1), out var min));
        Assert.Equal(1.0, min.Value.Number);
        Assert.Equal(DvcStatus.Ok, engine.GetCellState(new DvcCellAddr(6, 1), out var max));
        Assert.Equal(3.0, max.Value.Number);
        Assert.Equal(DvcStatus.Ok, engine.GetCellState(new DvcCellAddr(7, 1), out var abs));
        Assert.Equal(3.0, abs.Value.Number);
        Assert.Equal(DvcStatus.Ok, engine.GetCellState(new DvcCellAddr(8, 1), out var round));
        Assert.Equal(1.3, round.Value.Number, 6);
    }

    [Fact]
    public void SpillReferenceAndRangeInteraction_Works()
    {
        var engine = new DvcEngineCore();
        Assert.Equal(DvcStatus.Ok, engine.SetCellFormula(new DvcCellAddr(1, 1), "=SEQUENCE(2,2,1,1)"));
        Assert.Equal(DvcStatus.Ok, engine.SetCellFormula(new DvcCellAddr(3, 1), "=SUM(A1#)"));
        Assert.Equal(DvcStatus.Ok, engine.SetCellFormula(new DvcCellAddr(4, 1), "=SUM(A1:B2)"));
        Assert.Equal(DvcStatus.Ok, engine.SetCellFormula(new DvcCellAddr(5, 1), "=A1#"));

        Assert.Equal(DvcStatus.Ok, engine.GetCellState(new DvcCellAddr(3, 1), out var sumSpill));
        Assert.Equal(10.0, sumSpill.Value.Number);
        Assert.Equal(DvcStatus.Ok, engine.GetCellState(new DvcCellAddr(4, 1), out var sumRange));
        Assert.Equal(10.0, sumRange.Value.Number);
        Assert.Equal(DvcStatus.Ok, engine.GetCellState(new DvcCellAddr(5, 1), out var scalarSpill));
        Assert.Equal(1.0, scalarSpill.Value.Number);
    }

    [Fact]
    public void StructuralRewrite_PreservesMixedAbsoluteReferences()
    {
        var engine = new DvcEngineCore();
        Assert.Equal(DvcStatus.Ok, engine.SetCellFormula(new DvcCellAddr(2, 2), "=$A1+A$1+$A$1+A1"));
        Assert.Equal(DvcStatus.Ok, engine.InsertRow(1));
        Assert.Equal(DvcStatus.Ok, engine.GetCellInputText(new DvcCellAddr(2, 3), out var rewritten));
        Assert.Equal("=$A2+A$1+$A$1+A2", rewritten);
    }

    [Fact]
    public void ChangeTracking_ProvidesOldNewPayloadForSpillAndFormat()
    {
        var engine = new DvcEngineCore();
        Assert.Equal(DvcStatus.Ok, engine.ChangeTrackingEnable());
        Assert.Equal(DvcStatus.Ok, engine.SetCellFormula(new DvcCellAddr(1, 1), "=SEQUENCE(2,1,1,1)"));
        Assert.Equal(DvcStatus.Ok, engine.SetCellFormula(new DvcCellAddr(1, 1), "=SEQUENCE(1,1,1,1)"));

        var format = DvcCellFormat.Default;
        format.Bold = 1;
        Assert.Equal(DvcStatus.Ok, engine.SetCellFormat(new DvcCellAddr(2, 1), format));

        Assert.Equal(DvcStatus.Ok, engine.DrainChanges(out var iter));
        var sawSpillWithOld = false;
        var sawFormatChange = false;
        while (iter.Next(out var item))
        {
            if (item.Type == DvcChangeType.SpillRegion && item.HadOldSpill == 1 && item.HasNewSpill == 1)
            {
                sawSpillWithOld = true;
                Assert.Equal((ushort)2, item.OldSpill.End.Row);
                Assert.Equal((ushort)1, item.NewSpill.End.Row);
            }

            if (item.Type == DvcChangeType.CellFormat && item.Cell.Col == 2 && item.Cell.Row == 1)
            {
                sawFormatChange = true;
                Assert.Equal(DvcCellFormat.Default, item.OldFormat);
                Assert.Equal(1, item.NewFormat.Bold);
            }
        }

        Assert.True(sawSpillWithOld);
        Assert.True(sawFormatChange);
    }

    [Fact]
    public void VolatilityPresence_IncludesRegisteredUdfClasses()
    {
        var engine = new DvcEngineCore();
        DvcStatus VolatileUdf(DvcCellValue[] _, out DvcCellValue result)
        {
            result = new DvcCellValue { Type = DvcValueType.Number, Number = 1.0 };
            return DvcStatus.Ok;
        }

        Assert.Equal(DvcStatus.Ok, engine.UdfRegister("VOL", DvcVolatility.Volatile, VolatileUdf));
        Assert.Equal(DvcStatus.Ok, engine.UdfRegister("EXT", DvcVolatility.ExternallyInvalidated, VolatileUdf));
        Assert.Equal(DvcStatus.Ok, engine.SetCellFormula(new DvcCellAddr(1, 1), "=VOL()"));
        Assert.Equal(DvcStatus.Ok, engine.SetCellFormula(new DvcCellAddr(2, 1), "=EXT()"));

        Assert.Equal(DvcStatus.Ok, engine.HasVolatileCells(out var hasVolatile));
        Assert.Equal(DvcStatus.Ok, engine.HasExternallyInvalidatedCells(out var hasExternal));
        Assert.Equal(1, hasVolatile);
        Assert.Equal(1, hasExternal);
    }

    [Fact]
    public void CycleBehavior_NonIterativeRecalculateUsesFallbackSemantics()
    {
        var engine = new DvcEngineCore();
        Assert.Equal(DvcStatus.Ok, engine.SetRecalcMode(DvcRecalcMode.Manual));
        Assert.Equal(DvcStatus.Ok, engine.SetCellFormula(new DvcCellAddr(1, 1), "=B1"));
        Assert.Equal(DvcStatus.Ok, engine.SetCellFormula(new DvcCellAddr(2, 1), "=A1"));
        Assert.Equal(DvcStatus.Ok, engine.Recalculate());

        Assert.Equal(DvcStatus.Ok, engine.GetCellState(new DvcCellAddr(1, 1), out var a1));
        Assert.Equal(DvcStatus.Ok, engine.GetCellState(new DvcCellAddr(2, 1), out var b1));
        Assert.Equal(DvcValueType.Number, a1.Value.Type);
        Assert.Equal(DvcValueType.Number, b1.Value.Type);
        Assert.Equal(0.0, a1.Value.Number, 6);
        Assert.Equal(0.0, b1.Value.Number, 6);
    }

    [Fact]
    public void CycleBehavior_NonIterativeEmitsDiagnosticWithEpochAndPayload()
    {
        var engine = new DvcEngineCore();
        Assert.Equal(DvcStatus.Ok, engine.SetRecalcMode(DvcRecalcMode.Manual));
        Assert.Equal(DvcStatus.Ok, engine.ChangeTrackingEnable());

        Assert.Equal(DvcStatus.Ok, engine.SetCellNumber(new DvcCellAddr(1, 1), 5));
        Assert.Equal(DvcStatus.Ok, engine.Recalculate());
        Assert.Equal(DvcStatus.Ok, engine.SetCellFormula(new DvcCellAddr(1, 1), "=A1+1"));
        Assert.Equal(DvcStatus.Ok, engine.Recalculate());

        Assert.Equal(DvcStatus.Ok, engine.GetCellState(new DvcCellAddr(1, 1), out var state));
        Assert.Equal(DvcValueType.Number, state.Value.Type);
        Assert.Equal(6.0, state.Value.Number, 6);

        Assert.Equal(DvcStatus.Ok, engine.DrainChanges(out var iter));
        var sawDiagnostic = false;
        while (iter.Next(out var item))
        {
            if (item.Type != DvcChangeType.Diagnostic)
            {
                continue;
            }

            sawDiagnostic = true;
            Assert.Equal(engine.CommittedEpoch, item.Epoch);
            Assert.Equal(DvcDiagnosticCode.CircularReferenceDetected, item.DiagnosticCode);
            Assert.Equal("Circular reference detected.", item.DiagnosticMessage);
        }

        Assert.True(sawDiagnostic);
    }

    [Fact]
    public void CycleBehavior_IterationEnabledStillRecalculates()
    {
        var engine = new DvcEngineCore();
        Assert.Equal(DvcStatus.Ok, engine.SetRecalcMode(DvcRecalcMode.Manual));
        Assert.Equal(DvcStatus.Ok, engine.ChangeTrackingEnable());

        var cfg = new DvcIterationConfig
        {
            Enabled = 1,
            MaxIterations = 8,
            ConvergenceTolerance = 0.0001,
        };

        Assert.Equal(DvcStatus.Ok, engine.SetIterationConfig(cfg));
        Assert.Equal(DvcStatus.Ok, engine.SetCellFormula(new DvcCellAddr(1, 1), "=B1"));
        Assert.Equal(DvcStatus.Ok, engine.SetCellFormula(new DvcCellAddr(2, 1), "=A1"));
        Assert.Equal(DvcStatus.Ok, engine.Recalculate());

        Assert.Equal(DvcStatus.Ok, engine.DrainChanges(out var iter));
        while (iter.Next(out var item))
        {
            Assert.NotEqual(DvcChangeType.Diagnostic, item.Type);
        }
    }

    [Fact]
    public void GapEvidence_LetLambdaIndirectAndOffsetRemainUnsupported()
    {
        var engine = new DvcEngineCore();
        Assert.Equal(DvcStatus.Ok, engine.SetCellFormula(new DvcCellAddr(1, 1), "=LET(X,1,X)"));
        Assert.Equal(DvcStatus.Ok, engine.SetCellFormula(new DvcCellAddr(2, 1), "=INDIRECT(\"A1\")"));
        Assert.Equal(DvcStatus.Ok, engine.SetCellFormula(new DvcCellAddr(3, 1), "=OFFSET(A1,1,0)"));
        Assert.Equal(DvcStatus.Ok, engine.Recalculate());

        Assert.Equal(DvcStatus.Ok, engine.GetCellState(new DvcCellAddr(1, 1), out var letState));
        Assert.Equal(DvcValueType.Error, letState.Value.Type);
        Assert.Equal(DvcCellErrorKind.UnknownName, letState.Value.ErrorKind);

        Assert.Equal(DvcStatus.Ok, engine.GetCellState(new DvcCellAddr(2, 1), out var indirectState));
        Assert.Equal(DvcValueType.Error, indirectState.Value.Type);
        Assert.Equal(DvcCellErrorKind.UnknownName, indirectState.Value.ErrorKind);

        Assert.Equal(DvcStatus.Ok, engine.GetCellState(new DvcCellAddr(3, 1), out var offsetState));
        Assert.Equal(DvcValueType.Error, offsetState.Value.Type);
        Assert.Equal(DvcCellErrorKind.UnknownName, offsetState.Value.ErrorKind);
    }
}
