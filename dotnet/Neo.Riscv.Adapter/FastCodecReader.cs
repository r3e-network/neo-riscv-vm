// Copyright (C) 2015-2026 The Neo Project.
//
// FastCodecReader.cs file belongs to the neo project and is free
// software distributed under the MIT software license, see the
// accompanying file LICENSE in the main directory of the
// repository or http://www.opensource.org/licenses/mit-license.php
// for more details.

using Neo.VM;
using Neo.VM.Types;
using System;
using System.Numerics;
using Array = Neo.VM.Types.Array;
using Buffer = Neo.VM.Types.Buffer;

namespace Neo.SmartContract.RiscV
{
    /// <summary>
    /// Minimal decoder for the Rust-side <c>neo_riscv_abi::fast_codec</c> wire format.
    /// Used by the fault-side-channel pathway to deserialize a faulting frame's local
    /// variables (captured by the guest at fault) into NeoVM <see cref="StackItem"/>s.
    ///
    /// Tag map (mirrors <c>crates/neo-riscv-abi/src/fast_codec.rs</c>):
    ///   0x01 Integer     (i64 LE)
    ///   0x02 BigInteger  (u32 LE length + signed little-endian magnitude)
    ///   0x03 ByteString  (u32 LE length + bytes)
    ///   0x04 Boolean     (1 byte)
    ///   0x05 Array       (u32 LE count + items)
    ///   0x06 Struct      (u32 LE count + items)
    ///   0x07 Map         (u32 LE count + key/value pairs)
    ///   0x08 Interop     (u64 LE handle — decoded as integer stub)
    ///   0x09 Iterator    (u64 LE handle — decoded as integer stub)
    ///   0x0A Null
    ///   0x0B Pointer     (i64 LE — decoded as integer stub)
    ///   0x0C Buffer      (u32 LE length + bytes)
    /// </summary>
    internal static class FastCodecReader
    {
        private const byte TagInteger = 0x01;
        private const byte TagBigInteger = 0x02;
        private const byte TagByteString = 0x03;
        private const byte TagBoolean = 0x04;
        private const byte TagArray = 0x05;
        private const byte TagStruct = 0x06;
        private const byte TagMap = 0x07;
        private const byte TagInterop = 0x08;
        private const byte TagIterator = 0x09;
        private const byte TagNull = 0x0A;
        private const byte TagPointer = 0x0B;
        private const byte TagBuffer = 0x0C;

        internal static StackItem[] DecodeStack(ReadOnlySpan<byte> bytes, IReferenceCounter referenceCounter)
        {
            if (bytes.Length < 4) return System.Array.Empty<StackItem>();
            int pos = 0;
            var count = (int)ReadUInt32(bytes, ref pos);
            if (count < 0 || count > 4096) return System.Array.Empty<StackItem>();
            var items = new StackItem[count];
            for (var i = 0; i < count; i++)
            {
                items[i] = DecodeValue(bytes, ref pos, referenceCounter);
            }
            return items;
        }

        private static StackItem DecodeValue(ReadOnlySpan<byte> bytes, ref int pos, IReferenceCounter referenceCounter)
        {
            if (pos >= bytes.Length) return StackItem.Null;
            var tag = bytes[pos++];
            switch (tag)
            {
                case TagInteger:
                    return new Integer(ReadInt64(bytes, ref pos));
                case TagBigInteger:
                {
                    var len = (int)ReadUInt32(bytes, ref pos);
                    var slice = bytes.Slice(pos, len);
                    pos += len;
                    return new Integer(new BigInteger(slice, isUnsigned: false, isBigEndian: false));
                }
                case TagByteString:
                {
                    var len = (int)ReadUInt32(bytes, ref pos);
                    var buf = bytes.Slice(pos, len).ToArray();
                    pos += len;
                    return new ByteString(buf);
                }
                case TagBoolean:
                    return bytes[pos++] != 0 ? StackItem.True : StackItem.False;
                case TagArray:
                {
                    var count = (int)ReadUInt32(bytes, ref pos);
                    var array = new Array(referenceCounter);
                    for (var i = 0; i < count; i++) array.Add(DecodeValue(bytes, ref pos, referenceCounter));
                    return array;
                }
                case TagStruct:
                {
                    var count = (int)ReadUInt32(bytes, ref pos);
                    var s = new Struct(referenceCounter);
                    for (var i = 0; i < count; i++) s.Add(DecodeValue(bytes, ref pos, referenceCounter));
                    return s;
                }
                case TagMap:
                {
                    var count = (int)ReadUInt32(bytes, ref pos);
                    var map = new Map(referenceCounter);
                    for (var i = 0; i < count; i++)
                    {
                        var k = DecodeValue(bytes, ref pos, referenceCounter);
                        var v = DecodeValue(bytes, ref pos, referenceCounter);
                        if (k is PrimitiveType p) map[p] = v;
                    }
                    return map;
                }
                case TagInterop:
                case TagIterator:
                    pos += 8;
                    return StackItem.Null;   // dev-time locals snapshot: handles aren't faithfully reconstructable
                case TagNull:
                    return StackItem.Null;
                case TagPointer:
                    pos += 8;
                    return StackItem.Null;   // pointers have no meaningful StackItem equivalent out of context
                case TagBuffer:
                {
                    var len = (int)ReadUInt32(bytes, ref pos);
                    var buf = bytes.Slice(pos, len).ToArray();
                    pos += len;
                    return new Buffer(buf);
                }
                default:
                    return StackItem.Null;
            }
        }

        private static uint ReadUInt32(ReadOnlySpan<byte> bytes, ref int pos)
        {
            var v = (uint)(bytes[pos] | (bytes[pos + 1] << 8) | (bytes[pos + 2] << 16) | (bytes[pos + 3] << 24));
            pos += 4;
            return v;
        }

        private static long ReadInt64(ReadOnlySpan<byte> bytes, ref int pos)
        {
            long v = 0;
            for (var i = 0; i < 8; i++) v |= ((long)bytes[pos + i]) << (i * 8);
            pos += 8;
            return v;
        }
    }
}
