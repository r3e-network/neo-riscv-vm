using System;
using System.Collections.Generic;
using System.Linq;
using System.Runtime.InteropServices;
using System.Text;
using Neo.SmartContract;
using Neo.VM;
using Neo.VM.Types;

namespace Neo.SmartContract.RiscV
{
    public sealed partial class NativeRiscvVmBridge
    {
        public RiscvExecutionResult Execute(RiscvExecutionRequest request)
        {
            if (request is null) throw new ArgumentNullException(nameof(request));
            if (request.Scripts.Count == 0) throw new InvalidOperationException("No script loaded for RISC-V execution.");
            var script = request.Scripts[^1];
            var contractType = request.ContractTypes.Count == request.Scripts.Count
                ? request.ContractTypes[^1]
                : ContractType.NeoVM;
            var executionKind = RiscvExecutionDispatcher.Resolve(contractType, script);
            var scope = new ExecutionScope();

            return executionKind switch
            {
                RiscvExecutionKind.GuestNeoVmContract =>
                    ExecuteScriptInternal(request, script, request.InitialStack.ToArray(), request.InitialInstructionPointer, scope),
                RiscvExecutionKind.NativeRiscvDirect =>
                    ExecuteNativeContractInternal(request, script, request.InitialStack.ToArray(), request.Method ?? throw new InvalidOperationException("Method is required for native RISC-V contract execution."), scope),
                _ => throw new InvalidOperationException($"Unsupported execution kind: {executionKind}."),
            };
        }

        public RiscvExecutionResult ExecuteContract(
            ApplicationEngine engine,
            ContractState contract,
            string method,
            CallFlags flags,
            IReadOnlyList<StackItem> args)
        {
            if (engine is null) throw new ArgumentNullException(nameof(engine));
            if (contract is null) throw new ArgumentNullException(nameof(contract));
            if (method is null) throw new ArgumentNullException(nameof(method));

            var script = contract.Script.ToArray();
            var contractHash = contract.Hash;
            var contractType = contract.Type;
            var executionFacadeHash = RiscvCompatibilityContracts.ResolveExecutionFacadeHash(contractType, contractHash);

            var descriptor = contract.Manifest.Abi.GetMethod(method, args.Count)
                ?? throw new InvalidOperationException(
                    $"Method \"{method}\" with {args.Count} parameter(s) doesn't exist in the contract {contractHash}.");

            var initialStack = new StackItem[args.Count];
            for (var i = 0; i < args.Count; i++)
            {
                initialStack[i] = args[args.Count - 1 - i];
            }

            var request = new RiscvExecutionRequest(
                engine,
                engine.Trigger,
                engine.ProtocolSettings.Network,
                engine.ProtocolSettings.AddressVersion,
                engine.PersistingBlock?.Timestamp ?? 0,
                engine.GasLeft,
                flags,
                new[] { script },
                new[] { contractHash },
                new[] { contractType },
                new[] { executionFacadeHash },
                initialStack,
                descriptor.Offset,
                method);

            return Execute(request);
        }

        private RiscvExecutionResult ExecuteScriptInternal(RiscvExecutionRequest request, byte[] script, StackItem[] initialStack, int initialInstructionPointer, ExecutionScope scope)
        {
            var scriptPtr = Marshal.AllocHGlobal(script.Length);
            NativeExecutionResult nativeResult = default;
            var callbackState = new HostCallbackState { Bridge = this, Request = request, Scope = scope };
            var callbackHandle = GCHandle.Alloc(callbackState);
            var initialState = CreateNativeHostResult(initialStack, scope);
            var previousScript = scope.CurrentScript;
            scope.CurrentScript = new Script(script, request.Engine.IsHardforkEnabled(Hardfork.HF_Basilisk));

            try
            {
                if (TraceEnabled)
                {
                    Trace($"execute script initialIp={initialInstructionPointer} scriptHex={Convert.ToHexString(script)}");
                    for (var i = 0; i < initialStack.Length; i++)
                        Trace($"execute initialStack[{i}] value={DescribeStackItem(initialStack[i])}");
                }
                Marshal.Copy(script, 0, scriptPtr, script.Length);
                if (!_executeScript(
                        scriptPtr,
                        (nuint)script.Length,
                        (nuint)initialInstructionPointer,
                        (byte)request.Trigger,
                        request.NetworkMagic,
                        request.AddressVersion,
                        request.PersistingTimestamp,
                        request.GasLeft,
                        checked((long)request.Engine.ExecFeePicoFactor),
                        initialState.StackPtr,
                        initialState.StackLen,
                        GCHandle.ToIntPtr(callbackHandle),
                        _hostCallbackPtr,
                        _hostFreeCallbackPtr,
                        out nativeResult))
                    throw new InvalidOperationException("Native RISC-V ABI call failed.");

                var stack = ReadStack(nativeResult.StackPtr, nativeResult.StackLen, request.Engine.ReferenceCounter, scope, decodeStorageContextTokens: true);
                if (TraceEnabled)
                {
                    for (var i = 0; i < stack.Length; i++)
                        Trace($"execute result[{i}] type={stack[i].GetType().Name} value={DescribeStackItem(stack[i])}");
                }
                var gasFault = TryChargeExecutionFee(request.Engine, nativeResult.FeeConsumedPico, "execute");
                if (gasFault is not null)
                    return gasFault;
                var state = nativeResult.State == 0 ? VMState.HALT : VMState.FAULT;
                var faultMessage = nativeResult.ErrorPtr == IntPtr.Zero
                    ? "Native Neo RISC-V execution fault."
                    : Marshal.PtrToStringUTF8(nativeResult.ErrorPtr, checked((int)nativeResult.ErrorLen)) ?? "Native Neo RISC-V execution fault.";
                var fault = state == VMState.FAULT ? RehydrateNativeException(faultMessage) : null;
                var faultIp = state == VMState.FAULT ? TryReadLastFaultIp() : null;
                if (state == VMState.FAULT)
                {
                    Trace($"execute fault state={state} message={faultMessage} ip={faultIp?.ToString() ?? "<none>"}");
                }

                return new RiscvExecutionResult(state, stack, fault, faultIp);
            }
            finally
            {
                scope.CurrentScript = previousScript;
                StaticHostFreeCallback(IntPtr.Zero, ref initialState);
                if (nativeResult.StackPtr != IntPtr.Zero || nativeResult.ErrorPtr != IntPtr.Zero)
                {
                    _freeExecutionResult(ref nativeResult);
                }
                if (callbackHandle.IsAllocated)
                {
                    callbackHandle.Free();
                }
                Marshal.FreeHGlobal(scriptPtr);
            }
        }

        private RiscvExecutionResult ExecuteNativeContractInternal(RiscvExecutionRequest request, byte[] binary, StackItem[] initialStack, string method, ExecutionScope scope)
        {
            var binaryPtr = Marshal.AllocHGlobal(binary.Length);
            var methodBytes = Encoding.UTF8.GetBytes(method);
            var methodPtr = Marshal.AllocHGlobal(methodBytes.Length);
            NativeExecutionResult nativeResult = default;
            var callbackState = new HostCallbackState { Bridge = this, Request = request, Scope = scope };
            var callbackHandle = GCHandle.Alloc(callbackState);
            var initialState = CreateNativeHostResult(initialStack, scope);
            var previousScript = scope.CurrentScript;
            scope.CurrentScript = new Script(binary, request.Engine.IsHardforkEnabled(Hardfork.HF_Basilisk));

            try
            {
                Marshal.Copy(binary, 0, binaryPtr, binary.Length);
                Marshal.Copy(methodBytes, 0, methodPtr, methodBytes.Length);
                if (!_executeNativeContract(
                        binaryPtr,
                        (nuint)binary.Length,
                        methodPtr,
                        (nuint)methodBytes.Length,
                        initialState.StackPtr,
                        initialState.StackLen,
                        (byte)request.Trigger,
                        request.NetworkMagic,
                        request.AddressVersion,
                        request.PersistingTimestamp,
                        request.GasLeft,
                        checked((long)request.Engine.ExecFeePicoFactor),
                        GCHandle.ToIntPtr(callbackHandle),
                        _hostCallbackPtr,
                        _hostFreeCallbackPtr,
                        out nativeResult))
                    throw new InvalidOperationException("Native RISC-V contract execution failed.");

                var stack = ReadStack(nativeResult.StackPtr, nativeResult.StackLen, request.Engine.ReferenceCounter, scope, decodeStorageContextTokens: true);
                if (TraceEnabled)
                {
                    for (var i = 0; i < stack.Length; i++)
                        Trace($"native execute result[{i}] type={stack[i].GetType().Name} value={DescribeStackItem(stack[i])}");
                }
                var gasFault = TryChargeExecutionFee(request.Engine, nativeResult.FeeConsumedPico, "native execute");
                if (gasFault is not null)
                    return gasFault;
                var state = nativeResult.State == 0 ? VMState.HALT : VMState.FAULT;
                var faultMessage = nativeResult.ErrorPtr == IntPtr.Zero
                    ? "Native RISC-V contract execution fault."
                    : Marshal.PtrToStringUTF8(nativeResult.ErrorPtr, checked((int)nativeResult.ErrorLen)) ?? "Native RISC-V contract execution fault.";
                var fault = state == VMState.FAULT ? RehydrateNativeException(faultMessage) : null;
                if (state == VMState.FAULT)
                {
                    Trace($"native contract fault state={state} message={faultMessage}");
                }

                return new RiscvExecutionResult(state, stack, fault);
            }
            finally
            {
                scope.CurrentScript = previousScript;
                StaticHostFreeCallback(IntPtr.Zero, ref initialState);
                if (nativeResult.StackPtr != IntPtr.Zero || nativeResult.ErrorPtr != IntPtr.Zero)
                {
                    _freeExecutionResult(ref nativeResult);
                }
                if (callbackHandle.IsAllocated)
                {
                    callbackHandle.Free();
                }
                Marshal.FreeHGlobal(methodPtr);
                Marshal.FreeHGlobal(binaryPtr);
            }
        }

        public void Dispose()
        {
            DumpHostProfile();
            if (_libraryHandle != IntPtr.Zero)
            {
                NativeLibrary.Free(_libraryHandle);
                _libraryHandle = IntPtr.Zero;
            }
        }

        internal static RiscvExecutionResult? TryChargeExecutionFee(ApplicationEngine engine, long feeConsumedPico, string operation)
        {
            if (engine is null) throw new ArgumentNullException(nameof(engine));
            if (feeConsumedPico <= 0)
                return null;

            try
            {
                engine.AddFee(feeConsumedPico);
                return null;
            }
            catch (InvalidOperationException gasEx) when (gasEx.Message.Contains("Insufficient GAS", StringComparison.Ordinal))
            {
                Trace($"{operation} gas exhaustion: {gasEx.Message}");
                return new RiscvExecutionResult(VMState.FAULT, System.Array.Empty<StackItem>(), gasEx);
            }
        }
    }
}
