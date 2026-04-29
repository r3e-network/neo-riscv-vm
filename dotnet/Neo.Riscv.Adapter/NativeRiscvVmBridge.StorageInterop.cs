using Neo.SmartContract;
using Neo.SmartContract.Iterators;
using Neo.SmartContract.Native;
using Neo.VM;
using Neo.VM.Types;
using System;

namespace Neo.SmartContract.RiscV
{
    public sealed partial class NativeRiscvVmBridge
    {
        private static StackItem CreateStorageContextItem(StorageContext context)
        {
            return StackItem.FromInterface(context);
        }

        private static StorageContext CreateStorageContext(RiscvExecutionRequest request, bool isReadOnly)
        {
            var contract = NativeContract.ContractManagement.GetContract(request.Engine.SnapshotCache, request.ScriptHashes[^1])
                ?? throw new InvalidOperationException("This method can only be called by a deployed contract.");
            return new StorageContext
            {
                Id = contract.Id,
                IsReadOnly = isReadOnly
            };
        }

        private static StackItem[] HandleStorageGet(RiscvExecutionRequest request, StackItem[] inputStack)
        {
            if (inputStack.Length < 2)
                throw new InvalidOperationException("Storage.Get requires context and key.");

            StorageContext context;
            byte[] key;
            if (TryParseStorageContextItem(inputStack[^2], out context) && TryGetByteLikeBytes(inputStack[^1], out var keyAtTop))
            {
                key = keyAtTop;
            }
            else if (TryParseStorageContextItem(inputStack[^1], out context) && TryGetByteLikeBytes(inputStack[^2], out var keyAtBottom))
            {
                key = keyAtBottom;
            }
            else
            {
                throw new InvalidOperationException("Storage.Get requires a storage context token and a byte-like key.");
            }

            var value = request.Engine.Get(context, key);
            var next = new StackItem[inputStack.Length - 1];
            if (inputStack.Length > 2)
            {
                System.Array.Copy(inputStack, next, inputStack.Length - 2);
            }
            next[^1] = value.HasValue ? new ByteString(value.Value) : StackItem.Null;
            return next;
        }

        private static StackItem[] HandleStorageLocalGet(RiscvExecutionRequest request, StackItem[] inputStack)
        {
            if (inputStack.Length == 0)
                throw new InvalidOperationException("Storage.Local.Get requires a key.");

            if (!TryGetByteLikeBytes(inputStack[^1], out var key))
                throw new InvalidOperationException("Storage.Local.Get requires a byte-like key.");

            var value = request.Engine.GetLocal(key);
            var next = new StackItem[inputStack.Length];
            if (inputStack.Length > 1)
            {
                System.Array.Copy(inputStack, next, inputStack.Length - 1);
            }
            next[^1] = value.HasValue ? new ByteString(value.Value) : StackItem.Null;
            return next;
        }

        private static StackItem[] HandleStorageLocalFind(RiscvExecutionRequest request, StackItem[] inputStack)
        {
            if (inputStack.Length < 2)
                throw new InvalidOperationException("Storage.Local.Find requires prefix and options.");

            if (!TryGetByteLikeBytes(inputStack[^2], out var prefix))
                throw new InvalidOperationException("Storage.Local.Find requires a byte-like prefix.");
            if (inputStack[^1] is not Integer options)
                throw new InvalidOperationException("Storage.Local.Find requires integer options.");

            var iterator = request.Engine.FindLocal(prefix, (FindOptions)(byte)options.GetInteger());
            var next = new StackItem[inputStack.Length - 1];
            if (inputStack.Length > 2)
            {
                System.Array.Copy(inputStack, next, inputStack.Length - 2);
            }
            next[^1] = StackItem.FromInterface(iterator);
            return next;
        }

        private static StackItem[] HandleStorageAsReadOnly(RiscvExecutionRequest request, StackItem[] inputStack)
        {
            if (inputStack.Length == 0)
                throw new InvalidOperationException("Storage.AsReadOnly requires one argument.");

            var context = ParseStorageContext(inputStack[^1]);
            var next = new StackItem[inputStack.Length];
            if (inputStack.Length > 1)
            {
                System.Array.Copy(inputStack, next, inputStack.Length - 1);
            }
            next[^1] = CreateStorageContextItem(ApplicationEngine.AsReadOnly(context));
            return next;
        }

        private static StackItem[] HandleStoragePut(RiscvExecutionRequest request, StackItem[] inputStack)
        {
            if (inputStack.Length < 3)
                throw new InvalidOperationException("Storage.Put requires context, key, and value.");

            StorageContext context;
            byte[] key;
            byte[] value;
            if (TryParseStorageContextItem(inputStack[^3], out context) &&
                TryGetByteLikeBytes(inputStack[^2], out var keyForward) &&
                TryGetByteLikeBytes(inputStack[^1], out var valueForward))
            {
                key = keyForward;
                value = valueForward;
            }
            else if (TryParseStorageContextItem(inputStack[^1], out context) &&
                     TryGetByteLikeBytes(inputStack[^2], out var keyReverse) &&
                     TryGetByteLikeBytes(inputStack[^3], out var valueReverse))
            {
                key = keyReverse;
                value = valueReverse;
            }
            else
            {
                throw new InvalidOperationException("Storage.Put requires a storage context token plus byte-like key and value.");
            }

            request.Engine.Put(context, key, value);

            var next = new StackItem[inputStack.Length - 3];
            if (next.Length > 0)
            {
                System.Array.Copy(inputStack, next, next.Length);
            }
            return next;
        }

        private static StackItem[] HandleStorageLocalPut(RiscvExecutionRequest request, StackItem[] inputStack)
        {
            if (inputStack.Length < 2)
                throw new InvalidOperationException("Storage.Local.Put requires key and value.");

            if (!TryGetByteLikeBytes(inputStack[^2], out var key))
                throw new InvalidOperationException("Storage.Local.Put requires a byte-like key.");
            if (!TryGetByteLikeBytes(inputStack[^1], out var value))
                throw new InvalidOperationException("Storage.Local.Put requires a byte-like value.");

            request.Engine.PutLocal(key, value);

            var next = new StackItem[inputStack.Length - 2];
            if (next.Length > 0)
            {
                System.Array.Copy(inputStack, next, next.Length);
            }
            return next;
        }

        private static StackItem[] HandleStorageDelete(RiscvExecutionRequest request, StackItem[] inputStack)
        {
            if (inputStack.Length < 2)
                throw new InvalidOperationException("Storage.Delete requires context and key.");

            StorageContext context;
            byte[] key;
            if (TryParseStorageContextItem(inputStack[^2], out context) && TryGetByteLikeBytes(inputStack[^1], out var keyAtTop))
            {
                key = keyAtTop;
            }
            else if (TryParseStorageContextItem(inputStack[^1], out context) && TryGetByteLikeBytes(inputStack[^2], out var keyAtBottom))
            {
                key = keyAtBottom;
            }
            else
            {
                throw new InvalidOperationException("Storage.Delete requires a storage context token and a byte-like key.");
            }

            request.Engine.Delete(context, key);

            var next = new StackItem[inputStack.Length - 2];
            if (next.Length > 0)
            {
                System.Array.Copy(inputStack, next, next.Length);
            }
            return next;
        }

        private static StackItem[] HandleStorageLocalDelete(RiscvExecutionRequest request, StackItem[] inputStack)
        {
            if (inputStack.Length == 0)
                throw new InvalidOperationException("Storage.Local.Delete requires a key.");

            if (!TryGetByteLikeBytes(inputStack[^1], out var key))
                throw new InvalidOperationException("Storage.Local.Delete requires a byte-like key.");

            request.Engine.DeleteLocal(key);

            var next = new StackItem[inputStack.Length - 1];
            if (next.Length > 0)
            {
                System.Array.Copy(inputStack, next, next.Length);
            }
            return next;
        }

        private static StackItem[] HandleStorageFind(RiscvExecutionRequest request, StackItem[] inputStack)
        {
            if (inputStack.Length < 3)
                throw new InvalidOperationException("Storage.Find requires context, prefix, and options.");

            StorageContext context;
            byte[] prefix;
            Integer options;
            if (TryParseStorageContextItem(inputStack[^3], out context) &&
                TryGetByteLikeBytes(inputStack[^2], out var prefixForward) &&
                inputStack[^1] is Integer optionsForward)
            {
                prefix = prefixForward;
                options = optionsForward;
            }
            else if (TryParseStorageContextItem(inputStack[^1], out context) &&
                     TryGetByteLikeBytes(inputStack[^2], out var prefixReverse) &&
                     inputStack[^3] is Integer optionsReverse)
            {
                prefix = prefixReverse;
                options = optionsReverse;
            }
            else
            {
                throw new InvalidOperationException("Storage.Find requires a storage context token, byte-like prefix, and integer options.");
            }

            var iterator = request.Engine.Find(context, prefix, (FindOptions)(byte)options.GetInteger());
            var next = new StackItem[inputStack.Length - 2];
            if (inputStack.Length > 3)
            {
                System.Array.Copy(inputStack, next, inputStack.Length - 3);
            }
            next[^1] = StackItem.FromInterface(iterator);
            return next;
        }

        private static StackItem[] HandleIteratorNext(StackItem[] inputStack)
        {
            if (inputStack.Length == 0)
                throw new InvalidOperationException("Iterator.Next requires an iterator.");

            if (inputStack[^1] is not InteropInterface interop || interop.GetInterface<object>() is not IIterator iterator)
                throw new InvalidOperationException("Iterator.Next requires an iterator handle.");

            var next = new StackItem[inputStack.Length];
            if (inputStack.Length > 1)
            {
                System.Array.Copy(inputStack, next, inputStack.Length - 1);
            }
            next[^1] = iterator.Next() ? StackItem.True : StackItem.False;
            return next;
        }

        private static StackItem[] HandleIteratorValue(RiscvExecutionRequest request, StackItem[] inputStack)
        {
            if (inputStack.Length == 0)
                throw new InvalidOperationException("Iterator.Value requires an iterator.");

            if (inputStack[^1] is not InteropInterface interop || interop.GetInterface<object>() is not IIterator iterator)
                throw new InvalidOperationException("Iterator.Value requires an iterator handle.");

            var next = new StackItem[inputStack.Length];
            if (inputStack.Length > 1)
            {
                System.Array.Copy(inputStack, next, inputStack.Length - 1);
            }
            next[^1] = iterator.Value(request.Engine.ReferenceCounter);
            return next;
        }

        private static bool TryParseStorageContextItem(StackItem item, out StorageContext context)
        {
            if (item is InteropInterface interop && interop.GetInterface<object>() is StorageContext storageContext)
            {
                context = storageContext;
                return true;
            }

            context = new StorageContext();
            return false;
        }

        private static StorageContext ParseStorageContext(StackItem item)
        {
            if (TryParseStorageContextItem(item, out var context))
                return context;

            var detail = item switch
            {
                ByteString bytes => $"bytes:{Convert.ToHexString(bytes.GetSpan())}",
                Neo.VM.Types.Buffer bytes => $"buffer:{Convert.ToHexString(bytes.GetSpan())}",
                InteropInterface interop => $"interop:{interop.GetInterface<object>()?.GetType().Name ?? "null"}",
                _ => item.GetType().Name
            };
            throw new InvalidOperationException($"Storage context must be an opaque interop handle, got {detail}.");
        }

        private static bool TryGetByteLikeBytes(StackItem item, out byte[] bytes)
        {
            switch (item)
            {
                case ByteString byteString:
                    bytes = byteString.GetSpan().ToArray();
                    return true;
                case Neo.VM.Types.Buffer buffer:
                    bytes = buffer.GetSpan().ToArray();
                    return true;
                default:
                    bytes = System.Array.Empty<byte>();
                    return false;
            }
        }

    }
}
