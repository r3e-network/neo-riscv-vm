using Neo.Test.Extensions;
using Neo.Test.Types;
using Neo.VM;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;
using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using Array = System.Array;

namespace Neo.Test;

[TestClass]
public class UT_RiscvVmJson
{
    [TestMethod]
    public void TestDirectSmoke()
    {
        using var runner = RiscvVmRunner.CreateFromEnvironment();
        using var script = new ScriptBuilder();
        script.Emit(OpCode.PUSH1);
        script.Emit(OpCode.RET);

        var outcome = runner.Execute(script.ToArray());

        Assert.AreEqual(VMState.HALT, outcome.State);
        Assert.AreEqual(1, outcome.ResultStack.Count);
        Assert.AreEqual("1", outcome.ResultStack[0]["value"]!.Value<string>());
    }

    [TestMethod]
    public void TestCopiedNeoVmJsonFinalStates()
    {
        using var runner = RiscvVmRunner.CreateFromEnvironment();
        var root = Path.Combine(AppContext.BaseDirectory, "Corpus", "Tests");
        var filter = Environment.GetEnvironmentVariable("NEO_RISCV_VM_JSON_FILTER");
        var failures = new List<string>();
        var executed = 0;
        var skippedBreak = 0;

        foreach (var file in Directory.GetFiles(root, "*.json", SearchOption.AllDirectories).OrderBy(p => p, StringComparer.Ordinal))
        {
            if (!string.IsNullOrWhiteSpace(filter) &&
                file.IndexOf(filter, StringComparison.OrdinalIgnoreCase) < 0)
            {
                continue;
            }

            var json = File.ReadAllText(file);
            var ut = json.DeserializeJson<VMUT>();

            foreach (var test in ut.Tests ?? Array.Empty<VMUTEntry>())
            {
                var final = test.Steps?.LastOrDefault()?.Result;
                if (final is null)
                {
                    failures.Add($"{Path.GetFileName(file)}::{test.Name} missing final result.");
                    continue;
                }

                if (final.State == VMState.BREAK)
                {
                    skippedBreak++;
                    continue;
                }

                executed++;
                Console.WriteLine($"RISC-V compatibility: {Path.GetRelativePath(root, file)} :: {test.Name}");
                try
                {
                    var outcome = runner.Execute(test.Script);
                    AssertFinalStateMatches(final, outcome, file, test.Name);
                }
                catch (Exception ex)
                {
                    failures.Add($"{Path.GetFileName(file)}::{test.Name} => {ex.GetType().Name}: {ex.Message}");
                }
            }
        }

        if (failures.Count > 0)
        {
            Assert.Fail(
                $"Copied NeoVM JSON compatibility failures: {failures.Count} out of {executed} executed cases. " +
                $"Skipped BREAK-only cases: {skippedBreak}.{Environment.NewLine}{string.Join(Environment.NewLine, failures.Take(50))}");
        }

        Assert.IsTrue(executed > 0, "No copied NeoVM JSON cases were executed.");
    }

    private static void AssertFinalStateMatches(VMUTExecutionEngineState expected, ExecutionOutcome actual, string file, string testName)
    {
        if (expected.State != actual.State)
        {
            Assert.Fail(
                $"{file}::{testName} state mismatch. Expected={expected.State}, Actual={actual.State}, Fault={actual.FaultMessage ?? "<none>"}");
        }

        if (actual.State == VMState.FAULT)
        {
            if (!string.IsNullOrEmpty(expected.ExceptionMessage))
            {
                Assert.AreEqual(expected.ExceptionMessage, actual.FaultMessage, $"{file}::{testName} fault message mismatch.");
            }

            return;
        }

        AssertStacksMatch(expected.ResultStack ?? Array.Empty<VMUTStackItem>(), actual.ResultStack, file, testName);
    }

    private static void AssertStacksMatch(IReadOnlyList<VMUTStackItem> expected, IReadOnlyList<JObject> actual, string file, string testName)
    {
        Assert.AreEqual(expected.Count, actual.Count, $"{file}::{testName} result stack length mismatch.");

        for (var i = 0; i < expected.Count; i++)
        {
            var expectedJson = PrepareJsonItem(expected[i]);
            var actualJson = NormalizeActualItem(expectedJson, actual[actual.Count - 1 - i]);

            Assert.AreEqual(
                expectedJson.ToString(Formatting.None),
                actualJson.ToString(Formatting.None),
                $"{file}::{testName} result stack item {i} mismatch.");
        }
    }

    private static JObject NormalizeActualItem(JObject expected, JObject item)
    {
        item.Remove("_handle");
        var expectedType = expected["type"]?.Value<string>();
        var actualType = item["type"]?.Value<string>();
        if (expectedType == "Buffer" && actualType == "ByteString")
        {
            item["type"] = "Buffer";
        }
        // Normalize empty byte values: "" and "0x" are equivalent for ByteString/Buffer types
        if (expectedType is "ByteString" or "Buffer" && actualType is "ByteString" or "Buffer")
        {
            var expectedValue = expected["value"]?.Type == Newtonsoft.Json.Linq.JTokenType.String ? expected["value"]?.Value<string>() : null;
            var actualValue = item["value"]?.Type == Newtonsoft.Json.Linq.JTokenType.String ? item["value"]?.Value<string>() : null;
            if (expectedValue != null && actualValue != null)
            {
                if ((expectedValue == "" && actualValue == "0x") || (expectedValue == "0x" && actualValue == ""))
                {
                    item["value"] = expectedValue;
                }
            }
        }
        return item;
    }

    private static JObject PrepareJsonItem(VMUTStackItem item)
    {
        var ret = new JObject
        {
            ["type"] = item.Type.ToString(),
            ["value"] = item.Value
        };

        switch (item.Type)
        {
            case VMUTStackItemType.Null:
                ret["type"] = VMUTStackItemType.Null.ToString();
                ret.Remove("value");
                break;
            case VMUTStackItemType.Pointer:
                ret["type"] = VMUTStackItemType.Pointer.ToString();
                ret["value"] = item.Value.Value<int>();
                break;
            case VMUTStackItemType.String:
                ret["type"] = VMUTStackItemType.ByteString.ToString();
                ret["value"] = Neo.Test.Extensions.StringExtensions.ToHexString(
                    System.Text.Encoding.UTF8.GetBytes(item.Value.Value<string>() ?? string.Empty));
                break;
            case VMUTStackItemType.ByteString:
            case VMUTStackItemType.Buffer:
                {
                    var value = ret["value"]?.Value<string>() ?? string.Empty;
                    Assert.IsTrue(string.IsNullOrEmpty(value) || value.StartsWith("0x"), $"'0x' prefix required for value: '{value}'");
                    ret["value"] = value.ToLowerInvariant();
                    break;
                }
            case VMUTStackItemType.Integer:
                ret["value"] = ret["value"]!.Value<string>();
                break;
            case VMUTStackItemType.Struct:
            case VMUTStackItemType.Array:
                {
                    var array = (JArray)ret["value"]!;
                    for (var i = 0; i < array.Count; i++)
                    {
                        array[i] = PrepareJsonItem(JsonConvert.DeserializeObject<VMUTStackItem>(array[i]!.ToString())!);
                    }
                    break;
                }
            case VMUTStackItemType.Map:
                {
                    var obj = (JObject)ret["value"]!;
                    foreach (var prop in obj.Properties().ToList())
                    {
                        obj[prop.Name] = PrepareJsonItem(JsonConvert.DeserializeObject<VMUTStackItem>(prop.Value.ToString())!);
                    }
                    break;
                }
        }

        return ret;
    }
}
