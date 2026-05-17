using Neo.Network.P2P.Payloads;
using Neo.SmartContract;
using Neo.VM;
using System;
using System.Linq;

namespace Neo.SmartContract.RiscV
{
    public sealed partial class NativeRiscvVmBridge
    {
        private const string TraceBlockEnvironmentVariable = "NEO_RISCV_TRACE_BLOCK";

        private static bool IsDiagnosticBlock(ApplicationEngine engine)
        {
            if (engine.PersistingBlock is not { } block)
                return false;

            var configured = Environment.GetEnvironmentVariable(TraceBlockEnvironmentVariable);
            if (string.IsNullOrWhiteSpace(configured))
                return false;

            return configured
                .Split(',', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries)
                .Any(value => uint.TryParse(value, out var height) && height == block.Index);
        }

        private static void DiagnosticTrace(RiscvExecutionRequest request, string category, string message)
        {
            if (!IsDiagnosticBlock(request.Engine))
                return;

            var blockIndex = request.Engine.PersistingBlock?.Index.ToString() ?? "<none>";
            Console.Error.WriteLine(
                $"[neo-riscv-diagnostic] block={blockIndex} trigger={request.Trigger} " +
                $"container={DescribeContainer(request.Engine.ScriptContainer)} " +
                $"context={DescribeContext(request.Engine.CurrentContext)} {category} {message}");
        }

        private static string DescribeContainer(IVerifiable? container)
        {
            return container switch
            {
                Transaction tx => tx.Hash.ToString(),
                Block block => block.Hash.ToString(),
                _ => container?.GetType().Name ?? "<none>"
            };
        }

        private static string DescribeContext(ExecutionContext? context)
        {
            if (context is null)
                return "<null>";

            var state = context.GetState<ExecutionContextState>();
            return $"{state.Contract?.Manifest.Name ?? "<script>"}:{state.MethodName ?? "<none>"}:{state.ScriptHash?.ToString() ?? "<no-hash>"}";
        }

        private static string DescribeBytes(byte[] bytes, int maxBytes = 96)
        {
            if (bytes.Length <= maxBytes)
                return Convert.ToHexString(bytes);

            return $"{Convert.ToHexString(bytes.AsSpan(0, maxBytes))}...(+{bytes.Length - maxBytes} bytes)";
        }
    }
}
