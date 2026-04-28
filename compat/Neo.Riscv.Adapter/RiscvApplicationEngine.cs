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
using System.Reflection;

namespace Neo.SmartContract.RiscV
{
    public sealed class RiscvApplicationEngine : ApplicationEngine
    {
        private const string TraceEnvironmentVariable = "NEO_RISCV_TRACE_ENGINE";
        private static readonly FieldInfo StrictModeField = typeof(Script).GetField("_strictMode", BindingFlags.Instance | BindingFlags.NonPublic)
            ?? throw new InvalidOperationException("Unable to locate Neo.VM.Script strict mode field.");

        /// <summary>
        /// Backing field of <see cref="ExecutionContext.InstructionPointer"/>. The
        /// property setter is declared <c>internal</c> and Neo.VM's
        /// <c>InternalsVisibleTo</c> list does not grant access to this adapter, so we
        /// reflect the private field once at class-load and write directly on fault.
        /// Lookup is tolerant of compiler-generated field names ("InstructionPointer",
        /// "_instructionPointer", "instructionPointer") so a future rename in Neo.VM
        /// degrades gracefully to <see langword="null"/> rather than throwing at class load.
        /// </summary>
        private static readonly FieldInfo? InstructionPointerField =
            typeof(ExecutionContext).GetField("instructionPointer", BindingFlags.Instance | BindingFlags.NonPublic)
            ?? typeof(ExecutionContext).GetField("_instructionPointer", BindingFlags.Instance | BindingFlags.NonPublic)
            ?? typeof(ExecutionContext).GetField("<InstructionPointer>k__BackingField", BindingFlags.Instance | BindingFlags.NonPublic);

        /// <summary>
        /// Backing property <see cref="ExecutionContext.LocalVariables"/> — internal setter.
        /// Reflected once at class-load so the adapter can populate the faulting frame's
        /// locals from the guest side-channel (fault-locals FFI) for dev-time test harnesses
        /// like Test_Abort that assert <c>exception.CurrentContext.LocalVariables[0]</c>.
        /// </summary>
        private static readonly PropertyInfo? LocalVariablesProperty =
            typeof(ExecutionContext).GetProperty("LocalVariables", BindingFlags.Instance | BindingFlags.Public);

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
            var contexts = InvocationStack
                .Reverse()
                .ToArray();

            Trace($"execute start contexts={contexts.Length} trigger={Trigger}");
            var result = new RiscvExecutionResult(VMState.HALT, System.Array.Empty<StackItem>(), null);
            IReadOnlyList<StackItem> initialStack = System.Array.Empty<StackItem>();

            for (var index = contexts.Length - 1; index >= 0; index--)
            {
                var context = contexts[index];
                var contextState = context.GetState<ExecutionContextState>();
                Trace($"bridge dispatch index={index} ip={context.InstructionPointer} scriptLen={((ReadOnlyMemory<byte>)context.Script).Length}");
                var contextInitialStack = context.EvaluationStack.Count > 0
                    ? Enumerable.Range(0, context.EvaluationStack.Count)
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
                    ContextUnloaded(context);
                }
            }
            else if (result.State == VMState.FAULT && CurrentContext is not null)
            {
                RollbackContextNotifications(CurrentContext);
                // Restore the faulting opcode offset reported by the guest so dev-time
                // introspection (TestException.CurrentContext.InstructionPointer) sees the
                // real offset instead of 0. Not consensus-affecting — FAULT rolls back
                // snapshot commits already (see the HALT branch above).
                if (result.FaultIp is int ip && InstructionPointerField is not null)
                {
                    try
                    {
                        InstructionPointerField.SetValue(CurrentContext, ip);
                    }
                    catch (Exception ex)
                    {
                        Trace($"failed to propagate fault IP {ip}: {ex.Message}");
                    }
                }
                // Populate LocalVariables from the guest's fault-time locals snapshot so
                // dev tests like Test_Abort/Test_Assert* that assert on
                // exception.CurrentContext.LocalVariables[n] see the runtime state.
                var setter = LocalVariablesProperty?.GetSetMethod(nonPublic: true);
                if (_bridge is NativeRiscvVmBridge nativeBridge && setter is not null)
                {
                    var localsBytes = nativeBridge.TryReadLastFaultLocals();
                    if (localsBytes.Length > 0)
                    {
                        try
                        {
                            var items = FastCodecReader.DecodeStack(localsBytes, ReferenceCounter);
                            if (items.Length > 0)
                            {
                                setter.Invoke(CurrentContext, new object[] { new Slot(items, ReferenceCounter) });
                            }
                        }
                        catch (Exception ex)
                        {
                            Trace($"failed to propagate fault locals ({localsBytes.Length} bytes): {ex.Message}");
                        }
                    }
                }
            }

            FaultException = result.FaultException;
            State = result.State;
            return State;
        }

        private static void Trace(string message)
        {
            if (!string.Equals(Environment.GetEnvironmentVariable(TraceEnvironmentVariable), "1", StringComparison.Ordinal))
                return;

            Console.Error.WriteLine($"[neo-riscv-engine] {message}");
        }

        private static bool IsStrictMode(Script script)
        {
            return (bool)StrictModeField.GetValue(script)!;
        }
    }
}
