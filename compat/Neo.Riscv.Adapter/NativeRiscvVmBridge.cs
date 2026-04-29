// Copyright (C) 2015-2026 The Neo Project.
//
// NativeRiscvVmBridge.cs file belongs to the neo project and is free
// software distributed under the MIT software license, see the
// accompanying file LICENSE in the main directory of the
// repository or http://www.opensource.org/licenses/mit-license.php
// for more details.

using Neo.VM;
using Neo.VM.Types;
using Neo.Extensions;
using Neo.SmartContract;
using Neo.SmartContract.Manifest;
using Neo.SmartContract.Iterators;
using Neo.SmartContract.Native;
using Neo.Cryptography.ECC;
using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Diagnostics;
using System.Linq;
using System.Numerics;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;

namespace Neo.SmartContract.RiscV
{
    public sealed partial class NativeRiscvVmBridge : IRiscvVmBridge, IDisposable
    {
        public const string LibraryPathEnvironmentVariable = "NEO_RISCV_HOST_LIB";
        private const string TraceEnvironmentVariable = "NEO_RISCV_TRACE_HOST";
        private const string ProfileEnvironmentVariable = "NEO_RISCV_PROFILE_HOST";
        private static readonly ConcurrentDictionary<uint, HostProfileStat> HostProfileStats = new();
        private static readonly ConcurrentDictionary<string, PhaseProfileStat> CallContractPhaseStats = new();
        private static int s_profileDumped;

        [StructLayout(LayoutKind.Sequential)]
        private struct NativeExecutionResult
        {
            public long FeeConsumedPico;
            public uint State;
            public IntPtr StackPtr;
            public nuint StackLen;
            public IntPtr ErrorPtr;
            public nuint ErrorLen;
        }

        [StructLayout(LayoutKind.Sequential)]
        private struct NativeStackItem
        {
            public uint Kind;
            public long IntegerValue;
            public IntPtr BytesPtr;
            public nuint BytesLen;
        }

        [StructLayout(LayoutKind.Sequential)]
        private struct NativeHostResult
        {
            public IntPtr StackPtr;
            public nuint StackLen;
            public IntPtr ErrorPtr;
            public nuint ErrorLen;
        }

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        [return: MarshalAs(UnmanagedType.I1)]
        private delegate bool ExecuteNativeContractDelegate(
            IntPtr binaryPtr,
            nuint binaryLen,
            IntPtr methodPtr,
            nuint methodLen,
            IntPtr initialStackPtr,
            nuint initialStackLen,
            byte trigger,
            uint network,
            byte addressVersion,
            ulong timestamp,
            long gasLeft,
            long execFeeFactorPico,
            IntPtr userData,
            IntPtr hostCallback,
            IntPtr hostFree,
            out NativeExecutionResult result);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate uint LastFaultIpDelegate();

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate nuint LastFaultLocalsDelegate(IntPtr outPtr, nuint outCapacity);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        [return: MarshalAs(UnmanagedType.I1)]
        private delegate bool ExecuteScriptDelegate(
            IntPtr scriptPtr,
            nuint scriptLen,
            nuint initialInstructionPointer,
            byte trigger,
            uint networkMagic,
            byte addressVersion,
            ulong persistingTimestamp,
            long gasLeft,
            long execFeeFactorPico,
            IntPtr initialStackPtr,
            nuint initialStackLen,
            IntPtr userData,
            IntPtr hostCallback,
            IntPtr hostFree,
            out NativeExecutionResult result);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        [return: MarshalAs(UnmanagedType.I1)]
        private delegate bool HostCallbackDelegate(
            IntPtr userData,
            uint api,
            nuint instructionPointer,
            byte trigger,
            uint networkMagic,
            byte addressVersion,
            ulong persistingTimestamp,
            long gasLeft,
            IntPtr inputStackPtr,
            nuint inputStackLen,
            out NativeHostResult result);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate void HostFreeCallbackDelegate(IntPtr userData, ref NativeHostResult result);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate void FreeExecutionResultDelegate(ref NativeExecutionResult result);

        private sealed class HostCallbackState
        {
            public required NativeRiscvVmBridge Bridge { get; init; }

            public required RiscvExecutionRequest Request { get; init; }

            public required ExecutionScope Scope { get; init; }
        }

        private sealed class ExecutionScope
        {
            public Script CurrentScript { get; set; } = Script.Empty;

            public Dictionary<ulong, IIterator> Iterators { get; } = new();

            public Dictionary<ulong, object> InteropObjects { get; } = new();

            public Dictionary<ContractCallCacheKey, ContractCallCacheEntry> ContractCallCache { get; } = new();

            public ulong NextIteratorHandle { get; set; } = 1;

            public ulong NextInteropHandle { get; set; } = 1;
        }

        private IntPtr _libraryHandle;
        private readonly ExecuteScriptDelegate _executeScript;
        private readonly ExecuteNativeContractDelegate _executeNativeContract;
        private readonly FreeExecutionResultDelegate _freeExecutionResult;
        private readonly LastFaultIpDelegate? _lastFaultIp;
        private readonly LastFaultLocalsDelegate? _lastFaultLocals;
        private readonly HostCallbackDelegate _hostCallback;
        private readonly HostFreeCallbackDelegate _hostFreeCallback;
        private readonly IntPtr _hostCallbackPtr;
        private readonly IntPtr _hostFreeCallbackPtr;
        private const CallFlags CallTRequiredCallFlags = CallFlags.ReadStates | CallFlags.AllowCall;
        private const int CachedSmallIntMin = -1;
        private const int CachedSmallIntMax = 8;
        private static readonly IntPtr CachedNullStackPtr = CreateCachedSingleStackItem(2, 0);
        private static readonly IntPtr CachedBoolTrueStackPtr = CreateCachedSingleStackItem(3, 1);
        private static readonly IntPtr CachedBoolFalseStackPtr = CreateCachedSingleStackItem(3, 0);
        private static readonly IntPtr CachedIntZeroStackPtr = CreateCachedSingleStackItem(0, 0);
        private static readonly IntPtr[] CachedSmallIntStackPtrs = CreateCachedSmallIntStackPtrs();

        private static bool TraceEnabled =>
            string.Equals(Environment.GetEnvironmentVariable(TraceEnvironmentVariable), "1", StringComparison.Ordinal);

        private static bool ProfileEnabled =>
            string.Equals(Environment.GetEnvironmentVariable(ProfileEnvironmentVariable), "1", StringComparison.Ordinal);

        private sealed class HostProfileStat
        {
            public required string Name { get; init; }
            public long Count;
            public long InputItems;
            public long ReadStackTicks;
            public long HandleTicks;
        }

        private sealed class PhaseProfileStat
        {
            public long Count;
            public long Ticks;
        }

        private readonly record struct ContractCallCacheKey(
            UInt160 ContractHash,
            string Method,
            int ParameterCount);

        private sealed record ContractCallCacheEntry(
            ContractState ContractState,
            ContractMethodDescriptor Descriptor,
            bool HasReturnValue);

        public NativeRiscvVmBridge(string libraryPath)
        {
            if (string.IsNullOrWhiteSpace(libraryPath))
                throw new ArgumentException("Library path is required.", nameof(libraryPath));

            _hostCallback = StaticHostCallback;
            _hostFreeCallback = StaticHostFreeCallback;
            _hostCallbackPtr = Marshal.GetFunctionPointerForDelegate(_hostCallback);
            _hostFreeCallbackPtr = Marshal.GetFunctionPointerForDelegate(_hostFreeCallback);

            _libraryHandle = NativeLibrary.Load(libraryPath);
            var executeExport = NativeLibrary.GetExport(_libraryHandle, "neo_riscv_execute_script_with_host");
            var nativeContractExport = NativeLibrary.GetExport(_libraryHandle, "neo_riscv_execute_native_contract");
            var freeExport = NativeLibrary.GetExport(_libraryHandle, "neo_riscv_free_execution_result");
            _executeScript = Marshal.GetDelegateForFunctionPointer<ExecuteScriptDelegate>(executeExport);
            _executeNativeContract = Marshal.GetDelegateForFunctionPointer<ExecuteNativeContractDelegate>(nativeContractExport);
            _freeExecutionResult = Marshal.GetDelegateForFunctionPointer<FreeExecutionResultDelegate>(freeExport);
            // Optional side-channel FFI: if the loaded library predates the fault-IP export,
            // leave the delegate null and the adapter simply won't surface fault IPs.
            if (NativeLibrary.TryGetExport(_libraryHandle, "neo_riscv_last_fault_ip", out var lastFaultIpExport))
            {
                _lastFaultIp = Marshal.GetDelegateForFunctionPointer<LastFaultIpDelegate>(lastFaultIpExport);
            }
            if (NativeLibrary.TryGetExport(_libraryHandle, "neo_riscv_last_fault_locals", out var lastFaultLocalsExport))
            {
                _lastFaultLocals = Marshal.GetDelegateForFunctionPointer<LastFaultLocalsDelegate>(lastFaultLocalsExport);
            }
        }

        /// <summary>
        /// Fetches the fast-codec-serialized locals snapshot of the most recent FAULT on
        /// this thread. Returns an empty array if unavailable (no snapshot captured or
        /// missing FFI export). The caller is expected to pass the bytes to
        /// <c>fast_codec.decode_stack</c>-equivalent decoding on the C# side.
        /// </summary>
        internal byte[] TryReadLastFaultLocals()
        {
            if (_lastFaultLocals is null) return System.Array.Empty<byte>();
            var len = (int)_lastFaultLocals(IntPtr.Zero, 0);
            if (len == 0) return System.Array.Empty<byte>();
            var buffer = new byte[len];
            var handle = GCHandle.Alloc(buffer, GCHandleType.Pinned);
            try
            {
                _lastFaultLocals(handle.AddrOfPinnedObject(), (nuint)len);
                return buffer;
            }
            finally
            {
                handle.Free();
            }
        }

        /// <summary>
        /// Fetches the instruction pointer of the most recent FAULT on this thread from the
        /// Rust-side thread-local, or null if unavailable (sentinel <c>uint.MaxValue</c> or
        /// missing FFI export).
        /// </summary>
        internal int? TryReadLastFaultIp()
        {
            if (_lastFaultIp is null) return null;
            var ip = _lastFaultIp();
            return ip == uint.MaxValue ? null : (int)ip;
        }



        private static StackItem[] HandleCryptoCheckSig(RiscvExecutionRequest request, StackItem[] inputStack)
        {
            if (inputStack.Length < 2)
                throw new InvalidOperationException("CheckSig requires pubkey and signature.");

            if (inputStack[^1] is not ByteString pubkey)
                throw new InvalidOperationException("CheckSig requires a byte string public key.");
            if (inputStack[^2] is not ByteString signature)
                throw new InvalidOperationException("CheckSig requires a byte string signature.");

            var next = new StackItem[inputStack.Length - 1];
            if (inputStack.Length > 2)
            {
                System.Array.Copy(inputStack, next, inputStack.Length - 2);
            }
            next[^1] = request.Engine.CheckSig(pubkey.GetSpan().ToArray(), signature.GetSpan().ToArray()) ? StackItem.True : StackItem.False;
            return next;
        }

        private static StackItem[] HandleCryptoCheckMultisig(RiscvExecutionRequest request, StackItem[] inputStack)
        {
            if (inputStack.Length < 2)
                throw new InvalidOperationException("CheckMultisig requires pubkeys and signatures.");

            byte[][] pubKeys;
            byte[][] signatures;
            var consumed = 0;

            if (inputStack[^1] is Neo.VM.Types.Array pubKeysArray && inputStack[^2] is Neo.VM.Types.Array signaturesArray)
            {
                pubKeys = new byte[pubKeysArray.Count][];
                for (var index = 0; index < pubKeysArray.Count; index++)
                {
                    if (pubKeysArray[index] is not ByteString pubKey)
                        throw new InvalidOperationException("CheckMultisig public keys must be byte strings.");
                    pubKeys[index] = pubKey.GetSpan().ToArray();
                }

                signatures = new byte[signaturesArray.Count][];
                for (var index = 0; index < signaturesArray.Count; index++)
                {
                    if (signaturesArray[index] is not ByteString signature)
                        throw new InvalidOperationException("CheckMultisig signatures must be byte strings.");
                    signatures[index] = signature.GetSpan().ToArray();
                }

                consumed = 2;
            }
            else
            {
                if (inputStack[^1] is not Integer pubKeyCountItem)
                    throw new InvalidOperationException("CheckMultisig requires a public key count.");

                var pubKeyCount = checked((int)pubKeyCountItem.GetInteger());
                if (pubKeyCount <= 0 || inputStack.Length < pubKeyCount + 2)
                    throw new InvalidOperationException("CheckMultisig public key count is invalid.");

                var signatureCountIndex = inputStack.Length - 2 - pubKeyCount;
                if (signatureCountIndex < 0 || inputStack[signatureCountIndex] is not Integer signatureCountItem)
                    throw new InvalidOperationException("CheckMultisig requires a signature count.");

                var signatureCount = checked((int)signatureCountItem.GetInteger());
                if (signatureCount <= 0 || signatureCountIndex < signatureCount)
                    throw new InvalidOperationException("CheckMultisig signature count is invalid.");

                pubKeys = new byte[pubKeyCount][];
                for (var index = 0; index < pubKeyCount; index++)
                {
                    if (inputStack[inputStack.Length - 2 - index] is not ByteString pubKey)
                        throw new InvalidOperationException("CheckMultisig public keys must be byte strings.");
                    pubKeys[pubKeyCount - index - 1] = pubKey.GetSpan().ToArray();
                }

                signatures = new byte[signatureCount][];
                for (var index = 0; index < signatureCount; index++)
                {
                    if (inputStack[signatureCountIndex - index - 1] is not ByteString signature)
                        throw new InvalidOperationException("CheckMultisig signatures must be byte strings.");
                    signatures[signatureCount - index - 1] = signature.GetSpan().ToArray();
                }

                consumed = pubKeyCount + signatureCount + 2;
            }

            var next = new StackItem[inputStack.Length - consumed + 1];
            if (inputStack.Length > consumed)
            {
                System.Array.Copy(inputStack, next, inputStack.Length - consumed);
            }
            next[^1] = request.Engine.CheckMultisig(pubKeys, signatures) ? StackItem.True : StackItem.False;
            return next;
        }

        private static StackItem[] HandleNativeOnPersist(RiscvExecutionRequest request, StackItem[] inputStack)
        {
            if (request.Trigger != TriggerType.OnPersist)
                throw new InvalidOperationException();

            foreach (var contract in NativeContract.Contracts)
            {
                if (!contract.IsActive(request.Engine.ProtocolSettings, request.Engine.PersistingBlock!.Index))
                    continue;

                CompleteContractTask(contract.OnPersistAsync(request.Engine), "NativeOnPersist");
            }
            return inputStack;
        }

        private static StackItem[] HandleNativePostPersist(RiscvExecutionRequest request, StackItem[] inputStack)
        {
            if (request.Trigger != TriggerType.PostPersist)
                throw new InvalidOperationException();

            foreach (var contract in NativeContract.Contracts)
            {
                if (!contract.IsActive(request.Engine.ProtocolSettings, request.Engine.PersistingBlock!.Index))
                    continue;

                CompleteContractTask(contract.PostPersistAsync(request.Engine), "NativePostPersist");
            }
            return inputStack;
        }

        private static void CompleteContractTask(ContractTask task, string operation)
        {
            if (!task.GetAwaiter().IsCompleted)
                throw new InvalidOperationException($"{operation} returned an incomplete asynchronous native task.");

            task.GetResult();
        }

        internal static uint ResolveNativeContractSnapshotIndex(ApplicationEngine engine)
        {
            if (engine.PersistingBlock is not null)
                return engine.PersistingBlock.Index;

            try
            {
                return NativeContract.Ledger.CurrentIndex(engine.SnapshotCache);
            }
            catch (KeyNotFoundException)
            {
                return 0;
            }
        }

        private static StackItem[] HandleContractCallNative(RiscvExecutionRequest request, ExecutionScope scope, nuint instructionPointer, StackItem[] inputStack)
        {
            if (inputStack.Length == 0)
                throw new InvalidOperationException("Contract.CallNative requires a version.");

            if (inputStack[^1] is not Integer versionItem)
                throw new InvalidOperationException("Contract.CallNative requires an integer version.");

            var currentContract = NativeContract.GetContract(request.ScriptHashes[^1])
                ?? throw new InvalidOperationException("It is not allowed to use \"System.Contract.CallNative\" directly.");
            if (!currentContract.IsActive(request.Engine.ProtocolSettings, ResolveNativeContractSnapshotIndex(request.Engine)))
                throw new InvalidOperationException($"The native contract {currentContract.Name} is not active.");

            var versionRaw = versionItem.GetInteger();
            if (versionRaw != 0)
                throw new InvalidOperationException($"The native contract of version {versionRaw} is not active.");

            var currentAllowedMethods = currentContract.GetContractMethods(request.Engine);
            if (!currentAllowedMethods.TryGetValue(checked((int)instructionPointer), out var method))
                throw new InvalidOperationException($"No native method is available at instruction pointer {instructionPointer} for contract {currentContract.Name}.");
            if (method.ActiveIn is not null && !request.Engine.IsHardforkEnabled(method.ActiveIn.Value))
                throw new InvalidOperationException($"Cannot call this method before hardfork {method.ActiveIn}.");
            if (method.DeprecatedIn is not null && request.Engine.IsHardforkEnabled(method.DeprecatedIn.Value))
                throw new InvalidOperationException($"Cannot call this method after hardfork {method.DeprecatedIn}.");
            if (!request.CurrentCallFlags.HasFlag(method.RequiredCallFlags))
                throw new InvalidOperationException($"Cannot call this method with the flag {request.CurrentCallFlags}.");

            if (!request.Engine.IsHardforkEnabled(Hardfork.HF_Faun) ||
                !NativeContract.Policy.IsWhitelistFeeContract(request.Engine.SnapshotCache, currentContract.Hash, method.Descriptor, out var fixedFee))
            {
                request.Engine.AddFee(
                    (method.CpuFee * request.Engine.ExecFeePicoFactor) +
                    (method.StorageFee * request.Engine.StoragePrice * ApplicationEngine.FeeFactor));
            }
            else
            {
                request.Engine.AddFee(fixedFee!.Value * ApplicationEngine.FeeFactor);
            }

            var parameterCount = method.Parameters.Length;
            if (inputStack.Length < parameterCount + 1)
                throw new InvalidOperationException($"Native contract method \"{method.Descriptor.Name}\" expects {parameterCount} argument(s).");

            var parameters = new List<object?>();
            if (method.NeedApplicationEngine) parameters.Add(request.Engine);
            if (method.NeedSnapshot) parameters.Add(request.Engine.SnapshotCache);
            for (var index = 0; index < parameterCount; index++)
            {
                parameters.Add(request.Engine.Convert(inputStack[inputStack.Length - 2 - index], method.Parameters[index]));
            }

            var currentContractState = NativeContract.ContractManagement.GetContract(request.Engine.SnapshotCache, currentContract.Hash)
                ?? currentContract.GetContractState(request.Engine.ProtocolSettings, ResolveNativeContractSnapshotIndex(request.Engine));

            object? returnValue = request.Engine.ExecuteInNativeContractContext(
                currentContract.Hash,
                request.ScriptHashes.Count > 1 ? request.ScriptHashes[^2] : null,
                currentContractState,
                request.CurrentCallFlags,
                () => method.Handler.Invoke(currentContract, parameters.ToArray()));
            Trace($"callnative raw-return method={method.Descriptor.Name} returnType={method.Descriptor.ReturnType} valueType={returnValue?.GetType().FullName ?? "null"}");
            if (returnValue is ContractTask task)
            {
                if (!task.GetAwaiter().IsCompleted)
                {
                    // Execute pending user contract contexts so ContextUnloaded completes the task.
                    request.Engine.Execute();
                }
                returnValue = task.GetResult();
                Trace($"callnative task-result method={method.Descriptor.Name} valueType={returnValue?.GetType().FullName ?? "null"}");
            }

            var returnedStack = method.Descriptor.ReturnType == ContractParameterType.Void
                ? System.Array.Empty<StackItem>()
                : new[] { request.Engine.Convert(returnValue) };
            Trace($"callnative pushed method={method.Descriptor.Name} count={returnedStack.Length}");
            var next = BuildContractCallReturnStack(inputStack, parameterCount + 1, method.Descriptor.ReturnType, returnedStack);
            if (currentContract.Hash == NativeContract.ContractManagement.Hash)
                scope.ContractCallCache.Clear();
            return next;
        }

        private static StackItem[] HandleCreateStandardAccount(RiscvExecutionRequest request, StackItem[] inputStack)
        {
            if (inputStack.Length == 0)
                throw new InvalidOperationException("CreateStandardAccount requires a public key.");

            if (inputStack[^1] is not ByteString pubKeyBytes)
                throw new InvalidOperationException("CreateStandardAccount requires a byte string public key.");

            var pubKey = ECPoint.DecodePoint(pubKeyBytes.GetSpan().ToArray(), ECCurve.Secp256r1);
            var next = new StackItem[inputStack.Length];
            if (inputStack.Length > 1)
            {
                System.Array.Copy(inputStack, next, inputStack.Length - 1);
            }
            next[^1] = new ByteString(request.Engine.CreateStandardAccount(pubKey).GetSpan().ToArray());
            return next;
        }

        private static StackItem[] HandleCreateMultisigAccount(RiscvExecutionRequest request, StackItem[] inputStack)
        {
            if (inputStack.Length < 2)
                throw new InvalidOperationException("CreateMultisigAccount requires m and public keys.");

            if (inputStack[^1] is not Integer mValue)
                throw new InvalidOperationException("CreateMultisigAccount requires integer m.");
            if (inputStack[^2] is not Integer countValue)
                throw new InvalidOperationException("CreateMultisigAccount requires integer public key count.");

            var count = (int)countValue.GetInteger();
            if (count < 0 || inputStack.Length < count + 2)
                throw new InvalidOperationException("CreateMultisigAccount public key count is invalid.");

            var pubKeys = new ECPoint[count];
            for (var index = 0; index < count; index++)
            {
                var item = inputStack[inputStack.Length - 3 - index];
                if (item is not ByteString pubKeyBytes)
                    throw new InvalidOperationException("CreateMultisigAccount requires byte string public keys.");
                pubKeys[count - index - 1] = ECPoint.DecodePoint(pubKeyBytes.GetSpan().ToArray(), ECCurve.Secp256r1);
            }

            var next = new StackItem[inputStack.Length - count - 1];
            if (next.Length > 0)
            {
                System.Array.Copy(inputStack, next, next.Length - 1);
            }
            next[^1] = new ByteString(request.Engine.CreateMultisigAccount((int)mValue.GetInteger(), pubKeys).GetSpan().ToArray());
            return next;
        }

        private StackItem[] HandleCallT(RiscvExecutionRequest request, ExecutionScope scope, long gasLeft, ushort token, StackItem[] inputStack)
        {
            ValidateCallTCallFlags(request.CurrentCallFlags);

            // Resolve the method token from the current contract's NEF.
            var currentHash = request.ScriptHashes[^1];
            var contractState = NativeContract.ContractManagement.GetContract(request.Engine.SnapshotCache, currentHash)
                ?? throw new InvalidOperationException($"CALLT: current contract {currentHash} not found in storage.");
            if (token >= contractState.Nef.Tokens.Length)
                throw new InvalidOperationException($"CALLT: token {token} out of range (contract {currentHash} has {contractState.Nef.Tokens.Length} tokens).");

            var methodToken = contractState.Nef.Tokens[token];
            Trace($"callt enter token={token} target={methodToken.Hash} method={methodToken.Method} params={methodToken.ParametersCount} callFlags={methodToken.CallFlags}");

            // NOTE: CALLT opcode fee is already charged by the guest via host_on_instruction.
            // Do NOT call AddFee here — that would double-charge.

            // Pop the method's parameter count from the stack and pack into an Array.
            if (inputStack.Length < methodToken.ParametersCount)
                throw new InvalidOperationException($"CALLT: stack has {inputStack.Length} items but method {methodToken.Method} expects {methodToken.ParametersCount} parameters.");

            var argsArray = new Neo.VM.Types.Array();
            for (var i = 0; i < methodToken.ParametersCount; i++)
                argsArray.Add(inputStack[inputStack.Length - 1 - i]);

            // Build the equivalent System.Contract.Call stack: [remaining..., argsArray, callFlags, method, hash]
            var callStack = new StackItem[inputStack.Length - methodToken.ParametersCount + 4];
            if (inputStack.Length > methodToken.ParametersCount)
                System.Array.Copy(inputStack, callStack, inputStack.Length - methodToken.ParametersCount);

            callStack[^4] = argsArray;
            callStack[^3] = new Integer((int)methodToken.CallFlags);
            callStack[^2] = new ByteString(Encoding.UTF8.GetBytes(methodToken.Method));
            callStack[^1] = new ByteString(methodToken.Hash.GetSpan().ToArray());

            return HandleContractCall(request, scope, gasLeft, callStack, methodToken.HasReturnValue);
        }

        internal static void ValidateCallTForTesting(
            CallFlags currentCallFlags,
            bool tokenHasReturnValue,
            ContractParameterType returnType)
        {
            ValidateCallTCallFlags(currentCallFlags);
            ValidateCallTReturnValue(tokenHasReturnValue, returnType);
        }

        private static void ValidateCallTCallFlags(CallFlags currentCallFlags)
        {
            if (!currentCallFlags.HasFlag(CallTRequiredCallFlags))
                throw new InvalidOperationException($"Cannot call this SYSCALL with the flag {currentCallFlags}.");
        }

        private static void ValidateCallTReturnValue(bool tokenHasReturnValue, ContractParameterType returnType)
        {
            if (tokenHasReturnValue != (returnType != ContractParameterType.Void))
                throw new InvalidOperationException("The return value type does not match.");
        }

        private StackItem[] HandleContractCall(
            RiscvExecutionRequest request,
            ExecutionScope scope,
            long gasLeft,
            StackItem[] inputStack,
            bool? expectedHasReturnValue = null)
        {
            var phaseStart = ProfileEnabled ? Stopwatch.GetTimestamp() : 0;
            if (inputStack.Length < 4)
                throw new InvalidOperationException("Contract.Call requires hash, method, call flags, and args.");

            if (inputStack[^1] is not ByteString hashBytes || hashBytes.GetSpan().Length != UInt160.Length)
                throw new InvalidOperationException("Contract.Call requires a contract hash.");
            if (inputStack[^2] is not ByteString methodBytes)
                throw new InvalidOperationException("Contract.Call requires a method name.");
            if (inputStack[^3] is not Integer callFlagsItem)
                throw new InvalidOperationException("Contract.Call requires integer call flags.");
            if (inputStack[^4] is not Neo.VM.Types.Array argsArray)
                throw new InvalidOperationException("Contract.Call requires an argument array.");

            var contractHash = new UInt160(hashBytes.GetSpan());
            var method = methodBytes.GetString() ?? throw new InvalidOperationException("Method name must be valid UTF-8.");
            var callFlags = (CallFlags)(byte)callFlagsItem.GetInteger();
            if ((callFlags & ~CallFlags.All) != 0)
                throw new InvalidOperationException($"Invalid call flags: {callFlags}");
            if (ProfileEnabled)
            {
                RecordCallContractPhase("decode_input", Stopwatch.GetTimestamp() - phaseStart);
                phaseStart = Stopwatch.GetTimestamp();
            }
            Trace($"contract.call enter hash={contractHash} method={method} stackLen={inputStack.Length} args={argsArray.Count}");

            if (expectedHasReturnValue is null &&
                TryInvokeTestingCustomMock(request, contractHash, method, argsArray, out var mockedResult))
            {
                var next = new StackItem[inputStack.Length - 3];
                if (inputStack.Length > 4)
                    System.Array.Copy(inputStack, next, inputStack.Length - 4);
                next[^1] = mockedResult;
                return next;
            }

            var skipCache = contractHash == NativeContract.ContractManagement.Hash;
            var cacheKey = new ContractCallCacheKey(contractHash, method, argsArray.Count);
            ContractCallCacheEntry? cacheEntry = null;
            if (!skipCache && !scope.ContractCallCache.TryGetValue(cacheKey, out cacheEntry))
            {
                var contractState = NativeContract.ContractManagement.GetContract(request.Engine.SnapshotCache, contractHash)
                    ?? NativeContract.GetContract(contractHash)?.GetContractState(request.Engine.ProtocolSettings, ResolveNativeContractSnapshotIndex(request.Engine));
                if (contractState == null)
                    throw new InvalidOperationException($"Called Contract Does Not Exist: {contractHash}.{method}");
                var methodDescriptor = contractState.Manifest.Abi.GetMethod(method, argsArray.Count)
                    ?? throw new InvalidOperationException($"Method \"{method}\" with {argsArray.Count} parameter(s) doesn't exist in the contract {contractHash}.");
                cacheEntry = new ContractCallCacheEntry(
                    contractState,
                    methodDescriptor,
                    methodDescriptor.ReturnType != ContractParameterType.Void);
                scope.ContractCallCache[cacheKey] = cacheEntry;
            }

            if (skipCache)
            {
                var contractState = NativeContract.ContractManagement.GetContract(request.Engine.SnapshotCache, contractHash)
                    ?? NativeContract.GetContract(contractHash)?.GetContractState(request.Engine.ProtocolSettings, ResolveNativeContractSnapshotIndex(request.Engine));
                if (contractState == null)
                    throw new InvalidOperationException($"Called Contract Does Not Exist: {contractHash}.{method}");
                var methodDescriptor = contractState.Manifest.Abi.GetMethod(method, argsArray.Count)
                    ?? throw new InvalidOperationException($"Method \"{method}\" with {argsArray.Count} parameter(s) doesn't exist in the contract {contractHash}.");
                cacheEntry = new ContractCallCacheEntry(
                    contractState,
                    methodDescriptor,
                    methodDescriptor.ReturnType != ContractParameterType.Void);
            }

            cacheEntry ??= scope.ContractCallCache[cacheKey];
            if (expectedHasReturnValue.HasValue)
            {
                ValidateCallTReturnValue(expectedHasReturnValue.Value, cacheEntry.Descriptor.ReturnType);
                if (TryInvokeTestingCustomMock(request, contractHash, method, argsArray, out var mockedCallTResult))
                {
                    var mockedResultStack = cacheEntry.HasReturnValue
                        ? new[] { mockedCallTResult }
                        : System.Array.Empty<StackItem>();
                    return BuildContractCallReturnStack(inputStack, 4, cacheEntry.Descriptor.ReturnType, mockedResultStack);
                }
            }

            if (ProfileEnabled)
            {
                RecordCallContractPhase("resolve_contract", Stopwatch.GetTimestamp() - phaseStart);
                phaseStart = Stopwatch.GetTimestamp();
            }

            var result = HandleContractCallViaRiscv(
                request,
                scope,
                gasLeft,
                inputStack,
                cacheEntry.ContractState,
                cacheEntry.Descriptor,
                callFlags,
                argsArray);

            if (skipCache)
                scope.ContractCallCache.Clear();

            if (ProfileEnabled)
                RecordCallContractPhase("nested_riscv_execute", Stopwatch.GetTimestamp() - phaseStart);

            return result;
        }

        private static bool TryInvokeTestingCustomMock(
            RiscvExecutionRequest request,
            UInt160 contractHash,
            string method,
            Neo.VM.Types.Array argsArray,
            out StackItem result)
        {
            result = StackItem.Null;
            var hooks = GetTestingHooks(request.Engine);
            if (hooks is null)
                return false;

            var args = new StackItem[argsArray.Count];
            for (var index = 0; index < argsArray.Count; index++)
                args[index] = argsArray[index];

            var snapshot = request.Engine.CurrentContext?.GetState<ExecutionContextState>().SnapshotCache
                ?? request.Engine.SnapshotCache;
            return hooks.TryInvokeCustomMock(request.Engine, snapshot, contractHash, method, args, out result);
        }

        private static string DescribeStackItem(StackItem item, int depth = 0)
        {
            if (depth >= 2)
                return item.GetType().Name;

            return item switch
            {
                ByteString bytes => $"bytes:{Convert.ToHexString(bytes.GetSpan())}",
                Integer integer => $"int:{integer.GetInteger()}",
                Neo.VM.Types.Boolean boolean => $"bool:{boolean.GetBoolean()}",
                Null => "null",
                Neo.VM.Types.Struct @struct => $"struct:{@struct.Count}[{string.Join(",", Enumerable.Range(0, @struct.Count).Select(i => DescribeStackItem(@struct[i], depth + 1)))}]",
                Neo.VM.Types.Array array => $"array:{array.Count}[{string.Join(",", Enumerable.Range(0, array.Count).Select(i => DescribeStackItem(array[i], depth + 1)))}]",
                Neo.VM.Types.Map map => $"map:{map.Count}",
                InteropInterface => "interop",
                _ => item.GetType().Name,
            };
        }
    }
}
