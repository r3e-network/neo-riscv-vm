// Copyright (C) 2015-2026 The Neo Project.
//
// RiscvApplicationEngine.cs file belongs to the neo project and is free
// software distributed under the MIT software license, see the
// accompanying file LICENSE in the main directory of the
// repository or http://www.opensource.org/licenses/mit-license.php
// for more details.

using Neo.Network.P2P.Payloads;
using Neo.Persistence;
using Neo.VM;
using Neo.VM.Types;
using System;
using System.Collections.Generic;
using System.Linq;

namespace Neo.SmartContract.RiscV
{
    /// <summary>
    /// The RISC-V-backed <see cref="ApplicationEngine"/> used to execute smart
    /// contracts through the PolkaVM runtime.
    /// </summary>
    /// <remarks>
    /// <para><b>Role in the neovm→riscvvm replacement:</b> This engine is the
    /// .NET-side execution entry point. When the node's
    /// <see cref="IRiscvApplicationEngineProvider"/> creates an engine, it
    /// produces a <see cref="RiscvApplicationEngine"/> whose <see cref="Execute"/>
    /// override forwards the contract script to the native
    /// <see cref="IRiscvVmBridge"/> (a <see cref="NativeRiscvVmBridge"/> backed
    /// by the Rust <c>neo-riscv-host</c> via P/Invoke).</para>
    /// <para><b>Two execution paths:</b> Depending on
    /// <see cref="RiscvExecutionDispatcher"/>, the contract is either run
    /// through the NeoVM-compatibility interpreter (legacy contracts) or
    /// executed directly as a PolkaVM binary (true RISC-V contracts). Both
    /// paths return through this engine so that gas accounting, fault
    /// reporting, and the result stack are unified.</para>
    /// <para><b>Location:</b> This class lives in <c>Neo.Riscv.Adapter.dll</c>,
    /// the native-dependency adapter loaded by the node and devpack test
    /// framework.</para>
    /// </remarks>
    public sealed class RiscvApplicationEngine : ApplicationEngine, IRiscvApplicationEngine
    {
        private const string TraceEnvironmentVariable = "NEO_RISCV_TRACE_ENGINE";
        private const string FaultLogEnvironmentVariable = "NEO_RISCV_LOG_FAULTS";
        private const string TraceBlockEnvironmentVariable = "NEO_RISCV_TRACE_BLOCK";
        private const string StopBeforeBlockEnvironmentVariable = "NEO_RISCV_STOP_BEFORE_BLOCK";
        private readonly IRiscvVmBridge _bridge;

        /// <summary>
        /// Optional devpack/test-framework hooks for intercepting adapter execution.
        /// Leave <see langword="null"/> in production — adapter call sites check for null
        /// and incur no overhead when unset.
        /// </summary>
        public IRiscvApplicationEngineTestingHooks? TestingHooks { get; set; }

        internal RiscvApplicationEngine(
            TriggerType trigger,
            IVerifiable? container,
            DataCache snapshotCache,
            Block? persistingBlock,
            ProtocolSettings settings,
            long gas,
            IRiscvVmBridge bridge,
            IDiagnostic? diagnostic = null,
            JumpTable? jumpTable = null)
            : base(trigger, container, snapshotCache, persistingBlock, settings, gas, diagnostic, jumpTable)
        {
            _bridge = bridge ?? throw new ArgumentNullException(nameof(bridge));
        }

        public override VMState Execute()
        {
            StopBeforeRequestedBlock();
            var contexts = InvocationStack
                .Reverse()
                .ToArray();

            Trace($"execute start contexts={contexts.Length} trigger={Trigger}");
            TraceDiagnosticBlock($"execute start contexts={contexts.Length} trigger={Trigger} container={DescribeContainer(ScriptContainer)} current={DescribeContext(CurrentContext)}");
            var result = new RiscvExecutionResult(VMState.HALT, System.Array.Empty<StackItem>(), null);
            IReadOnlyList<StackItem> initialStack = System.Array.Empty<StackItem>();

            for (var index = contexts.Length - 1; index >= 0; index--)
            {
                var context = contexts[index];
                var contextState = context.GetState<ExecutionContextState>();
                Trace($"bridge dispatch index={index} ip={context.InstructionPointer} scriptLen={((ReadOnlyMemory<byte>)context.Script).Length}");
                TraceDiagnosticBlock($"dispatch index={index} ip={context.InstructionPointer} scriptLen={((ReadOnlyMemory<byte>)context.Script).Length} context={DescribeContext(context)}");
                var contextInitialStack = context.EvaluationStack.Count > 0
                    ? Enumerable.Range(0, context.EvaluationStack.Count)
                        .Reverse()
                        .Select(context.EvaluationStack.Peek)
                        .ToArray()
                    : initialStack;
                var prefix = contexts.Take(index + 1).ToArray();
                var scripts = prefix
                    .Select(current => ((ReadOnlyMemory<byte>)current.Script).ToArray())
                    .ToArray();
                var scriptHashes = prefix
                    .Select(current => current.GetState<ExecutionContextState>().ScriptHash ?? ((ReadOnlyMemory<byte>)current.Script).Span.ToScriptHash())
                    .ToArray();
                var contractTypes = prefix
                    .Select(current => current.GetState<ExecutionContextState>().Contract?.Type ?? ContractType.NeoVM)
                    .ToArray();
                var executionFacadeHashes = contractTypes
                    .Zip(scriptHashes, (contractType, scriptHash) =>
                        RiscvCompatibilityContracts.ResolveExecutionFacadeHash(contractType, scriptHash))
                    .ToArray();
                var methodName = contextState.MethodName
                    ?? contextState.Contract?.Manifest.Abi.Methods
                        .FirstOrDefault(method => method.Offset == context.InstructionPointer)
                        ?.Name;

                result = _bridge.Execute(new RiscvExecutionRequest(
                    this,
                    Trigger,
                    ProtocolSettings.Network,
                    ProtocolSettings.AddressVersion,
                    PersistingBlock?.Timestamp ?? 0,
                    GasLeft,
                    context.GetState<ExecutionContextState>().CallFlags,
                    scripts,
                    scriptHashes,
                    contractTypes,
                    executionFacadeHashes,
                    contextInitialStack,
                    context.InstructionPointer,
                    methodName));

                if (result.State != VMState.HALT)
                    break;

                initialStack = result.ResultStack;
                if (Trigger != TriggerType.Verification && !IsStrictMode(context.Script))
                    break;
            }

            while (ResultStack.Count > 0)
            {
                ResultStack.Pop();
            }

            foreach (var item in result.ResultStack)
            {
                ResultStack.Push(item);
            }

            if (result.State == VMState.HALT)
            {
                while (InvocationStack.Count > 0)
                {
                    var context = InvocationStack.Pop();
                    UnloadContextFromBridge(context);
                }
            }
            else if (result.State == VMState.FAULT && CurrentContext is not null)
            {
                RollbackContextNotifications(CurrentContext);
                // Restore the faulting opcode offset reported by the guest so dev-time
                // introspection (TestException.CurrentContext.InstructionPointer) sees the
                // real offset instead of 0. Not consensus-affecting — FAULT rolls back
                // snapshot commits already (see the HALT branch above).
                ApplyFaultMetadata(result);
            }

            if (result.State == VMState.FAULT)
                LogFault(result);

            FaultException = result.FaultException;
            State = result.State;
            return State;
        }

        internal void CompleteCurrentContextFromBridge(RiscvExecutionResult result)
        {
            while (ResultStack.Count > 0)
            {
                ResultStack.Pop();
            }

            if (result.State == VMState.HALT)
            {
                if (InvocationStack.Count == 0 || CurrentContext is null)
                    throw new InvalidOperationException("No current execution context is available.");

                var currentContext = CurrentContext;
                ReplaceEvaluationStack(currentContext, result.ResultStack);
                var context = InvocationStack.Pop();
                if (!ReferenceEquals(context, currentContext))
                    throw new InvalidOperationException("The completed RISC-V context is not current.");
                ContextUnloaded(context);
            }
            else if (CurrentContext is not null)
            {
                RollbackContextNotifications(CurrentContext);
                ApplyFaultMetadata(result);
            }

            FaultException = result.FaultException;
            State = result.State;
        }

        internal void UnloadNestedContextFromBridge(ExecutionContext nestedContext, RiscvExecutionResult result)
        {
            if (InvocationStack.Count == 0 || !ReferenceEquals(CurrentContext, nestedContext))
            {
                Trace($"nested unload mismatch count={InvocationStack.Count} current={DescribeContext(CurrentContext)} nested={DescribeContext(nestedContext)}");
                throw new InvalidOperationException("The nested RISC-V context is not current.");
            }

            var callerContext = nestedContext.GetState<ExecutionContextState>().CallingContext;
            var callerStackDepth = callerContext?.EvaluationStack.Count ?? 0;

            ReplaceEvaluationStack(nestedContext, result.ResultStack);

            var context = InvocationStack.Pop();
            if (!ReferenceEquals(context, nestedContext))
                throw new InvalidOperationException("The nested RISC-V context is not current.");
            ContextUnloaded(context);

            if (callerContext is not null)
            {
                while (callerContext.EvaluationStack.Count > callerStackDepth)
                {
                    callerContext.EvaluationStack.Pop();
                }
            }
        }

        internal void DiscardNestedContextFromBridge(ExecutionContext nestedContext)
        {
            if (!InvocationStack.Any(context => ReferenceEquals(context, nestedContext)))
                throw new InvalidOperationException("The nested RISC-V context is not current.");

            var callerContext = nestedContext.GetState<ExecutionContextState>().CallingContext;
            var removedNestedContext = false;
            while (InvocationStack.Count > 0 && !ReferenceEquals(InvocationStack.Peek(), callerContext))
            {
                var context = InvocationStack.Pop();
                removedNestedContext |= ReferenceEquals(context, nestedContext);
                RollbackContextNotifications(context);
                SetCurrentContextFromInvocationStack();
                Diagnostic?.ContextUnloaded(context);
            }

            if (!removedNestedContext)
                throw new InvalidOperationException("The nested RISC-V context is not current.");
        }

        private void ApplyFaultMetadata(RiscvExecutionResult result)
        {
            if (result.FaultIp is int ip)
            {
                NeoVmExecutionContextAccessors.TrySetInstructionPointer(CurrentContext, ip, Trace);
            }

            if (_bridge is NativeRiscvVmBridge nativeBridge)
            {
                var localsBytes = nativeBridge.TryReadLastFaultLocals();
                if (localsBytes.Length > 0)
                {
                    try
                    {
                        var items = FastCodecReader.DecodeStack(localsBytes, ReferenceCounter);
                        if (items.Length > 0)
                        {
                            NeoVmExecutionContextAccessors.TrySetLocalVariables(
                                CurrentContext,
                                new Slot(items, ReferenceCounter),
                                Trace);
                        }
                    }
                    catch (Exception ex)
                    {
                        Trace($"failed to propagate fault locals ({localsBytes.Length} bytes): {ex.Message}");
                    }
                }
            }
        }

        private void UnloadContextFromBridge(ExecutionContext context)
        {
            if (!IsRiscvContext(context))
            {
                ContextUnloaded(context);
                return;
            }

            SetCurrentContextFromInvocationStack();
            var state = context.GetState<ExecutionContextState>();
            if (UncaughtException is null)
            {
                state.SnapshotCache?.Commit();
                if (CurrentContext is not null)
                    CurrentContext.GetState<ExecutionContextState>().NotificationCount += state.NotificationCount;
            }
            else
            {
                RollbackContextNotifications(context);
            }

            Diagnostic?.ContextUnloaded(context);
        }

        private void SetCurrentContextFromInvocationStack()
        {
            NeoVmExecutionContextAccessors.SetCurrentContext(this, InvocationStack.Count > 0 ? InvocationStack.Peek() : null);
        }

        private static void ReplaceEvaluationStack(ExecutionContext context, IReadOnlyList<StackItem> stack)
        {
            while (context.EvaluationStack.Count > 0)
                context.EvaluationStack.Pop();

            foreach (var item in stack)
                context.EvaluationStack.Push(item);
        }

        private static bool IsRiscvContext(ExecutionContext context)
        {
            if (context.GetState<ExecutionContextState>().Contract?.Type == ContractType.RiscV)
                return true;

            // Delegate to the single canonical PVM magic-byte check to avoid drift.
            return RiscvExecutionDispatcher.IsPvmBinary(context.Script);
        }

        private static string DescribeContext(ExecutionContext? context)
        {
            if (context is null)
                return "<null>";

            var state = context.GetState<ExecutionContextState>();
            return $"{state.Contract?.Manifest.Name ?? "<script>"}:{state.MethodName ?? "<none>"}:{state.ScriptHash?.ToString() ?? "<no-hash>"}";
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

        internal void RollbackCurrentContextNotificationsTo(int notificationCount)
        {
            if (CurrentContext is null)
                return;

            var state = CurrentContext.GetState<ExecutionContextState>();
            if (state.NotificationCount <= notificationCount)
                return;

            var nestedNotificationCount = state.NotificationCount - notificationCount;
            state.NotificationCount = nestedNotificationCount;
            RollbackContextNotifications(CurrentContext);
            state.NotificationCount = notificationCount;
        }

        private static void Trace(string message)
        {
            if (!string.Equals(Environment.GetEnvironmentVariable(TraceEnvironmentVariable), "1", StringComparison.Ordinal))
                return;

            Console.Error.WriteLine($"[neo-riscv-engine] {message}");
        }

        private void StopBeforeRequestedBlock()
        {
            var configured = Environment.GetEnvironmentVariable(StopBeforeBlockEnvironmentVariable);
            if (!uint.TryParse(configured, out var stopHeight))
                return;
            if (PersistingBlock is not { } block || block.Index < stopHeight)
                return;

            Console.Error.WriteLine(
                $"[neo-riscv-stop] block={block.Index} trigger={Trigger} container={DescribeContainer(ScriptContainer)} " +
                $"context={DescribeContext(CurrentContext)} stopBefore={stopHeight}");
            throw new InvalidOperationException($"{StopBeforeBlockEnvironmentVariable} reached at block {block.Index}.");
        }

        private void TraceDiagnosticBlock(string message)
        {
            if (PersistingBlock is not { } block)
                return;
            var configured = Environment.GetEnvironmentVariable(TraceBlockEnvironmentVariable);
            if (string.IsNullOrWhiteSpace(configured))
                return;
            var matches = configured
                .Split(',', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries)
                .Any(value => uint.TryParse(value, out var height) && height == block.Index);
            if (!matches)
                return;

            Console.Error.WriteLine($"[neo-riscv-engine][diagnostic] block={block.Index} {message}");
        }

        private void LogFault(RiscvExecutionResult result)
        {
            if (!string.Equals(Environment.GetEnvironmentVariable(FaultLogEnvironmentVariable), "1", StringComparison.Ordinal))
                return;

            var containerHash = ScriptContainer switch
            {
                Transaction tx => tx.Hash.ToString(),
                Block block => block.Hash.ToString(),
                _ => ScriptContainer?.GetType().Name ?? "<none>"
            };
            var blockIndex = PersistingBlock?.Index.ToString() ?? "<none>";
            var context = DescribeContext(CurrentContext);
            var message = result.FaultException?.Message ?? "<none>";
            var baseException = result.FaultException?.GetBaseException();
            var baseMessage = baseException is null || ReferenceEquals(baseException, result.FaultException)
                ? string.Empty
                : $" base={baseException.GetType().FullName}:{baseException.Message}";
            var ip = result.FaultIp?.ToString() ?? "<none>";
            Console.Error.WriteLine(
                $"[neo-riscv-fault] block={blockIndex} trigger={Trigger} container={containerHash} " +
                $"context={context} ip={ip} message={message}{baseMessage}");
        }

        private static bool IsStrictMode(Script script)
        {
            return NeoVmExecutionContextAccessors.IsStrictMode(script);
        }
    }
}
