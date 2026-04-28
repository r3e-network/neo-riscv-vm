using Neo.VM;
using Neo.VM.Types;
using Neo.SmartContract.Iterators;
using System;
using System.Collections.Generic;
using System.Linq;
using System.Numerics;
using System.Runtime.InteropServices;
using System.Text;

namespace Neo.SmartContract.RiscV
{
    public sealed partial class NativeRiscvVmBridge
    {
        private static void StaticHostFreeCallback(IntPtr userData, ref NativeHostResult result)
        {
            if (result.StackPtr != IntPtr.Zero)
            {
                if (result.StackPtr != CachedNullStackPtr &&
                    result.StackPtr != CachedBoolTrueStackPtr &&
                    result.StackPtr != CachedBoolFalseStackPtr &&
                    result.StackPtr != CachedIntZeroStackPtr &&
                    !IsCachedSmallIntStackPtr(result.StackPtr))
                {
                    FreeNativeStackItems(result.StackPtr, (int)result.StackLen);
                }
                result.StackPtr = IntPtr.Zero;
                result.StackLen = 0;
            }

            if (result.ErrorPtr != IntPtr.Zero)
            {
                Marshal.FreeHGlobal(result.ErrorPtr);
                result.ErrorPtr = IntPtr.Zero;
                result.ErrorLen = 0;
            }
        }

        private static void FreeNativeStackItems(IntPtr stackPtr, int stackLen)
        {
            if (stackPtr == IntPtr.Zero) return;
            if (stackPtr == CachedNullStackPtr ||
                stackPtr == CachedBoolTrueStackPtr ||
                stackPtr == CachedBoolFalseStackPtr ||
                stackPtr == CachedIntZeroStackPtr ||
                IsCachedSmallIntStackPtr(stackPtr))
                return;
            var itemSize = Marshal.SizeOf<NativeStackItem>();
            for (var index = 0; index < stackLen; index++)
            {
                var itemPtr = IntPtr.Add(stackPtr, index * itemSize);
                var item = Marshal.PtrToStructure<NativeStackItem>(itemPtr);
                if (item.BytesPtr != IntPtr.Zero)
                {
                    if (item.Kind == 4 || item.Kind == 7 || item.Kind == 8)
                    {
                        FreeNativeStackItems(item.BytesPtr, (int)item.BytesLen);
                    }
                    else
                    {
                        Marshal.FreeHGlobal(item.BytesPtr);
                    }
                }
            }
            Marshal.FreeHGlobal(stackPtr);
        }

        private NativeHostResult CreateNativeHostResult(StackItem[] stack, ExecutionScope scope)
        {
            if (stack.Length == 0)
            {
                return new NativeHostResult();
            }

            if (TryCreateCachedNativeHostResult(stack, out var cached))
                return cached;

            var itemSize = Marshal.SizeOf<NativeStackItem>();
            var stackPtr = Marshal.AllocHGlobal(itemSize * stack.Length);

            for (var index = 0; index < stack.Length; index++)
            {
                var nativeItem = stack[index] switch
                {
                    Integer integer when integer.Size > sizeof(long) => CreateNativeBigIntegerItem(integer),
                    Integer integer => new NativeStackItem
                    {
                        Kind = 0,
                        IntegerValue = (long)integer.GetInteger(),
                        BytesPtr = IntPtr.Zero,
                        BytesLen = 0,
                    },
                    ByteString byteString => CreateNativeByteStringItem(byteString),
                    Neo.VM.Types.Buffer buffer => CreateNativeBufferItem(buffer),
                    Neo.VM.Types.Struct @struct => CreateNativeStructItem(@struct, scope),
                    Neo.VM.Types.Array array => CreateNativeArrayItem(array, scope),
                    Neo.VM.Types.Map map => CreateNativeMapItem(map, scope),
                    Neo.VM.Types.Boolean boolean => new NativeStackItem
                    {
                        Kind = 3,
                        IntegerValue = boolean.GetBoolean() ? 1 : 0,
                        BytesPtr = IntPtr.Zero,
                        BytesLen = 0,
                    },
                    InteropInterface interop when interop.GetInterface<object>() is IIterator iterator => new NativeStackItem
                    {
                        Kind = 6,
                        IntegerValue = checked((long)RegisterIterator(scope, iterator)),
                        BytesPtr = IntPtr.Zero,
                        BytesLen = 0,
                    },
                    InteropInterface interop => new NativeStackItem
                    {
                        Kind = 9,
                        IntegerValue = checked((long)RegisterInterop(scope, interop.GetInterface<object>()!)),
                        BytesPtr = IntPtr.Zero,
                        BytesLen = 0,
                    },
                    Null => new NativeStackItem
                    {
                        Kind = 2,
                        IntegerValue = 0,
                        BytesPtr = IntPtr.Zero,
                        BytesLen = 0,
                    },
                    Neo.VM.Types.Pointer pointer => new NativeStackItem
                    {
                        Kind = 10,
                        IntegerValue = pointer.Position,
                        BytesPtr = IntPtr.Zero,
                        BytesLen = 0,
                    },
                    _ => throw new InvalidOperationException($"Unsupported host callback stack item type: {stack[index].GetType().Name}.")
                };

                Marshal.StructureToPtr(nativeItem, IntPtr.Add(stackPtr, index * itemSize), false);
            }

            return new NativeHostResult
            {
                StackPtr = stackPtr,
                StackLen = (nuint)stack.Length,
                ErrorPtr = IntPtr.Zero,
                ErrorLen = 0,
            };
        }

        private static bool TryCreateCachedNativeHostResult(StackItem[] stack, out NativeHostResult result)
        {
            result = default;
            if (stack.Length != 1)
                return false;

            var stackPtr = stack[0] switch
            {
                Null => CachedNullStackPtr,
                Neo.VM.Types.Boolean boolean when boolean.GetBoolean() => CachedBoolTrueStackPtr,
                Neo.VM.Types.Boolean => CachedBoolFalseStackPtr,
                Integer integer when integer.Size <= sizeof(long) && integer.GetInteger().IsZero => CachedIntZeroStackPtr,
                Integer integer when integer.Size <= sizeof(long) && TryGetCachedSmallIntStackPtr((long)integer.GetInteger(), out var cachedIntPtr) => cachedIntPtr,
                _ => IntPtr.Zero
            };

            if (stackPtr == IntPtr.Zero)
                return false;

            result = new NativeHostResult
            {
                StackPtr = stackPtr,
                StackLen = 1,
                ErrorPtr = IntPtr.Zero,
                ErrorLen = 0,
            };
            return true;
        }

        private static IntPtr CreateCachedSingleStackItem(uint kind, long integerValue)
        {
            var itemSize = Marshal.SizeOf<NativeStackItem>();
            var stackPtr = Marshal.AllocHGlobal(itemSize);
            var item = new NativeStackItem
            {
                Kind = kind,
                IntegerValue = integerValue,
                BytesPtr = IntPtr.Zero,
                BytesLen = 0,
            };
            Marshal.StructureToPtr(item, stackPtr, false);
            return stackPtr;
        }

        private static IntPtr[] CreateCachedSmallIntStackPtrs()
        {
            var values = new IntPtr[CachedSmallIntMax - CachedSmallIntMin + 1];
            for (var value = CachedSmallIntMin; value <= CachedSmallIntMax; value++)
            {
                values[value - CachedSmallIntMin] = value == 0
                    ? CachedIntZeroStackPtr
                    : CreateCachedSingleStackItem(0, value);
            }
            return values;
        }

        private static bool TryGetCachedSmallIntStackPtr(long value, out IntPtr stackPtr)
        {
            if (value >= CachedSmallIntMin && value <= CachedSmallIntMax)
            {
                stackPtr = CachedSmallIntStackPtrs[value - CachedSmallIntMin];
                return true;
            }

            stackPtr = IntPtr.Zero;
            return false;
        }

        private static bool IsCachedSmallIntStackPtr(IntPtr stackPtr)
        {
            foreach (var cached in CachedSmallIntStackPtrs)
            {
                if (cached == stackPtr)
                    return true;
            }

            return false;
        }

        private static NativeStackItem CreateNativeByteStringItem(ByteString byteString)
        {
            var bytes = byteString.GetSpan().ToArray();
            var bytesPtr = bytes.Length == 0 ? IntPtr.Zero : Marshal.AllocHGlobal(bytes.Length);
            if (bytes.Length > 0)
            {
                Marshal.Copy(bytes, 0, bytesPtr, bytes.Length);
            }

            return new NativeStackItem
            {
                Kind = 1,
                IntegerValue = 0,
                BytesPtr = bytesPtr,
                BytesLen = (nuint)bytes.Length,
            };
        }

        private static NativeStackItem CreateNativeBufferItem(Neo.VM.Types.Buffer buffer)
        {
            var bytes = buffer.GetSpan().ToArray();
            var bytesPtr = bytes.Length == 0 ? IntPtr.Zero : Marshal.AllocHGlobal(bytes.Length);
            if (bytes.Length > 0)
            {
                Marshal.Copy(bytes, 0, bytesPtr, bytes.Length);
            }

            return new NativeStackItem
            {
                Kind = 11,
                IntegerValue = 0,
                BytesPtr = bytesPtr,
                BytesLen = (nuint)bytes.Length,
            };
        }

        private static NativeStackItem CreateNativeBigIntegerItem(Integer integer)
        {
            var bytes = integer.GetInteger().ToByteArray();
            var bytesPtr = bytes.Length == 0 ? IntPtr.Zero : Marshal.AllocHGlobal(bytes.Length);
            if (bytes.Length > 0)
            {
                Marshal.Copy(bytes, 0, bytesPtr, bytes.Length);
            }

            return new NativeStackItem
            {
                Kind = 5,
                IntegerValue = 0,
                BytesPtr = bytesPtr,
                BytesLen = (nuint)bytes.Length,
            };
        }

        private NativeStackItem CreateNativeArrayItem(Neo.VM.Types.Array array, ExecutionScope scope)
        {
            var items = new StackItem[array.Count];
            for (var index = 0; index < array.Count; index++)
            {
                items[index] = array[index];
            }

            var nested = CreateNativeHostResult(items, scope);
            return new NativeStackItem
            {
                Kind = 4,
                IntegerValue = 0,
                BytesPtr = nested.StackPtr,
                BytesLen = nested.StackLen,
            };
        }

        private NativeStackItem CreateNativeStructItem(Neo.VM.Types.Struct @struct, ExecutionScope scope)
        {
            var items = new StackItem[@struct.Count];
            for (var index = 0; index < @struct.Count; index++)
            {
                items[index] = @struct[index];
            }

            var nested = CreateNativeHostResult(items, scope);
            return new NativeStackItem
            {
                Kind = 7,
                IntegerValue = 0,
                BytesPtr = nested.StackPtr,
                BytesLen = nested.StackLen,
            };
        }

        private NativeStackItem CreateNativeMapItem(Neo.VM.Types.Map map, ExecutionScope scope)
        {
            var items = new StackItem[map.Count * 2];
            var offset = 0;
            foreach (var entry in map)
            {
                items[offset++] = entry.Key;
                items[offset++] = entry.Value;
            }

            var nested = CreateNativeHostResult(items, scope);
            return new NativeStackItem
            {
                Kind = 8,
                IntegerValue = 0,
                BytesPtr = nested.StackPtr,
                BytesLen = nested.StackLen,
            };
        }

        private static NativeHostResult CreateNativeHostError(Exception exception)
        {
            var payload = string.Join("\n", new[]
            {
                exception.GetType().FullName ?? typeof(InvalidOperationException).FullName!,
                exception.Message,
                exception.InnerException?.GetType().FullName ?? string.Empty,
                exception.InnerException?.Message ?? string.Empty,
            });
            var bytes = Encoding.UTF8.GetBytes(payload);
            var errorPtr = bytes.Length == 0 ? IntPtr.Zero : Marshal.AllocHGlobal(bytes.Length);
            if (bytes.Length > 0)
            {
                Marshal.Copy(bytes, 0, errorPtr, bytes.Length);
            }

            return new NativeHostResult
            {
                StackPtr = IntPtr.Zero,
                StackLen = 0,
                ErrorPtr = errorPtr,
                ErrorLen = (nuint)bytes.Length,
            };
        }

        private static Exception RehydrateNativeException(string payload)
        {
            var parts = payload.Split('\n');
            if (parts.Length < 4)
            {
                return new InvalidOperationException(payload);
            }

            var innerMessage = parts.Length > 4
                ? string.Join("\n", parts[3..])
                : parts[3];
            Exception? inner = string.IsNullOrEmpty(parts[2]) ? null : CreateException(parts[2], innerMessage);
            if (parts[0] == typeof(System.Reflection.TargetInvocationException).FullName && inner is not null)
                return new System.Reflection.TargetInvocationException(inner);
            return CreateException(parts[0], parts[1], inner);
        }

        private static Exception CreateException(string typeName, string message, Exception? inner = null)
        {
            return typeName switch
            {
                nameof(ArgumentException) or "System.ArgumentException" => new ArgumentException(message, inner),
                nameof(ArgumentOutOfRangeException) or "System.ArgumentOutOfRangeException" => new ArgumentOutOfRangeException(paramName: null, message: message),
                nameof(FormatException) or "System.FormatException" => new FormatException(message, inner),
                nameof(InvalidOperationException) or "System.InvalidOperationException" => new InvalidOperationException(message, inner),
                nameof(NullReferenceException) or "System.NullReferenceException" => new NullReferenceException(message),
                nameof(NotSupportedException) or "System.NotSupportedException" => new NotSupportedException(message, inner),
                nameof(NotImplementedException) or "System.NotImplementedException" => new NotImplementedException(message, inner),
                nameof(OverflowException) or "System.OverflowException" => new OverflowException(message, inner),
                nameof(IndexOutOfRangeException) or "System.IndexOutOfRangeException" => new IndexOutOfRangeException(message),
                nameof(KeyNotFoundException) or "System.Collections.Generic.KeyNotFoundException" => new KeyNotFoundException(message, inner),
                nameof(DivideByZeroException) or "System.DivideByZeroException" => new DivideByZeroException(message, inner),
                _ => new InvalidOperationException(message, inner),
            };
        }

        private static StackItem ReadByteString(NativeStackItem nativeItem)
        {
            if (nativeItem.BytesPtr == IntPtr.Zero)
                return ByteString.Empty;

            var bytes = new byte[checked((int)nativeItem.BytesLen)];
            Marshal.Copy(nativeItem.BytesPtr, bytes, 0, bytes.Length);
            return new ByteString(bytes);
        }

        private static StackItem ReadBuffer(NativeStackItem nativeItem)
        {
            var bytes = new byte[checked((int)nativeItem.BytesLen)];
            if (bytes.Length > 0)
            {
                Marshal.Copy(nativeItem.BytesPtr, bytes, 0, bytes.Length);
            }
            return new Neo.VM.Types.Buffer(bytes);
        }

        private static Integer ReadBigInteger(NativeStackItem nativeItem)
        {
            var bytes = new byte[checked((int)nativeItem.BytesLen)];
            if (bytes.Length > 0)
            {
                Marshal.Copy(nativeItem.BytesPtr, bytes, 0, bytes.Length);
            }
            return new Integer(new BigInteger(bytes));
        }

        private StackItem[] ReadStack(IntPtr stackPtr, nuint stackLen, IReferenceCounter? referenceCounter, ExecutionScope scope, bool decodeStorageContextTokens)
        {
            if (stackPtr == IntPtr.Zero || stackLen == 0)
                return System.Array.Empty<StackItem>();

            var stack = new StackItem[(int)stackLen];
            for (var index = 0; index < stack.Length; index++)
            {
                var itemPtr = IntPtr.Add(stackPtr, index * Marshal.SizeOf<NativeStackItem>());
                var nativeItem = Marshal.PtrToStructure<NativeStackItem>(itemPtr);
                if (TraceEnabled)
                {
                    Trace($"readstack item[{index}] kind={nativeItem.Kind} int={nativeItem.IntegerValue} bytesLen={nativeItem.BytesLen} bytesPtr=0x{nativeItem.BytesPtr.ToInt64():x}");
                }
                stack[index] = nativeItem.Kind switch
                {
                    0 => new Integer(nativeItem.IntegerValue),
                    5 => ReadBigInteger(nativeItem),
                    1 => ReadByteString(nativeItem),
                    11 => ReadBuffer(nativeItem),
                    3 => nativeItem.IntegerValue != 0 ? StackItem.True : StackItem.False,
                    4 => ReadArray(nativeItem, referenceCounter, scope),
                    7 => ReadStruct(nativeItem, referenceCounter, scope),
                    9 => ReadInteropHandle(scope, checked((ulong)nativeItem.IntegerValue)),
                    8 => ReadMap(nativeItem, referenceCounter, scope),
                    6 => ReadIteratorHandle(scope, checked((ulong)nativeItem.IntegerValue)),
                    2 => StackItem.Null,
                    10 => new Neo.VM.Types.Pointer(scope.CurrentScript, (int)nativeItem.IntegerValue),
                    _ => throw new InvalidOperationException($"Unsupported native stack item kind: {nativeItem.Kind}.")
                };
            }
            return stack;
        }

        private static StackItem ReadIteratorHandle(ExecutionScope scope, ulong handle)
        {
            if (!scope.Iterators.TryGetValue(handle, out var iterator))
                throw new InvalidOperationException($"Unknown iterator handle: {handle}.");
            return StackItem.FromInterface(iterator);
        }

        private static StackItem ReadInteropHandle(ExecutionScope scope, ulong handle)
        {
            if (!scope.InteropObjects.TryGetValue(handle, out var value))
                throw new InvalidOperationException($"Unknown interop handle: {handle}.");
            return StackItem.FromInterface(value);
        }

        private static ulong RegisterIterator(ExecutionScope scope, IIterator iterator)
        {
            var handle = scope.NextIteratorHandle++;
            scope.Iterators[handle] = iterator;
            return handle;
        }

        private static ulong RegisterInterop(ExecutionScope scope, object value)
        {
            var handle = scope.NextInteropHandle++;
            scope.InteropObjects[handle] = value;
            return handle;
        }

        private Neo.VM.Types.Array ReadArray(NativeStackItem nativeItem, IReferenceCounter? referenceCounter, ExecutionScope scope)
        {
            var children = ReadStack(nativeItem.BytesPtr, (nuint)nativeItem.BytesLen, referenceCounter, scope, decodeStorageContextTokens: true);
            return new Neo.VM.Types.Array(referenceCounter, children);
        }

        private Neo.VM.Types.Struct ReadStruct(NativeStackItem nativeItem, IReferenceCounter? referenceCounter, ExecutionScope scope)
        {
            var children = ReadStack(nativeItem.BytesPtr, (nuint)nativeItem.BytesLen, referenceCounter, scope, decodeStorageContextTokens: true);
            return new Neo.VM.Types.Struct(referenceCounter, children);
        }

        private Neo.VM.Types.Map ReadMap(NativeStackItem nativeItem, IReferenceCounter? referenceCounter, ExecutionScope scope)
        {
            var children = ReadStack(nativeItem.BytesPtr, (nuint)nativeItem.BytesLen, referenceCounter, scope, decodeStorageContextTokens: true);
            if (children.Length % 2 != 0)
                throw new InvalidOperationException("Native map stack item contains an odd number of entries.");

            var map = new Neo.VM.Types.Map(referenceCounter);
            for (var index = 0; index < children.Length; index += 2)
            {
                if (children[index] is not PrimitiveType key)
                    throw new InvalidOperationException("Native map stack item contains a non-primitive key.");
                map[key] = children[index + 1];
            }
            return map;
        }
    }
}
