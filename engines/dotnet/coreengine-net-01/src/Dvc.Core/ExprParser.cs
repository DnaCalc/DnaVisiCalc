using System.Globalization;

namespace Dvc.Core;

internal sealed class ExprParser
{
    private readonly string _text;
    private readonly Func<CellAddressToken, EvalValue> _cellResolver;
    private readonly Func<string, EvalValue> _nameResolver;
    private int _pos;

    public ExprParser(
        string formula,
        Func<CellAddressToken, EvalValue> cellResolver,
        Func<string, EvalValue> nameResolver)
    {
        _text = formula.Trim();
        if (_text.StartsWith('='))
        {
            _text = _text[1..];
        }

        _cellResolver = cellResolver;
        _nameResolver = nameResolver;
    }

    public bool TryParse(out DvcEngineCore.ExprNode root)
    {
        if (!TryParseExpr(out root))
        {
            return false;
        }

        SkipWs();
        return _pos == _text.Length;
    }

    private bool TryParseExpr(out DvcEngineCore.ExprNode node) => TryParseAddSub(out node);

    private bool TryParseAddSub(out DvcEngineCore.ExprNode node)
    {
        if (!TryParseMulDiv(out node))
        {
            return false;
        }

        while (true)
        {
            SkipWs();
            if (!TryConsume("+") && !TryConsume("-"))
            {
                return true;
            }

            var op = _text[_pos - 1].ToString(CultureInfo.InvariantCulture);
            if (!TryParseMulDiv(out var rhs))
            {
                return false;
            }

            node = new DvcEngineCore.ExprNode(DvcEngineCore.ExprKind.Binary)
            {
                Operator = op,
                Arguments = [node, rhs],
            };
        }
    }

    private bool TryParseMulDiv(out DvcEngineCore.ExprNode node)
    {
        if (!TryParseUnary(out node))
        {
            return false;
        }

        while (true)
        {
            SkipWs();
            if (!TryConsume("*") && !TryConsume("/"))
            {
                return true;
            }

            var op = _text[_pos - 1].ToString(CultureInfo.InvariantCulture);
            if (!TryParseUnary(out var rhs))
            {
                return false;
            }

            node = new DvcEngineCore.ExprNode(DvcEngineCore.ExprKind.Binary)
            {
                Operator = op,
                Arguments = [node, rhs],
            };
        }
    }

    private bool TryParseUnary(out DvcEngineCore.ExprNode node)
    {
        SkipWs();
        if (TryConsume("+"))
        {
            return TryParseUnary(out node);
        }

        if (TryConsume("-"))
        {
            if (!TryParseUnary(out var inner))
            {
                node = default!;
                return false;
            }

            node = new DvcEngineCore.ExprNode(DvcEngineCore.ExprKind.Unary)
            {
                Operator = "-",
                Arguments = [inner],
            };
            return true;
        }

        return TryParsePrimary(out node);
    }

    private bool TryParsePrimary(out DvcEngineCore.ExprNode node)
    {
        SkipWs();
        if (TryConsume("("))
        {
            if (!TryParseExpr(out node))
            {
                return false;
            }

            return TryConsume(")");
        }

        if (TryParseString(out node))
        {
            return true;
        }

        if (TryParseNumber(out node))
        {
            return true;
        }

        if (!TryParseIdentifier(out var ident))
        {
            node = default!;
            return false;
        }

        SkipWs();
        if (TryConsume("("))
        {
            var args = new List<DvcEngineCore.ExprNode>();
            SkipWs();
            if (!TryConsume(")"))
            {
                while (true)
                {
                    if (!TryParseExpr(out var arg))
                    {
                        node = default!;
                        return false;
                    }

                    args.Add(arg);
                    SkipWs();
                    if (TryConsume(")"))
                    {
                        break;
                    }

                    if (!TryConsume(","))
                    {
                        node = default!;
                        return false;
                    }
                }
            }

            node = new DvcEngineCore.ExprNode(DvcEngineCore.ExprKind.Function)
            {
                Name = ident.ToUpperInvariant(),
                Arguments = args,
            };
            return true;
        }

        if (A1Ref.TryParseCellRef(ident, out var cell))
        {
            SkipWs();
            if (TryConsume(":"))
            {
                if (!TryParseIdentifier(out var rhsId) || !A1Ref.TryParseCellRef(rhsId, out var rhsCell))
                {
                    node = default!;
                    return false;
                }

                node = new DvcEngineCore.ExprNode(DvcEngineCore.ExprKind.Range)
                {
                    RangeStart = cell,
                    RangeEnd = rhsCell,
                };
                return true;
            }

            node = new DvcEngineCore.ExprNode(DvcEngineCore.ExprKind.Cell)
            {
                CellToken = cell,
                CellResolver = _cellResolver,
            };
            return true;
        }

        node = new DvcEngineCore.ExprNode(DvcEngineCore.ExprKind.Name)
        {
            Name = ident.ToUpperInvariant(),
            NameResolver = _nameResolver,
        };
        return true;
    }

    private bool TryParseNumber(out DvcEngineCore.ExprNode node)
    {
        node = default!;
        SkipWs();
        var start = _pos;
        var hasDigit = false;
        while (_pos < _text.Length && char.IsDigit(_text[_pos]))
        {
            hasDigit = true;
            _pos++;
        }

        if (_pos < _text.Length && _text[_pos] == '.')
        {
            _pos++;
            while (_pos < _text.Length && char.IsDigit(_text[_pos]))
            {
                hasDigit = true;
                _pos++;
            }
        }

        if (!hasDigit)
        {
            _pos = start;
            return false;
        }

        if (!double.TryParse(_text[start.._pos], NumberStyles.Float, CultureInfo.InvariantCulture, out var value))
        {
            return false;
        }

        node = new DvcEngineCore.ExprNode(DvcEngineCore.ExprKind.Number)
        {
            Number = value,
        };
        return true;
    }

    private bool TryParseString(out DvcEngineCore.ExprNode node)
    {
        node = default!;
        SkipWs();
        if (_pos >= _text.Length || _text[_pos] != '"')
        {
            return false;
        }

        _pos++;
        var start = _pos;
        while (_pos < _text.Length && _text[_pos] != '"')
        {
            _pos++;
        }

        if (_pos >= _text.Length)
        {
            return false;
        }

        var text = _text[start.._pos];
        _pos++;
        node = new DvcEngineCore.ExprNode(DvcEngineCore.ExprKind.Text)
        {
            Text = text,
        };
        return true;
    }

    private bool TryParseIdentifier(out string ident)
    {
        ident = string.Empty;
        SkipWs();
        if (_pos >= _text.Length)
        {
            return false;
        }

        var start = _pos;
        if (_text[_pos] == '$')
        {
            _pos++;
        }

        while (_pos < _text.Length && (char.IsLetterOrDigit(_text[_pos]) || _text[_pos] is '_' or '$'))
        {
            _pos++;
        }

        if (_pos == start)
        {
            return false;
        }

        ident = _text[start.._pos];
        return true;
    }

    private void SkipWs()
    {
        while (_pos < _text.Length && char.IsWhiteSpace(_text[_pos]))
        {
            _pos++;
        }
    }

    private bool TryConsume(string token)
    {
        SkipWs();
        if (_pos + token.Length > _text.Length)
        {
            return false;
        }

        if (!MemoryExtensions.Equals(_text.AsSpan(_pos, token.Length), token.AsSpan(), StringComparison.Ordinal))
        {
            return false;
        }

        _pos += token.Length;
        return true;
    }
}
