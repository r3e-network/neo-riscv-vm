using Neo.VM;
using Neo.VM.Types;
using System;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text;

namespace Neo.SmartContract.RiscV
{
    public sealed partial class NativeRiscvVmBridge
    {
        private static bool StaticHostCallback(
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
            out NativeHostResult result)
        {
            result = default;
            try
            {
                Trace($"callback raw-enter api=0x{api:x8} ip={instructionPointer} inputStackLen={inputStackLen}");
                var handle = GCHandle.FromIntPtr(userData);
                var state = (HostCallbackState)handle.Target!;
                StackItem[] inputStack;
                long readStackTicks = 0;
                try
                {
                    var readStart = ProfileEnabled ? Stopwatch.GetTimestamp() : 0;
                    inputStack = state.Bridge.ReadStack(inputStackPtr, inputStackLen, state.Request.Engine.ReferenceCounter, state.Scope, decodeStorageContextTokens: false);
                    if (ProfileEnabled)
                        readStackTicks = Stopwatch.GetTimestamp() - readStart;
                    Trace($"callback read-stack api=0x{api:x8} managedStackLen={inputStack.Length}");
                }
                catch (Exception ex)
                {
                    Trace($"callback read-stack fault api=0x{api:x8} type={ex.GetType().FullName} message={ex.Message}");
                    result = CreateNativeHostError(ex);
                    return true;
                }

                var handleStart = ProfileEnabled ? Stopwatch.GetTimestamp() : 0;
                var handled = state.Bridge.HandleHostCallback(state.Request, state.Scope, api, instructionPointer, gasLeft, inputStack, out result);
                if (ProfileEnabled)
                {
                    var descriptor = ApplicationEngine.GetInteropDescriptor(api);
                    RecordHostProfile(api, descriptor.Name, inputStack.Length, readStackTicks, Stopwatch.GetTimestamp() - handleStart);
                }
                return handled;
            }
            catch (Exception ex)
            {
                Trace($"callback outer fault api=0x{api:x8} type={ex.GetType().FullName} message={ex.Message}");
                result = CreateNativeHostError(ex);
                return true;
            }
        }

        private const uint CalltMarkerHi = 0x4354;

        private bool HandleHostCallback(RiscvExecutionRequest request, ExecutionScope scope, uint api, nuint instructionPointer, long gasLeft, StackItem[] inputStack, out NativeHostResult result)
        {
            try
            {
                if ((api >> 16) == CalltMarkerHi)
                {
                    var calltToken = (ushort)(api & 0xFFFF);
                    var calltGasLeft = gasLeft - (request.GasLeft - request.Engine.GasLeft);
                    var calltResult = HandleCallT(request, scope, calltGasLeft, calltToken, inputStack);
                    result = CreateNativeHostResult(calltResult, scope);
                    return true;
                }

                var descriptor = ApplicationEngine.GetInteropDescriptor(api);
                Trace($"syscall enter name={descriptor.Name} api=0x{api:x8} ip={instructionPointer} gasLeft={gasLeft} stackLen={inputStack.Length}");
                if (!request.CurrentCallFlags.HasFlag(descriptor.RequiredCallFlags))
                    throw new InvalidOperationException($"Cannot call this SYSCALL with the flag {request.CurrentCallFlags}.");
                if (descriptor.FixedPrice != 0)
                {
                    request.Engine.AddFee(descriptor.FixedPrice * request.Engine.ExecFeePicoFactor);
                }
                var effectiveGasLeft = gasLeft - (request.GasLeft - request.Engine.GasLeft);

                var stack = api switch
                {
                    uint hash when hash == ApplicationEngine.System_Runtime_Platform =>
                        Append(inputStack, new ByteString(Encoding.UTF8.GetBytes("NEO"))),
                    uint hash when hash == ApplicationEngine.System_Runtime_GetTrigger =>
                        Append(inputStack, new Integer((int)request.Trigger)),
                    uint hash when hash == ApplicationEngine.System_Runtime_GetNetwork =>
                        Append(inputStack, new Integer(request.NetworkMagic)),
                    uint hash when hash == ApplicationEngine.System_Runtime_GetAddressVersion =>
                        Append(inputStack, new Integer(request.AddressVersion)),
                    uint hash when hash == ApplicationEngine.System_Runtime_GasLeft =>
                        Append(inputStack, new Integer(effectiveGasLeft)),
                    uint hash when hash == ApplicationEngine.System_Runtime_GetRandom =>
                        Append(inputStack, new Integer(request.Engine.GetRandom())),
                    uint hash when hash == ApplicationEngine.System_Runtime_GetScriptContainer =>
                        Append(inputStack, CreateScriptContainerItem(request)),
                    uint hash when hash == ApplicationEngine.System_Storage_GetContext =>
                        Append(inputStack, CreateStorageContextItem(request.Engine.GetStorageContext())),
                    uint hash when hash == ApplicationEngine.System_Storage_GetReadOnlyContext =>
                        Append(inputStack, CreateStorageContextItem(request.Engine.GetReadOnlyContext())),
                    uint hash when hash == ApplicationEngine.System_Storage_AsReadOnly =>
                        HandleStorageAsReadOnly(request, inputStack),
                    uint hash when hash == ApplicationEngine.System_Storage_Local_Get =>
                        HandleStorageLocalGet(request, inputStack),
                    uint hash when hash == ApplicationEngine.System_Storage_Local_Find =>
                        HandleStorageLocalFind(request, inputStack),
                    uint hash when hash == ApplicationEngine.System_Storage_Local_Put =>
                        HandleStorageLocalPut(request, inputStack),
                    uint hash when hash == ApplicationEngine.System_Storage_Local_Delete =>
                        HandleStorageLocalDelete(request, inputStack),
                    uint hash when hash == ApplicationEngine.System_Storage_Find =>
                        HandleStorageFind(request, inputStack),
                    uint hash when hash == ApplicationEngine.System_Storage_Get =>
                        HandleStorageGet(request, inputStack),
                    uint hash when hash == ApplicationEngine.System_Storage_Put =>
                        HandleStoragePut(request, inputStack),
                    uint hash when hash == ApplicationEngine.System_Storage_Delete =>
                        HandleStorageDelete(request, inputStack),
                    uint hash when hash == ApplicationEngine.System_Crypto_CheckSig =>
                        HandleCryptoCheckSig(request, inputStack),
                    uint hash when hash == ApplicationEngine.System_Crypto_CheckMultisig =>
                        HandleCryptoCheckMultisig(request, inputStack),
                    uint hash when hash == ApplicationEngine.System_Contract_NativeOnPersist =>
                        HandleNativeOnPersist(request, inputStack),
                    uint hash when hash == ApplicationEngine.System_Contract_NativePostPersist =>
                        HandleNativePostPersist(request, inputStack),
                    uint hash when hash == ApplicationEngine.System_Contract_CallNative =>
                        HandleContractCallNative(request, scope, instructionPointer, inputStack),
                    uint hash when hash == ApplicationEngine.System_Contract_Call =>
                        HandleContractCall(request, scope, effectiveGasLeft, inputStack),
                    uint hash when hash == ApplicationEngine.System_Contract_GetCallFlags =>
                        Append(inputStack, new Integer((int)request.CurrentCallFlags)),
                    uint hash when hash == ApplicationEngine.System_Contract_CreateStandardAccount =>
                        HandleCreateStandardAccount(request, inputStack),
                    uint hash when hash == ApplicationEngine.System_Contract_CreateMultisigAccount =>
                        HandleCreateMultisigAccount(request, inputStack),
                    uint hash when hash == ApplicationEngine.System_Iterator_Next =>
                        HandleIteratorNext(inputStack),
                    uint hash when hash == ApplicationEngine.System_Iterator_Value =>
                        HandleIteratorValue(request, inputStack),
                    uint hash when hash == ApplicationEngine.System_Runtime_GetInvocationCounter =>
                        HandleGetInvocationCounter(request, inputStack),
                    uint hash when hash == ApplicationEngine.System_Runtime_CurrentSigners =>
                        Append(inputStack, request.Engine.Convert(request.Engine.GetCurrentSigners()) ?? StackItem.Null),
                    uint hash when hash == ApplicationEngine.System_Runtime_BurnGas =>
                        HandleBurnGas(request, inputStack),
                    uint hash when hash == ApplicationEngine.System_Runtime_CheckWitness =>
                        HandleCheckWitness(request, inputStack),
                    uint hash when hash == ApplicationEngine.System_Runtime_GetCallingScriptHash =>
                        HandleCallingScriptHash(request, inputStack),
                    uint hash when hash == ApplicationEngine.System_Runtime_GetNotifications =>
                        HandleGetNotifications(request, inputStack),
                    uint hash when hash == ApplicationEngine.System_Runtime_GetTime && request.PersistingTimestamp != 0 =>
                        Append(inputStack, new Integer(request.PersistingTimestamp)),
                    uint hash when hash == ApplicationEngine.System_Runtime_GetTime =>
                        throw new InvalidOperationException("GetTime requires a persisting block timestamp."),
                    uint hash when hash == ApplicationEngine.System_Runtime_GetExecutingScriptHash =>
                        Append(inputStack, new ByteString(request.ScriptHashes[^1].GetSpan().ToArray())),
                    uint hash when hash == ApplicationEngine.System_Runtime_GetEntryScriptHash =>
                        HandleEntryScriptHash(request, inputStack),
                    uint hash when hash == ApplicationEngine.System_Runtime_LoadScript =>
                        HandleRuntimeLoadScript(request, scope, effectiveGasLeft, inputStack),
                    uint hash when hash == ApplicationEngine.System_Runtime_Notify =>
                        HandleRuntimeNotify(request, inputStack),
                    uint hash when hash == ApplicationEngine.System_Runtime_Log =>
                        HandleRuntimeLog(request, inputStack),
                    _ => throw new InvalidOperationException($"Unsupported syscall 0x{api:x8}.")
                };

                if (TraceEnabled)
                {
                    for (var index = 0; index < stack.Length; index++)
                        Trace($"syscall exit item[{index}] name={descriptor.Name} value={DescribeStackItem(stack[index])}");
                }
                Trace($"syscall exit name={descriptor.Name} api=0x{api:x8} resultStackLen={stack.Length}");
                result = CreateNativeHostResult(stack, scope);
                return true;
            }
            catch (Exception ex)
            {
                Trace($"syscall fault api=0x{api:x8} type={ex.GetType().FullName} message={ex.Message}");
                result = CreateNativeHostError(ex);
                return true;
            }
        }

        private static StackItem[] Append(StackItem[] inputStack, StackItem item)
        {
            var next = new StackItem[inputStack.Length + 1];
            System.Array.Copy(inputStack, next, inputStack.Length);
            next[^1] = item;
            return next;
        }

        private static StackItem[] HandleCallingScriptHash(RiscvExecutionRequest request, StackItem[] inputStack)
        {
            var current = request.ScriptHashes[^1];
            var expected = request.Scripts.Count > 1 ? request.ScriptHashes[^2] : null;
            var value = GetTestingHooks(request.Engine)?.OverrideCallingScriptHash(current, expected) ?? expected;
            return AppendHashOrNull(inputStack, value);
        }

        private static StackItem[] HandleEntryScriptHash(RiscvExecutionRequest request, StackItem[] inputStack)
        {
            var current = request.ScriptHashes[^1];
            var expected = request.ScriptHashes[0];
            var value = GetTestingHooks(request.Engine)?.OverrideEntryScriptHash(current, expected) ?? expected;
            return AppendHashOrNull(inputStack, value);
        }

        private static StackItem[] AppendHashOrNull(StackItem[] inputStack, UInt160? hash)
        {
            return Append(inputStack, hash is null ? StackItem.Null : new ByteString(hash.GetSpan().ToArray()));
        }

        private static IRiscvApplicationEngineTestingHooks? GetTestingHooks(ApplicationEngine engine)
        {
            return engine is RiscvApplicationEngine riscvEngine ? riscvEngine.TestingHooks : null;
        }
    }
}
