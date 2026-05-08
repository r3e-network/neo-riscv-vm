using Neo.Test.Extensions;
using Neo.VM;
using Neo.VM.Types;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;
using System;
using System.Collections.Generic;
using System.Linq;
using System.Numerics;
using Array = System.Array;

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

    [TestMethod]
    public void TestOracleMatrixCoversEveryNeoVmOpcodeAndStackItemType()
    {
        var caseNames = GenerateMatrixCases().Select(testCase => testCase.Name).ToArray();
        var missingOpcodes = Enum.GetValues<OpCode>()
            .Where(opCode => !caseNames.Any(name => name.StartsWith($"{opCode}(", StringComparison.Ordinal) ||
                name.StartsWith($"{opCode}:", StringComparison.Ordinal)))
            .Select(opCode => opCode.ToString())
            .ToArray();

        Assert.IsFalse(missingOpcodes.Any(), $"Missing oracle matrix opcode coverage: {string.Join(", ", missingOpcodes)}");

        foreach (var requiredType in new[]
                 {
                     "Null", "BooleanFalse", "BooleanTrue", "Integer0", "IntegerMaxI128",
                     "IntegerMinI128", "ByteStringEmpty", "ByteStringSignBit", "ByteString33",
                     "BufferEmpty", "Buffer33", "ArrayEmpty", "StructEmpty", "MapEmpty",
                 })
        {
            Assert.IsTrue(
                caseNames.Any(name => name.Contains(requiredType, StringComparison.Ordinal)),
                $"Missing oracle matrix StackItem/boundary operand coverage: {requiredType}");
        }

        Assert.IsTrue(
            caseNames.Any(name => name.StartsWith("InvalidEncoding(PUSHDATA1-truncated)", StringComparison.Ordinal)),
            "Matrix must include malformed PUSHDATA1 coverage.");
        Assert.IsTrue(
            caseNames.Any(name => name.StartsWith("InvalidEncoding(CONVERT-invalid-type)", StringComparison.Ordinal)),
            "Matrix must include invalid StackItemType operand coverage.");
    }

    [TestMethod]
    public void TestDeterministicDifferentialFuzzMatchesNeoVm()
    {
        using var runner = RiscvVmRunner.CreateFromEnvironment();
        var seed = ReadEnvInt("NEO_RISCV_DIFFERENTIAL_FUZZ_SEED", 0x4e454f);
        var count = ReadEnvInt("NEO_RISCV_DIFFERENTIAL_FUZZ_CASES", 128);
        var cases = GenerateDifferentialFuzzCases(seed, count).ToArray();
        var failures = new List<string>();

        Assert.HasCount(count, cases, "Differential fuzz generator must produce the requested deterministic case count.");

        foreach (var testCase in cases)
        {
            var neoVm = RunNeoVm(testCase.Script);
            var riscv = runner.Execute(testCase.Script);
            var failure = CompareOutcomes(testCase.Name, neoVm, riscv, strictFaultMessage: false);
            if (failure is not null)
                failures.Add(failure);
        }

        Assert.IsFalse(failures.Any(), string.Join(Environment.NewLine, failures.Take(50)));
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
            OperandSpec.IntegerMaxI128,
            OperandSpec.IntegerMinI128,
            OperandSpec.ByteStringEmpty,
            OperandSpec.ByteString0f,
            OperandSpec.ByteStringSignBit,
            OperandSpec.ByteString33,
            OperandSpec.BufferEmpty,
            OperandSpec.Buffer33,
            OperandSpec.ArrayEmpty,
            OperandSpec.StructEmpty,
            OperandSpec.MapEmpty,
        };

        var numericish = new[]
        {
            OperandSpec.False,
            OperandSpec.True,
            OperandSpec.IntegerZero,
            OperandSpec.IntegerFifteen,
            OperandSpec.IntegerMinusOne,
            OperandSpec.IntegerMaxI128,
            OperandSpec.IntegerMinI128,
            OperandSpec.ByteStringEmpty,
            OperandSpec.ByteString0f,
            OperandSpec.ByteStringSignBit,
            OperandSpec.ByteString33,
        };

        foreach (var opCode in new[]
                 {
                     OpCode.INVERT, OpCode.NOT, OpCode.NZ, OpCode.INC, OpCode.DEC,
                     OpCode.SIGN, OpCode.ABS, OpCode.NEGATE, OpCode.SQRT, OpCode.SIZE,
                     OpCode.ISNULL,
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
                     OpCode.POW,
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

        foreach (var testCase in BuildInvalidEncodingCases())
            yield return testCase;

        foreach (var testCase in BuildAllOpcodeSmokeCases())
            yield return testCase;
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

    private static IEnumerable<MatrixCase> BuildInvalidEncodingCases()
    {
        yield return new MatrixCase("InvalidEncoding(PUSHDATA1-truncated)", [(byte)OpCode.PUSHDATA1, 0x02, 0x01]);
        yield return new MatrixCase("InvalidEncoding(PUSHDATA2-truncated-length)", [(byte)OpCode.PUSHDATA2, 0x01]);
        yield return new MatrixCase("InvalidEncoding(PUSHDATA4-truncated-payload)", [(byte)OpCode.PUSHDATA4, 0x04, 0x00, 0x00, 0x00, 0x01]);
        yield return new MatrixCase("InvalidEncoding(PUSHINT16-truncated)", [(byte)OpCode.PUSHINT16, 0x01]);
        yield return new MatrixCase("InvalidEncoding(JMP-missing-offset)", [(byte)OpCode.JMP]);
        yield return new MatrixCase("InvalidEncoding(CALLT-missing-token)", [(byte)OpCode.CALLT, 0x00]);
        yield return new MatrixCase("InvalidEncoding(SYSCALL-missing-hash)", [(byte)OpCode.SYSCALL, 0x01, 0x02]);
        yield return new MatrixCase("InvalidEncoding(CONVERT-invalid-type)", [(byte)OpCode.PUSH0, (byte)OpCode.CONVERT, 0xff, (byte)OpCode.RET]);
        yield return new MatrixCase("InvalidEncoding(ISTYPE-invalid-type)", [(byte)OpCode.PUSH0, (byte)OpCode.ISTYPE, 0xff, (byte)OpCode.RET]);
    }

    private static IEnumerable<MatrixCase> BuildAllOpcodeSmokeCases()
    {
        foreach (var opCode in Enum.GetValues<OpCode>())
            yield return BuildOpcodeSmokeCase(opCode);
    }

    private static MatrixCase BuildOpcodeSmokeCase(OpCode opCode)
    {
        var script = opCode switch
        {
            OpCode.PUSHINT8 => WithRet((byte)opCode, 0x7f),
            OpCode.PUSHINT16 => WithRet([(byte)opCode], I16(0x1234)),
            OpCode.PUSHINT32 => WithRet([(byte)opCode], I32(0x12345678)),
            OpCode.PUSHINT64 => WithRet([(byte)opCode], BitConverter.GetBytes(0x0102030405060708L)),
            OpCode.PUSHINT128 => WithRet([(byte)opCode], Enumerable.Repeat((byte)0x11, 16).ToArray()),
            OpCode.PUSHINT256 => WithRet([(byte)opCode], Enumerable.Repeat((byte)0x22, 32).ToArray()),
            OpCode.PUSHA => WithRet((byte)opCode, 0x04, 0x00, 0x00, 0x00),
            OpCode.PUSHDATA1 => WithRet((byte)opCode, 0x01, 0x42),
            OpCode.PUSHDATA2 => WithRet([(byte)opCode], I16(1), [0x42]),
            OpCode.PUSHDATA4 => WithRet([(byte)opCode], I32(1), [0x42]),

            OpCode.JMP => [(byte)OpCode.JMP, 0x03, (byte)OpCode.PUSH0, (byte)OpCode.RET],
            OpCode.JMP_L => Join([(byte)OpCode.JMP_L], I32(6), [(byte)OpCode.PUSH0, (byte)OpCode.RET]),
            OpCode.JMPIF => [(byte)OpCode.PUSH1, (byte)OpCode.JMPIF, 0x03, (byte)OpCode.PUSH0, (byte)OpCode.RET],
            OpCode.JMPIFNOT => [(byte)OpCode.PUSH0, (byte)OpCode.JMPIFNOT, 0x03, (byte)OpCode.PUSH1, (byte)OpCode.RET],
            OpCode.JMPIF_L => Join([(byte)OpCode.PUSH1, (byte)OpCode.JMPIF_L], I32(6), [(byte)OpCode.PUSH0, (byte)OpCode.RET]),
            OpCode.JMPIFNOT_L => Join([(byte)OpCode.PUSH0, (byte)OpCode.JMPIFNOT_L], I32(6), [(byte)OpCode.PUSH1, (byte)OpCode.RET]),
            OpCode.JMPEQ => JumpCompareCase(OpCode.JMPEQ, longForm: false, left: OpCode.PUSH1, right: OpCode.PUSH1),
            OpCode.JMPNE => JumpCompareCase(OpCode.JMPNE, longForm: false, left: OpCode.PUSH1, right: OpCode.PUSH2),
            OpCode.JMPGT => JumpCompareCase(OpCode.JMPGT, longForm: false, left: OpCode.PUSH2, right: OpCode.PUSH1),
            OpCode.JMPGE => JumpCompareCase(OpCode.JMPGE, longForm: false, left: OpCode.PUSH2, right: OpCode.PUSH1),
            OpCode.JMPLT => JumpCompareCase(OpCode.JMPLT, longForm: false, left: OpCode.PUSH1, right: OpCode.PUSH2),
            OpCode.JMPLE => JumpCompareCase(OpCode.JMPLE, longForm: false, left: OpCode.PUSH1, right: OpCode.PUSH2),
            OpCode.JMPEQ_L => JumpCompareCase(OpCode.JMPEQ_L, longForm: true, left: OpCode.PUSH1, right: OpCode.PUSH1),
            OpCode.JMPNE_L => JumpCompareCase(OpCode.JMPNE_L, longForm: true, left: OpCode.PUSH1, right: OpCode.PUSH2),
            OpCode.JMPGT_L => JumpCompareCase(OpCode.JMPGT_L, longForm: true, left: OpCode.PUSH2, right: OpCode.PUSH1),
            OpCode.JMPGE_L => JumpCompareCase(OpCode.JMPGE_L, longForm: true, left: OpCode.PUSH2, right: OpCode.PUSH1),
            OpCode.JMPLT_L => JumpCompareCase(OpCode.JMPLT_L, longForm: true, left: OpCode.PUSH1, right: OpCode.PUSH2),
            OpCode.JMPLE_L => JumpCompareCase(OpCode.JMPLE_L, longForm: true, left: OpCode.PUSH1, right: OpCode.PUSH2),
            OpCode.CALL => [(byte)OpCode.CALL, 0x04, (byte)OpCode.PUSH0, (byte)OpCode.RET, (byte)OpCode.PUSH1, (byte)OpCode.RET],
            OpCode.CALL_L => Join([(byte)OpCode.CALL_L], I32(7), [(byte)OpCode.PUSH0, (byte)OpCode.RET, (byte)OpCode.PUSH1, (byte)OpCode.RET]),
            OpCode.CALLT => WithRet((byte)OpCode.CALLT, 0x00, 0x00),
            OpCode.TRY => TryNormalShortCase(),
            OpCode.TRY_L => TryNormalLongCase(),
            OpCode.ENDTRY => TryNormalShortCase(),
            OpCode.ENDTRY_L => EndTryLongCase(),
            OpCode.SYSCALL => WithRet([(byte)OpCode.SYSCALL], I32(0)),

            OpCode.INITSSLOT => WithRet((byte)OpCode.INITSSLOT, 0x01),
            OpCode.INITSLOT => WithRet((byte)OpCode.INITSLOT, 0x01, 0x00),
            OpCode.LDSFLD => WithRet((byte)OpCode.LDSFLD, 0x00),
            OpCode.STSFLD => WithRet((byte)OpCode.STSFLD, 0x00),
            OpCode.LDLOC => WithRet((byte)OpCode.LDLOC, 0x00),
            OpCode.STLOC => WithRet((byte)OpCode.STLOC, 0x00),
            OpCode.LDARG => WithRet((byte)OpCode.LDARG, 0x00),
            OpCode.STARG => WithRet((byte)OpCode.STARG, 0x00),

            OpCode.NEWBUFFER => BuildUnaryCase(OpCode.NEWBUFFER, OperandSpec.IntegerOne).Script,
            OpCode.CAT => BuildBinaryCase(OpCode.CAT, OperandSpec.ByteString0f, OperandSpec.ByteString33).Script,
            OpCode.SUBSTR => SpliceCase(OpCode.SUBSTR),
            OpCode.LEFT => BuildBinaryCase(OpCode.LEFT, OperandSpec.ByteString33, OperandSpec.IntegerOne).Script,
            OpCode.RIGHT => BuildBinaryCase(OpCode.RIGHT, OperandSpec.ByteString33, OperandSpec.IntegerOne).Script,

            OpCode.POW => BuildBinaryCase(OpCode.POW, OperandSpec.IntegerFifteen, OperandSpec.IntegerOne).Script,
            OpCode.SQRT => BuildUnaryCase(OpCode.SQRT, OperandSpec.IntegerFifteen).Script,

            OpCode.PACKMAP => UnderflowCase(OpCode.PACKMAP),
            OpCode.PACKSTRUCT => PackCase(OpCode.PACKSTRUCT),
            OpCode.PACK => PackCase(OpCode.PACK),
            OpCode.UNPACK => UnpackArrayCase(),
            OpCode.NEWARRAY => BuildUnaryCase(OpCode.NEWARRAY, OperandSpec.IntegerOne).Script,
            OpCode.NEWARRAY_T => WithRet((byte)OpCode.PUSH1, (byte)OpCode.NEWARRAY_T, (byte)StackItemType.Integer),
            OpCode.NEWSTRUCT => BuildUnaryCase(OpCode.NEWSTRUCT, OperandSpec.IntegerOne).Script,
            OpCode.HASKEY => UnderflowCase(OpCode.HASKEY),
            OpCode.KEYS => UnderflowCase(OpCode.KEYS),
            OpCode.VALUES => UnderflowCase(OpCode.VALUES),
            OpCode.PICKITEM => UnderflowCase(OpCode.PICKITEM),
            OpCode.APPEND => UnderflowCase(OpCode.APPEND),
            OpCode.SETITEM => UnderflowCase(OpCode.SETITEM),
            OpCode.REMOVE => UnderflowCase(OpCode.REMOVE),
            OpCode.POPITEM => UnderflowCase(OpCode.POPITEM),

            OpCode.ISTYPE => BuildIsTypeCase(OperandSpec.IntegerZero, StackItemType.Integer).Script,
            OpCode.CONVERT => BuildConvertCase(OperandSpec.IntegerZero, StackItemType.ByteString).Script,

            OpCode.ABORTMSG => WithRet((byte)OpCode.PUSHDATA1, 0x04, (byte)'b', (byte)'o', (byte)'o', (byte)'m', (byte)OpCode.ABORTMSG),
            OpCode.ASSERTMSG => WithRet((byte)OpCode.PUSH1, (byte)OpCode.PUSHDATA1, 0x02, (byte)'o', (byte)'k', (byte)OpCode.ASSERTMSG),

            _ => IsSmallIntegerPush(opCode) ? WithRet((byte)opCode) : WithRet((byte)opCode),
        };
        return new MatrixCase($"{opCode}:smoke", script);
    }

    private static byte[] UnderflowCase(OpCode opCode) => WithRet((byte)opCode);

    private static bool IsSmallIntegerPush(OpCode opCode) => opCode >= OpCode.PUSHM1 && opCode <= OpCode.PUSH16;

    private static byte[] JumpCompareCase(OpCode opCode, bool longForm, OpCode left, OpCode right)
    {
        return longForm
            ? Join([(byte)left, (byte)right, (byte)opCode], I32(6), [(byte)OpCode.PUSH0, (byte)OpCode.RET])
            : [(byte)left, (byte)right, (byte)opCode, 0x03, (byte)OpCode.PUSH0, (byte)OpCode.RET];
    }

    private static byte[] SpliceCase(OpCode opCode)
    {
        using var sb = new ScriptBuilder();
        OperandSpec.ByteString33.Emit(sb);
        OperandSpec.IntegerZero.Emit(sb);
        OperandSpec.IntegerOne.Emit(sb);
        sb.Emit(opCode);
        sb.Emit(OpCode.RET);
        return sb.ToArray();
    }

    private static byte[] PackCase(OpCode opCode)
    {
        using var sb = new ScriptBuilder();
        OperandSpec.IntegerFifteen.Emit(sb);
        OperandSpec.IntegerOne.Emit(sb);
        sb.Emit(opCode);
        sb.Emit(OpCode.RET);
        return sb.ToArray();
    }

    private static byte[] UnpackArrayCase()
    {
        using var sb = new ScriptBuilder();
        OperandSpec.IntegerFifteen.Emit(sb);
        OperandSpec.IntegerOne.Emit(sb);
        sb.Emit(OpCode.PACK);
        sb.Emit(OpCode.UNPACK);
        sb.Emit(OpCode.RET);
        return sb.ToArray();
    }

    private static byte[] TryNormalShortCase() =>
    [
        (byte)OpCode.TRY, 0x06, 0x00,
        (byte)OpCode.PUSH1,
        (byte)OpCode.ENDTRY, 0x03,
        (byte)OpCode.ABORT,
        (byte)OpCode.RET,
    ];

    private static byte[] TryNormalLongCase() =>
    [
        (byte)OpCode.TRY_L,
        0x0c, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00,
        (byte)OpCode.PUSH1,
        (byte)OpCode.ENDTRY, 0x03,
        (byte)OpCode.ABORT,
        (byte)OpCode.RET,
    ];

    private static byte[] EndTryLongCase() =>
    [
        (byte)OpCode.TRY, 0x09, 0x00,
        (byte)OpCode.PUSH1,
        (byte)OpCode.ENDTRY_L,
        0x06, 0x00, 0x00, 0x00,
        (byte)OpCode.ABORT,
        (byte)OpCode.RET,
    ];

    private static IEnumerable<MatrixCase> GenerateDifferentialFuzzCases(int seed, int count)
    {
        var rng = new Random(seed);
        var operands = new[]
        {
            OperandSpec.Null,
            OperandSpec.False,
            OperandSpec.True,
            OperandSpec.IntegerZero,
            OperandSpec.IntegerOne,
            OperandSpec.IntegerFifteen,
            OperandSpec.IntegerMinusOne,
            OperandSpec.IntegerMaxI128,
            OperandSpec.ByteStringEmpty,
            OperandSpec.ByteString0f,
            OperandSpec.ByteStringSignBit,
            OperandSpec.ByteString33,
            OperandSpec.BufferEmpty,
            OperandSpec.Buffer33,
        };
        var unary = new[]
        {
            OpCode.INVERT, OpCode.NOT, OpCode.NZ, OpCode.INC, OpCode.DEC,
            OpCode.SIGN, OpCode.ABS, OpCode.NEGATE, OpCode.SQRT, OpCode.SIZE,
            OpCode.ISNULL,
        };
        var binary = new[]
        {
            OpCode.AND, OpCode.OR, OpCode.XOR,
            OpCode.EQUAL, OpCode.NOTEQUAL,
            OpCode.NUMEQUAL, OpCode.NUMNOTEQUAL,
            OpCode.LT, OpCode.LE, OpCode.GT, OpCode.GE,
            OpCode.ADD, OpCode.SUB, OpCode.MUL, OpCode.DIV, OpCode.MOD, OpCode.POW,
            OpCode.SHL, OpCode.SHR,
            OpCode.MIN, OpCode.MAX,
            OpCode.BOOLAND, OpCode.BOOLOR,
        };

        for (var caseIndex = 0; caseIndex < count; caseIndex++)
        {
            using var sb = new ScriptBuilder();
            var depth = 0;
            var steps = rng.Next(6, 18);
            for (var step = 0; step < steps; step++)
            {
                if (depth == 0 || rng.NextDouble() < 0.45)
                {
                    operands[rng.Next(operands.Length)].Emit(sb);
                    depth++;
                    continue;
                }

                var choice = rng.Next(depth >= 2 ? 8 : 4);
                switch (choice)
                {
                    case 0:
                    case 1:
                        sb.Emit(unary[rng.Next(unary.Length)]);
                        break;
                    case 2:
                        sb.Emit(OpCode.CONVERT, new[] { (byte)RandomConvertibleType(rng) });
                        break;
                    case 3:
                        sb.Emit(OpCode.ISTYPE, new[] { (byte)RandomStackItemType(rng) });
                        break;
                    case 4:
                    case 5:
                        sb.Emit(binary[rng.Next(binary.Length)]);
                        depth--;
                        break;
                    case 6:
                        sb.Emit(OpCode.DUP);
                        depth++;
                        break;
                    case 7:
                        sb.Emit(OpCode.SWAP);
                        break;
                }
            }

            sb.Emit(OpCode.RET);
            yield return new MatrixCase($"DifferentialFuzz(seed={seed},case={caseIndex})", sb.ToArray());
        }
    }

    private static StackItemType RandomConvertibleType(Random rng)
    {
        var values = new[]
        {
            StackItemType.Boolean,
            StackItemType.Integer,
            StackItemType.ByteString,
            StackItemType.Buffer,
        };
        return values[rng.Next(values.Length)];
    }

    private static StackItemType RandomStackItemType(Random rng)
    {
        var values = new[]
        {
            StackItemType.Any,
            StackItemType.Boolean,
            StackItemType.Integer,
            StackItemType.ByteString,
            StackItemType.Buffer,
            StackItemType.Array,
            StackItemType.Struct,
            StackItemType.Map,
        };
        return values[rng.Next(values.Length)];
    }

    private static byte[] WithRet(params byte[] body) => Join(body, [(byte)OpCode.RET]);

    private static byte[] WithRet(params byte[][] parts) => Join([.. parts, [(byte)OpCode.RET]]);

    private static byte[] Join(params byte[][] parts)
    {
        var length = parts.Sum(part => part.Length);
        var result = new byte[length];
        var offset = 0;
        foreach (var part in parts)
        {
            System.Buffer.BlockCopy(part, 0, result, offset, part.Length);
            offset += part.Length;
        }
        return result;
    }

    private static byte[] I16(short value) => BitConverter.GetBytes(value);

    private static byte[] I32(int value) => BitConverter.GetBytes(value);

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
            StackItemType.Array when item is Neo.VM.Types.Array array => new JObject
            {
                ["type"] = "Array",
                ["value"] = new JArray(array.Select(NormalizeNeoVmItem)),
            },
            StackItemType.Struct when item is Struct @struct => new JObject
            {
                ["type"] = "Struct",
                ["value"] = new JArray(@struct.Select(NormalizeNeoVmItem)),
            },
            StackItemType.Map when item is Map map => new JObject
            {
                ["type"] = "Map",
                ["value"] = NormalizeNeoVmMap(map),
            },
            _ => new JObject { ["type"] = item.Type.ToString() },
        };
    }

    private static JObject NormalizeRiscvItem(JObject item)
    {
        var clone = (JObject)item.DeepClone();
        clone.Remove("_handle");
        if (string.Equals(clone["type"]?.Value<string>(), "Pointer", StringComparison.Ordinal))
            clone.Remove("value");
        if (clone["value"] is JValue { Type: JTokenType.String } value)
            clone["value"] = NormalizeHexString(value.Value<string>() ?? string.Empty);
        if (clone["value"] is JArray array)
        {
            for (var i = 0; i < array.Count; i++)
                array[i] = NormalizeRiscvItem((JObject)array[i]!);
        }
        if (clone["value"] is JObject map)
        {
            foreach (var property in map.Properties().ToArray())
                property.Value = NormalizeRiscvItem((JObject)property.Value);
        }
        return clone;
    }

    private static JObject NormalizeNeoVmMap(Map map)
    {
        var obj = new JObject();
        foreach (var pair in map)
            obj[NormalizeNeoVmMapKey(pair.Key)] = NormalizeNeoVmItem(pair.Value);
        return obj;
    }

    private static string NormalizeNeoVmMapKey(StackItem key)
    {
        if (key.IsNull)
            return "0x";
        return key.Type switch
        {
            StackItemType.Boolean => ToHex(new[] { key.GetBoolean() ? (byte)1 : (byte)0 }),
            StackItemType.Integer => ToHex(key.GetInteger().ToByteArray()),
            StackItemType.ByteString => ToHex(key.GetSpan().ToArray()),
            StackItemType.Buffer => ToHex(key.GetSpan().ToArray()),
            _ => key.Type.ToString(),
        };
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

        public static OperandSpec IntegerOne { get; } = new("Integer1", sb => sb.EmitPush(BigInteger.One));

        public static OperandSpec IntegerFifteen { get; } = new("Integer15", sb => sb.EmitPush(new BigInteger(15)));

        public static OperandSpec IntegerMinusOne { get; } = new("IntegerMinus1", sb => sb.EmitPush(new BigInteger(-1)));

        public static OperandSpec IntegerMaxI128 { get; } = new("IntegerMaxI128", sb => sb.EmitPush((BigInteger.One << 127) - BigInteger.One));

        public static OperandSpec IntegerMinI128 { get; } = new("IntegerMinI128", sb => sb.EmitPush(-(BigInteger.One << 127)));

        public static OperandSpec ByteStringEmpty { get; } = new("ByteStringEmpty", sb => sb.EmitPush(Array.Empty<byte>()));

        public static OperandSpec ByteString0f { get; } = new("ByteString0f", sb => sb.EmitPush(new byte[] { 0x0f }));

        public static OperandSpec ByteStringSignBit { get; } = new("ByteStringSignBit", sb => sb.EmitPush(new byte[] { 0x80 }));

        public static OperandSpec ByteString33 { get; } = new("ByteString33", sb => sb.EmitPush(new byte[] { 0x33 }));

        public static OperandSpec BufferEmpty { get; } = new(
            "BufferEmpty",
            sb =>
            {
                sb.EmitPush(Array.Empty<byte>());
                sb.Emit(OpCode.CONVERT, new[] { (byte)StackItemType.Buffer });
            });

        public static OperandSpec Buffer33 { get; } = new(
            "Buffer33",
            sb =>
            {
                sb.EmitPush(new byte[] { 0x33 });
                sb.Emit(OpCode.CONVERT, new[] { (byte)StackItemType.Buffer });
            });

        public static OperandSpec ArrayEmpty { get; } = new("ArrayEmpty", sb => sb.Emit(OpCode.NEWARRAY0));

        public static OperandSpec StructEmpty { get; } = new("StructEmpty", sb => sb.Emit(OpCode.NEWSTRUCT0));

        public static OperandSpec MapEmpty { get; } = new("MapEmpty", sb => sb.Emit(OpCode.NEWMAP));
    }
}
