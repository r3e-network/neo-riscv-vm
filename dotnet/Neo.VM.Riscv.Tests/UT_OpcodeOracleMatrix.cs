using Neo.Test.Extensions;
using Neo.VM;
using Neo.VM.Types;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;
using System;
using System.Collections.Generic;
using System.Linq;
using System.Numerics;

namespace Neo.Test;

[TestClass]
public class UT_OpcodeOracleMatrix
{
    [TestMethod]
    public void TestHighRiskOpcodeTypeMatrixMatchesNeoVm()
    {
        using var runner = RiscvVmRunner.CreateFromEnvironment();
        var failures = new List<string>();
        var cases = GenerateMatrixCases().ToArray();
        var maxFailures = ReadEnvInt("NEO_RISCV_ORACLE_MATRIX_MAX_FAILURES", 25);
        var strictFaultMessage = string.Equals(
            Environment.GetEnvironmentVariable("NEO_RISCV_ORACLE_MATRIX_STRICT_FAULT_MESSAGE"),
            "1",
            StringComparison.OrdinalIgnoreCase);

        foreach (var testCase in cases)
        {
            var neoVm = RunNeoVm(testCase.Script);
            var riscv = runner.Execute(testCase.Script);
            var failure = CompareOutcomes(testCase.Name, neoVm, riscv, strictFaultMessage);
            if (failure is not null)
            {
                failures.Add(failure);
                if (maxFailures > 0 && failures.Count >= maxFailures)
                    break;
            }
        }

        if (failures.Count > 0)
        {
            Assert.Fail(
                $"Opcode oracle matrix failures: {failures.Count} / {cases.Length} cases." +
                Environment.NewLine +
                string.Join(Environment.NewLine, failures));
        }

        Assert.IsTrue(
            cases.Any(testCase => testCase.Name == "AND(Integer15,ByteString33)"),
            "Matrix must include the GenesMixer-style mixed primitive bitwise regression.");
    }

    private static IEnumerable<MatrixCase> GenerateMatrixCases()
    {
        var operands = new[]
        {
            OperandSpec.Null,
            OperandSpec.False,
            OperandSpec.True,
            OperandSpec.IntegerZero,
            OperandSpec.IntegerFifteen,
            OperandSpec.IntegerMinusOne,
            OperandSpec.ByteString0f,
            OperandSpec.ByteString33,
            OperandSpec.Buffer33,
        };

        var numericish = new[]
        {
            OperandSpec.False,
            OperandSpec.True,
            OperandSpec.IntegerZero,
            OperandSpec.IntegerFifteen,
            OperandSpec.IntegerMinusOne,
            OperandSpec.ByteString0f,
            OperandSpec.ByteString33,
        };

        foreach (var opCode in new[]
                 {
                     OpCode.INVERT, OpCode.NOT, OpCode.NZ, OpCode.INC, OpCode.DEC,
                     OpCode.SIGN, OpCode.ABS, OpCode.NEGATE, OpCode.SIZE,
                 })
        {
            foreach (var operand in operands)
                yield return BuildUnaryCase(opCode, operand);
        }

        foreach (var opCode in new[]
                 {
                     OpCode.AND, OpCode.OR, OpCode.XOR,
                     OpCode.EQUAL, OpCode.NOTEQUAL,
                     OpCode.NUMEQUAL, OpCode.NUMNOTEQUAL,
                     OpCode.LT, OpCode.LE, OpCode.GT, OpCode.GE,
                     OpCode.ADD, OpCode.SUB, OpCode.MUL, OpCode.DIV, OpCode.MOD,
                     OpCode.SHL, OpCode.SHR,
                     OpCode.MIN, OpCode.MAX,
                     OpCode.BOOLAND, OpCode.BOOLOR,
                 })
        {
            foreach (var left in operands)
            foreach (var right in operands)
                yield return BuildBinaryCase(opCode, left, right);
        }

        foreach (var value in operands)
        foreach (var target in new[]
                 {
                     StackItemType.Boolean,
                     StackItemType.Integer,
                     StackItemType.ByteString,
                     StackItemType.Buffer,
                 })
        {
            yield return BuildConvertCase(value, target);
        }

        foreach (var value in operands)
        foreach (var target in new[]
                 {
                     StackItemType.Any,
                     StackItemType.Boolean,
                     StackItemType.Integer,
                     StackItemType.ByteString,
                     StackItemType.Buffer,
                     StackItemType.Array,
                     StackItemType.Map,
                 })
        {
            yield return BuildIsTypeCase(value, target);
        }

        foreach (var value in numericish)
        foreach (var start in numericish)
        foreach (var end in numericish)
            yield return BuildWithinCase(value, start, end);

        foreach (var opCode in new[] { OpCode.MODMUL, OpCode.MODPOW })
        foreach (var left in operands)
        foreach (var right in operands)
        foreach (var modulus in operands)
            yield return BuildTernaryCase(opCode, left, right, modulus);
    }

    private static MatrixCase BuildUnaryCase(OpCode opCode, OperandSpec operand)
    {
        using var sb = new ScriptBuilder();
        operand.Emit(sb);
        sb.Emit(opCode);
        sb.Emit(OpCode.RET);
        return new MatrixCase($"{opCode}({operand.Name})", sb.ToArray());
    }

    private static MatrixCase BuildBinaryCase(OpCode opCode, OperandSpec left, OperandSpec right)
    {
        using var sb = new ScriptBuilder();
        left.Emit(sb);
        right.Emit(sb);
        sb.Emit(opCode);
        sb.Emit(OpCode.RET);
        return new MatrixCase($"{opCode}({left.Name},{right.Name})", sb.ToArray());
    }

    private static MatrixCase BuildTernaryCase(OpCode opCode, OperandSpec first, OperandSpec second, OperandSpec third)
    {
        using var sb = new ScriptBuilder();
        first.Emit(sb);
        second.Emit(sb);
        third.Emit(sb);
        sb.Emit(opCode);
        sb.Emit(OpCode.RET);
        return new MatrixCase($"{opCode}({first.Name},{second.Name},{third.Name})", sb.ToArray());
    }

    private static MatrixCase BuildConvertCase(OperandSpec value, StackItemType target)
    {
        using var sb = new ScriptBuilder();
        value.Emit(sb);
        sb.Emit(OpCode.CONVERT, new[] { (byte)target });
        sb.Emit(OpCode.RET);
        return new MatrixCase($"CONVERT({value.Name}->{target})", sb.ToArray());
    }

    private static MatrixCase BuildIsTypeCase(OperandSpec value, StackItemType target)
    {
        using var sb = new ScriptBuilder();
        value.Emit(sb);
        sb.Emit(OpCode.ISTYPE, new[] { (byte)target });
        sb.Emit(OpCode.RET);
        return new MatrixCase($"ISTYPE({value.Name},{target})", sb.ToArray());
    }

    private static MatrixCase BuildWithinCase(OperandSpec value, OperandSpec start, OperandSpec end)
    {
        using var sb = new ScriptBuilder();
        value.Emit(sb);
        start.Emit(sb);
        end.Emit(sb);
        sb.Emit(OpCode.WITHIN);
        sb.Emit(OpCode.RET);
        return new MatrixCase($"WITHIN({value.Name},{start.Name},{end.Name})", sb.ToArray());
    }

    private static OracleOutcome RunNeoVm(byte[] script)
    {
        var engine = new ExecutionEngine(new JumpTable());
        engine.LoadScript(script);
        var state = engine.Execute();
        return new OracleOutcome(
            state,
            engine.ResultStack.ToArray().Select(NormalizeNeoVmItem).ToArray(),
            engine.UncaughtException?.ToString());
    }

    private static string? CompareOutcomes(
        string name,
        OracleOutcome expected,
        ExecutionOutcome actual,
        bool strictFaultMessage)
    {
        if (expected.State != actual.State)
        {
            return $"{name}: state mismatch NeoVM={expected.State} RISC-V={actual.State} RISC-V fault={actual.FaultMessage ?? "<none>"}";
        }

        if (expected.State == VMState.FAULT)
        {
            if (strictFaultMessage && !string.Equals(expected.FaultMessage, actual.FaultMessage, StringComparison.Ordinal))
            {
                return $"{name}: fault mismatch NeoVM={expected.FaultMessage ?? "<none>"} RISC-V={actual.FaultMessage ?? "<none>"}";
            }

            return null;
        }

        var actualStack = actual.ResultStack.Select(NormalizeRiscvItem).ToArray();
        if (expected.Stack.Count != actualStack.Length)
        {
            return $"{name}: stack length mismatch NeoVM={expected.Stack.Count} RISC-V={actualStack.Length}";
        }

        for (var i = 0; i < expected.Stack.Count; i++)
        {
            var expectedItem = expected.Stack[i].ToString(Formatting.None);
            var actualItem = actualStack[i].ToString(Formatting.None);
            if (!string.Equals(expectedItem, actualItem, StringComparison.Ordinal))
            {
                return $"{name}: stack[{i}] mismatch NeoVM={expectedItem} RISC-V={actualItem}";
            }
        }

        return null;
    }

    private static JObject NormalizeNeoVmItem(StackItem item)
    {
        if (item.IsNull)
        {
            return new JObject { ["type"] = "Null" };
        }

        return item.Type switch
        {
            StackItemType.Boolean => new JObject { ["type"] = "Boolean", ["value"] = item.GetBoolean() },
            StackItemType.Integer => new JObject { ["type"] = "Integer", ["value"] = item.GetInteger().ToString() },
            StackItemType.ByteString => new JObject { ["type"] = "ByteString", ["value"] = ToHex(item.GetSpan().ToArray()) },
            StackItemType.Buffer => new JObject { ["type"] = "Buffer", ["value"] = ToHex(item.GetSpan().ToArray()) },
            _ => new JObject { ["type"] = item.Type.ToString() },
        };
    }

    private static JObject NormalizeRiscvItem(JObject item)
    {
        var clone = (JObject)item.DeepClone();
        clone.Remove("_handle");
        if (clone["value"] is JValue { Type: JTokenType.String } value)
            clone["value"] = NormalizeHexString(value.Value<string>() ?? string.Empty);
        return clone;
    }

    private static string ToHex(byte[] bytes)
    {
        return bytes.Length == 0 ? "0x" : StringExtensions.ToHexString(bytes).ToLowerInvariant();
    }

    private static string NormalizeHexString(string value)
    {
        if (string.IsNullOrEmpty(value))
            return "0x";
        return value.StartsWith("0x", StringComparison.OrdinalIgnoreCase)
            ? value.ToLowerInvariant()
            : value;
    }

    private static int ReadEnvInt(string name, int defaultValue)
    {
        var raw = Environment.GetEnvironmentVariable(name);
        return int.TryParse(raw, out var parsed) ? parsed : defaultValue;
    }

    private sealed record MatrixCase(string Name, byte[] Script);

    private sealed record OracleOutcome(VMState State, IReadOnlyList<JObject> Stack, string? FaultMessage);

    private sealed record OperandSpec(string Name, Action<ScriptBuilder> Emit)
    {
        public static OperandSpec Null { get; } = new("Null", sb => sb.Emit(OpCode.PUSHNULL));

        public static OperandSpec False { get; } = new("BooleanFalse", sb => sb.EmitPush(false));

        public static OperandSpec True { get; } = new("BooleanTrue", sb => sb.EmitPush(true));

        public static OperandSpec IntegerZero { get; } = new("Integer0", sb => sb.EmitPush(BigInteger.Zero));

        public static OperandSpec IntegerFifteen { get; } = new("Integer15", sb => sb.EmitPush(new BigInteger(15)));

        public static OperandSpec IntegerMinusOne { get; } = new("IntegerMinus1", sb => sb.EmitPush(new BigInteger(-1)));

        public static OperandSpec ByteString0f { get; } = new("ByteString0f", sb => sb.EmitPush(new byte[] { 0x0f }));

        public static OperandSpec ByteString33 { get; } = new("ByteString33", sb => sb.EmitPush(new byte[] { 0x33 }));

        public static OperandSpec Buffer33 { get; } = new(
            "Buffer33",
            sb =>
            {
                sb.EmitPush(new byte[] { 0x33 });
                sb.Emit(OpCode.CONVERT, new[] { (byte)StackItemType.Buffer });
            });
    }
}
