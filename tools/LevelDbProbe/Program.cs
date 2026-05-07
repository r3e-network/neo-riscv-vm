using System.Buffers.Binary;
using System.Collections.Immutable;
using System.Numerics;
using System.Reflection;
using System.Runtime.CompilerServices;
using System.Text;
using System.Text.Json;
using Neo;
using Neo.Cryptography.ECC;
using Neo.Extensions;
using Neo.Network.P2P.Payloads;
using Neo.Persistence;
using Neo.Plugins.Storage;
using Neo.SmartContract;
using Neo.SmartContract.Native;
using Neo.SmartContract.RiscV;
using Neo.VM;

if (args.Length < 1)
{
    Usage();
    return 2;
}

return args[0] switch
{
    "state-roots" when args.Length == 4 => DumpStateRoots(args[1], uint.Parse(args[2]), uint.Parse(args[3])),
    "raw" when args.Length == 3 => DumpRaw(args[1], args[2]),
    "contract-script" when args.Length == 3 => DumpContractScript(args[1], args[2]),
    "disasm-contract" when args.Length == 5 => DumpContractDisasm(args[1], args[2], int.Parse(args[3]), int.Parse(args[4])),
    "storage-prefix" when args.Length is 4 or 5 => DumpStoragePrefix(args[1], args[2], args[3], args.Length == 5 ? int.Parse(args[4]) : 10),
    "replay-tx" when args.Length is 5 or 6 => await ReplayTransaction(args[1], uint.Parse(args[2]), args[3], args[4], args.Length == 6 ? args[5] : "http://seed1.neo.org:10332"),
    "replay-tx-neovm" when args.Length is 4 or 5 => await ReplayTransactionNeoVm(args[1], uint.Parse(args[2]), args[3], args.Length == 5 ? args[4] : "http://seed1.neo.org:10332"),
    _ => Usage()
};

static int Usage()
{
    Console.Error.WriteLine("Usage:");
    Console.Error.WriteLine("  dotnet run --project tools/LevelDbProbe -- state-roots <leveldb-path> <start> <end>");
    Console.Error.WriteLine("  dotnet run --project tools/LevelDbProbe -- raw <leveldb-path> <hex-key>");
    Console.Error.WriteLine("  dotnet run --project tools/LevelDbProbe -- contract-script <leveldb-path> <contract-hash>");
    Console.Error.WriteLine("  dotnet run --project tools/LevelDbProbe -- disasm-contract <leveldb-path> <contract-hash> <start-ip> <end-ip>");
    Console.Error.WriteLine("  dotnet run --project tools/LevelDbProbe -- storage-prefix <leveldb-path> <contract-hash> <prefix-hex> [sample-limit]");
    Console.Error.WriteLine("  dotnet run --project tools/LevelDbProbe -- replay-tx <leveldb-path> <block-index> <tx-hash> <host-lib> [rpc-url]");
    Console.Error.WriteLine("  dotnet run --project tools/LevelDbProbe -- replay-tx-neovm <leveldb-path> <block-index> <tx-hash> [rpc-url]");
    return 2;
}

static int DumpStateRoots(string path, uint start, uint end)
{
    using IStore store = new LevelDBStore().GetStore(path);

    store.TryGet([0x02], out var localRootIndexBytes);
    Console.WriteLine(localRootIndexBytes is { Length: >= 4 }
        ? $"CurrentLocalRootIndex={BitConverter.ToUInt32(localRootIndexBytes)}"
        : "CurrentLocalRootIndex=<missing>");

    store.TryGet([0x04], out var validatedRootIndexBytes);
    Console.WriteLine(validatedRootIndexBytes is { Length: >= 4 }
        ? $"CurrentValidatedRootIndex={BitConverter.ToUInt32(validatedRootIndexBytes)}"
        : "CurrentValidatedRootIndex=<missing>");

    for (var index = start; index <= end; index++)
    {
        store.TryGet(StateRootKey(index), out var value);
        if (value is null)
        {
            Console.WriteLine($"{index}\t<missing>");
            continue;
        }

        if (value.Length < 37)
        {
            Console.WriteLine($"{index}\t<short:{value.Length}>");
            continue;
        }

        var version = value[0];
        var storedIndex = BitConverter.ToUInt32(value.AsSpan(1, 4));
        var rootHash = new UInt256(value.AsSpan(5, UInt256.Length));
        Console.WriteLine($"{index}\tversion={version}\tstored={storedIndex}\troot={rootHash}\tbytes={value.Length}");
    }

    return 0;
}

static int DumpRaw(string path, string hexKey)
{
    using IStore store = new LevelDBStore().GetStore(path);
    var key = Convert.FromHexString(hexKey.StartsWith("0x", StringComparison.OrdinalIgnoreCase) ? hexKey[2..] : hexKey);
    store.TryGet(key, out var value);
    Console.WriteLine(value is null ? "<missing>" : Convert.ToHexString(value).ToLowerInvariant());
    return 0;
}

static int DumpContractScript(string path, string contractHash)
{
    using IStore store = new LevelDBStore().GetStore(path);
    using var snapshot = new StoreCache(store);
    var hash = UInt160.Parse(contractHash);
    var contract = NativeContract.ContractManagement.GetContract(snapshot, hash);
    if (contract is null)
    {
        Console.Error.WriteLine($"contract {hash} not found");
        return 1;
    }

    var script = contract.Script.ToArray();
    Console.WriteLine($"contract={hash}");
    Console.WriteLine($"id={contract.Id}");
    Console.WriteLine($"updateCounter={contract.UpdateCounter}");
    Console.WriteLine($"type={contract.Type}");
    Console.WriteLine($"manifestName={contract.Manifest.Name}");
    Console.WriteLine($"scriptLen={script.Length}");
    Console.WriteLine($"scriptSha256={Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(script)).ToLowerInvariant()}");
    Console.WriteLine($"scriptBase64={Convert.ToBase64String(script)}");
    Console.WriteLine("methods:");
    foreach (var method in contract.Manifest.Abi.Methods.OrderBy(method => method.Offset))
        Console.WriteLine($"{method.Offset}\t{method.Name}\tparams={method.Parameters.Length}\treturn={method.ReturnType}");
    return 0;
}

static int DumpContractDisasm(string path, string contractHash, int startIp, int endIp)
{
    using IStore store = new LevelDBStore().GetStore(path);
    using var snapshot = new StoreCache(store);
    var hash = UInt160.Parse(contractHash);
    var contract = NativeContract.ContractManagement.GetContract(snapshot, hash);
    if (contract is null)
    {
        Console.Error.WriteLine($"contract {hash} not found");
        return 1;
    }

    var script = contract.Script.ToArray();
    Console.WriteLine($"contract={hash}");
    Console.WriteLine($"manifestName={contract.Manifest.Name}");
    Console.WriteLine($"scriptLen={script.Length}");
    Console.WriteLine($"range={startIp}..{endIp}");
    foreach (var instruction in new ProbeInstruction(script))
    {
        if (instruction.Position >= startIp && instruction.Position <= endIp)
            Console.WriteLine(instruction);
        if (instruction.Position > endIp)
            break;
    }

    return 0;
}

static int DumpStoragePrefix(string path, string contractHash, string prefixHex, int sampleLimit)
{
    using IStore store = new LevelDBStore().GetStore(path);
    using var snapshot = new StoreCache(store);
    var hash = UInt160.Parse(contractHash);
    var contract = NativeContract.ContractManagement.GetContract(snapshot, hash);
    if (contract is null)
    {
        Console.Error.WriteLine($"contract {hash} not found");
        return 1;
    }

    var prefix = ParseHex(prefixHex);
    var prefixKey = new StorageKey { Id = contract.Id, Key = prefix };
    var entries = snapshot.Find(prefixKey, SeekDirection.Forward).ToList();

    Console.WriteLine($"contract={hash}");
    Console.WriteLine($"id={contract.Id}");
    Console.WriteLine($"prefix={Convert.ToHexString(prefix).ToLowerInvariant()}");
    Console.WriteLine($"count={entries.Count}");

    foreach (var (key, value) in entries.Take(Math.Max(0, sampleLimit)))
    {
        Console.WriteLine($"key={Convert.ToHexString(key.Key.Span).ToLowerInvariant()}\tvalueLen={value.Value.Length}\tvalue={DescribeValue(value.Value)}");
    }

    return 0;
}

static async Task<int> ReplayTransaction(string path, uint blockIndex, string txHashText, string hostLib, string rpcUrl)
{
    if (!File.Exists(hostLib))
    {
        Console.Error.WriteLine($"host library not found: {hostLib}");
        return 1;
    }

    var block = await FetchBlock(rpcUrl, blockIndex);
    var txHash = UInt256.Parse(txHashText);
    var tx = block.Transactions.SingleOrDefault(t => t.Hash == txHash);
    if (tx is null)
    {
        Console.Error.WriteLine($"transaction {txHash} not found in block {blockIndex}");
        return 1;
    }

    using IStore store = new LevelDBStore().GetStore(path);
    using var snapshot = new StoreCache(store);
    using var bridge = new NativeRiscvVmBridge(hostLib);
    using var provider = new RiscvApplicationEngineProvider(bridge);
    return ReplayTransactionWithProvider(block, tx, snapshot, provider);
}

static async Task<int> ReplayTransactionNeoVm(string path, uint blockIndex, string txHashText, string rpcUrl)
{
    var block = await FetchBlock(rpcUrl, blockIndex);
    var txHash = UInt256.Parse(txHashText);
    var tx = block.Transactions.SingleOrDefault(t => t.Hash == txHash);
    if (tx is null)
    {
        Console.Error.WriteLine($"transaction {txHash} not found in block {blockIndex}");
        return 1;
    }

    using IStore store = new LevelDBStore().GetStore(path);
    using var snapshot = new StoreCache(store);
    var provider = new NeoVMHostApplicationEngineProvider();
    return ReplayTransactionWithProvider(block, tx, snapshot, provider);
}

static int ReplayTransactionWithProvider(Block block, Transaction tx, StoreCache snapshot, IApplicationEngineProvider provider)
{
    ApplicationEngine.Provider = provider;

    using var engine = ApplicationEngine.Create(
        TriggerType.Application,
        tx,
        snapshot,
        block,
        MainnetSettings(),
        tx.SystemFee);
    engine.LoadScript(tx.Script);
    var state = engine.Execute();

    Console.WriteLine($"block={block.Index}");
    Console.WriteLine($"tx={tx.Hash}");
    Console.WriteLine($"state={state}");
    Console.WriteLine($"gas_consumed={engine.FeeConsumed}");
    Console.WriteLine($"gas_left={engine.GasLeft}");
    Console.WriteLine($"result_stack={engine.ResultStack.Count}");
    foreach (var (item, index) in engine.ResultStack.Reverse().Select((item, index) => (item, index)))
        Console.WriteLine($"result[{index}]={DescribeStackItem(item)}");
    Console.WriteLine($"notifications={engine.Notifications.Count}");
    foreach (var notification in engine.Notifications)
        Console.WriteLine($"notification={notification.ScriptHash}:{notification.EventName}:{DescribeStackItem(notification.State)}");

    if (engine.FaultException is not null)
    {
        Console.WriteLine($"fault={engine.FaultException.GetType().FullName}: {engine.FaultException.Message}");
        if (engine.FaultException.InnerException is not null)
            Console.WriteLine($"inner={engine.FaultException.InnerException.GetType().FullName}: {engine.FaultException.InnerException.Message}");
    }

    return state == VMState.HALT ? 0 : 3;
}

static async Task<Block> FetchBlock(string rpcUrl, uint index)
{
    using var http = new HttpClient { Timeout = TimeSpan.FromSeconds(60) };
    var payload = $$"""{"jsonrpc":"2.0","method":"getblock","params":[{{index}},false],"id":1}""";
    using var response = await http.PostAsync(rpcUrl, new StringContent(payload, Encoding.UTF8, "application/json"));
    response.EnsureSuccessStatusCode();
    using var document = JsonDocument.Parse(await response.Content.ReadAsStringAsync());
    if (document.RootElement.TryGetProperty("error", out var error))
        throw new InvalidOperationException(error.ToString());
    var base64 = document.RootElement.GetProperty("result").GetString()
        ?? throw new InvalidOperationException("RPC returned no block payload.");
    return Convert.FromBase64String(base64).AsSerializable<Block>();
}

static ProtocolSettings MainnetSettings() => ProtocolSettings.Default with
{
    Network = 860833102u,
    AddressVersion = 53,
    MillisecondsPerBlock = 15000,
    MaxTransactionsPerBlock = 512,
    MemoryPoolMaxTransactions = 50000,
    MaxTraceableBlocks = 2102400,
    Hardforks = new Dictionary<Hardfork, uint>
    {
        [Hardfork.HF_Aspidochelone] = 1730000,
        [Hardfork.HF_Basilisk] = 4120000,
        [Hardfork.HF_Cockatrice] = 5450000,
        [Hardfork.HF_Domovoi] = 5570000,
        [Hardfork.HF_Echidna] = 7300000,
        [Hardfork.HF_Faun] = 8800000,
    }.ToImmutableDictionary(),
    InitialGasDistribution = 5200000000000000,
    ValidatorsCount = 7,
    StandbyCommittee =
    [
        ECPoint.Parse("03b209fd4f53a7170ea4444e0cb0a6bb6a53c2bd016926989cf85f9b0fba17a70c", ECCurve.Secp256r1),
        ECPoint.Parse("02df48f60e8f3e01c48ff40b9b7f1310d7a8b2a193188befe1c2e3df740e895093", ECCurve.Secp256r1),
        ECPoint.Parse("03b8d9d5771d8f513aa0869b9cc8d50986403b78c6da36890638c3d46a5adce04a", ECCurve.Secp256r1),
        ECPoint.Parse("02ca0e27697b9c248f6f16e085fd0061e26f44da85b58ee835c110caa5ec3ba554", ECCurve.Secp256r1),
        ECPoint.Parse("024c7b7fb6c310fccf1ba33b082519d82964ea93868d676662d4a59ad548df0e7d", ECCurve.Secp256r1),
        ECPoint.Parse("02aaec38470f6aad0042c6e877cfd8087d2676b0f516fddd362801b9bd3936399e", ECCurve.Secp256r1),
        ECPoint.Parse("02486fd15702c4490a26703112a5cc1d0923fd697a33406bd5a1c00e0013b09a70", ECCurve.Secp256r1),
        ECPoint.Parse("023a36c72844610b4d34d1968662424011bf783ca9d984efa19a20babf5582f3fe", ECCurve.Secp256r1),
        ECPoint.Parse("03708b860c1de5d87f5b151a12c2a99feebd2e8b315ee8e7cf8aa19692a9e18379", ECCurve.Secp256r1),
        ECPoint.Parse("03c6aa6e12638b36e88adc1ccdceac4db9929575c3e03576c617c49cce7114a050", ECCurve.Secp256r1),
        ECPoint.Parse("03204223f8c86b8cd5c89ef12e4f0dbb314172e9241e30c9ef2293790793537cf0", ECCurve.Secp256r1),
        ECPoint.Parse("02a62c915cf19c7f19a50ec217e79fac2439bbaad658493de0c7d8ffa92ab0aa62", ECCurve.Secp256r1),
        ECPoint.Parse("03409f31f0d66bdc2f70a9730b66fe186658f84a8018204db01c106edc36553cd0", ECCurve.Secp256r1),
        ECPoint.Parse("0288342b141c30dc8ffcde0204929bb46aed5756b41ef4a56778d15ada8f0c6654", ECCurve.Secp256r1),
        ECPoint.Parse("020f2887f41474cfeb11fd262e982051c1541418137c02a0f4961af911045de639", ECCurve.Secp256r1),
        ECPoint.Parse("0222038884bbd1d8ff109ed3bdef3542e768eef76c1247aea8bc8171f532928c30", ECCurve.Secp256r1),
        ECPoint.Parse("03d281b42002647f0113f36c7b8efb30db66078dfaaa9ab3ff76d043a98d512fde", ECCurve.Secp256r1),
        ECPoint.Parse("02504acbc1f4b3bdad1d86d6e1a08603771db135a73e61c9d565ae06a1938cd2ad", ECCurve.Secp256r1),
        ECPoint.Parse("0226933336f1b75baa42d42b71d9091508b638046d19abd67f4e119bf64a7cfb4d", ECCurve.Secp256r1),
        ECPoint.Parse("03cdcea66032b82f5c30450e381e5295cae85c5e6943af716cc6b646352a6067dc", ECCurve.Secp256r1),
        ECPoint.Parse("02cd5a5547119e24feaa7c2a0f37b8c9366216bab7054de0065c9be42084003c8a", ECCurve.Secp256r1),
    ],
    SeedList =
    [
        "seed1.neo.org:10333",
        "seed2.neo.org:10333",
        "seed3.neo.org:10333",
        "seed4.neo.org:10333",
        "seed5.neo.org:10333",
    ],
};

static byte[] StateRootKey(uint index)
{
    var key = new byte[5];
    key[0] = 0x01;
    BinaryPrimitives.WriteUInt32BigEndian(key.AsSpan(1), index);
    return key;
}

static byte[] ParseHex(string value)
{
    return Convert.FromHexString(value.StartsWith("0x", StringComparison.OrdinalIgnoreCase) ? value[2..] : value);
}

static string DescribeValue(ReadOnlyMemory<byte> value)
{
    try
    {
        var item = BinarySerializer.Deserialize(value, ExecutionEngineLimits.Default, referenceCounter: null);
        return item switch
        {
            Neo.VM.Types.Struct @struct => $"Struct({@struct.Count})",
            Neo.VM.Types.Array array => $"Array({array.Count})",
            Neo.VM.Types.ByteString bytes => $"ByteString({bytes.GetSpan().Length})",
            Neo.VM.Types.Integer integer => $"Integer({integer.GetInteger()})",
            _ => item.GetType().Name,
        };
    }
    catch
    {
        return Convert.ToHexString(value.Span).ToLowerInvariant();
    }
}

static string DescribeStackItem(Neo.VM.Types.StackItem item)
{
    return item switch
    {
        Neo.VM.Types.Struct @struct => $"Struct({@struct.Count})",
        Neo.VM.Types.Array array => "Array[" + string.Join(",", Enumerable.Range(0, Math.Min(array.Count, 8)).Select(i => DescribeStackItem(array[i]))) + (array.Count > 8 ? ",..." : "") + "]",
        Neo.VM.Types.Map map => $"Map({map.Count})",
        Neo.VM.Types.ByteString bytes => $"ByteString({Convert.ToHexString(bytes.GetSpan()).ToLowerInvariant()})",
        Neo.VM.Types.Buffer bytes => $"Buffer({Convert.ToHexString(bytes.GetSpan()).ToLowerInvariant()})",
        Neo.VM.Types.Integer integer => $"Integer({integer.GetInteger()})",
        Neo.VM.Types.Boolean boolean => $"Boolean({boolean.GetBoolean()})",
        Neo.VM.Types.Null => "Null",
        Neo.VM.Types.InteropInterface interop => $"Interop({interop.GetInterface<object>()?.GetType().FullName ?? "null"})",
        _ => item.GetType().Name,
    };
}

sealed class ProbeInstruction : IEnumerable<ProbeInstruction>
{
    private const int OpCodeSize = 1;
    private static readonly int[] OperandSizeTable = new int[256];
    private static readonly int[] OperandSizePrefixTable = new int[256];
    private readonly ReadOnlyMemory<byte> script;

    static ProbeInstruction()
    {
        foreach (var field in typeof(OpCode).GetFields(BindingFlags.Public | BindingFlags.Static))
        {
            var attr = field.GetCustomAttribute<OperandSizeAttribute>();
            if (attr is null) continue;

            var index = (uint)(OpCode)field.GetValue(null)!;
            OperandSizeTable[index] = attr.Size;
            OperandSizePrefixTable[index] = attr.SizePrefix;
        }
    }

    public ProbeInstruction(ReadOnlyMemory<byte> script, int start = 0)
    {
        if (script.IsEmpty)
            throw new InvalidDataException("Bad script.");

        this.script = script;
        Position = start;
        OpCode = (OpCode)script.Span[start];
        OperandPrefixSize = OperandSizePrefixTable[(int)OpCode];
        OperandSize = OperandPrefixSize switch
        {
            0 => OperandSizeTable[(int)OpCode],
            1 => script.Span[start + 1],
            2 => BinaryPrimitives.ReadUInt16LittleEndian(script.Span[(start + 1)..]),
            4 => unchecked((int)BinaryPrimitives.ReadUInt32LittleEndian(script.Span[(start + 1)..])),
            _ => throw new InvalidDataException($"Invalid opcode prefix at position {start}."),
        };
        OperandSize += OperandPrefixSize;
        Operand = script.Slice(start + OpCodeSize, OperandSize);
    }

    public int Position { get; }
    public OpCode OpCode { get; }
    public ReadOnlyMemory<byte> Operand { get; }
    public int OperandSize { get; }
    public int OperandPrefixSize { get; }

    public IEnumerator<ProbeInstruction> GetEnumerator()
    {
        ProbeInstruction? instruction = this;
        for (var ip = Position; ip < script.Length; ip += instruction.OperandSize + OpCodeSize)
        {
            instruction = new ProbeInstruction(script, ip);
            yield return instruction;
        }
    }

    System.Collections.IEnumerator System.Collections.IEnumerable.GetEnumerator() => GetEnumerator();

    public override string ToString()
    {
        return $"{Position:D5} 0x{(byte)OpCode:x2} {OpCode,-12} {DecodeOperand()}";
    }

    private T AsToken<T>(uint index = 0)
        where T : unmanaged
    {
        var bytes = Operand[..OperandSize].ToArray();
        return Unsafe.As<byte, T>(ref bytes[index]);
    }

    private string DecodeOperand()
    {
        var operand = Operand[OperandPrefixSize..].ToArray();
        var asStr = Encoding.UTF8.GetString(operand);
        var readable = asStr.All(ch => ch is >= ' ' and <= '~');

        return OpCode switch
        {
            OpCode.JMP or OpCode.JMPIF or OpCode.JMPIFNOT or OpCode.JMPEQ or OpCode.JMPNE or
            OpCode.JMPGT or OpCode.JMPLT or OpCode.CALL or OpCode.ENDTRY =>
                $"[{checked(Position + AsToken<sbyte>()):D5}]",
            OpCode.JMP_L or OpCode.JMPIF_L or OpCode.PUSHA or OpCode.JMPIFNOT_L or
            OpCode.JMPEQ_L or OpCode.JMPNE_L or OpCode.JMPGT_L or OpCode.JMPLT_L or
            OpCode.CALL_L or OpCode.ENDTRY_L => $"[{checked(Position + AsToken<int>()):D5}]",
            OpCode.TRY => $"catch=[{checked(Position + AsToken<sbyte>()):D5}] finally=[{checked(Position + AsToken<sbyte>(1)):D5}]",
            OpCode.TRY_L => $"catch=[{checked(Position + AsToken<int>()):D5}] finally=[{checked(Position + AsToken<int>(4)):D5}]",
            OpCode.INITSLOT => $"{AsToken<byte>()}, {AsToken<byte>(1)}",
            OpCode.INITSSLOT or OpCode.LDARG or OpCode.STARG or OpCode.LDLOC or OpCode.STLOC or
            OpCode.LDSFLD or OpCode.STSFLD => $"{AsToken<byte>()}",
            OpCode.NEWARRAY_T or OpCode.ISTYPE or OpCode.CONVERT => $"0x{AsToken<byte>():x2}",
            OpCode.PUSHINT8 => $"{AsToken<sbyte>()}",
            OpCode.PUSHINT16 => $"{AsToken<short>()}",
            OpCode.PUSHINT32 => $"{AsToken<int>()}",
            OpCode.PUSHINT64 => $"{AsToken<long>()}",
            OpCode.PUSHINT128 or OpCode.PUSHINT256 => $"{new BigInteger(operand)}",
            OpCode.SYSCALL => DecodeSyscall(Unsafe.As<byte, uint>(ref operand[0])),
            OpCode.PUSHDATA1 or OpCode.PUSHDATA2 or OpCode.PUSHDATA4 =>
                readable ? $"{Convert.ToHexString(operand)} // {asStr}" : Convert.ToHexString(operand),
            _ => operand.Length == 0 ? "" : readable ? $"\"{asStr}\"" : Convert.ToHexString(operand),
        };
    }

    private static string DecodeSyscall(uint api)
    {
        return ApplicationEngine.Services.TryGetValue(api, out var service)
            ? $"0x{api:x8} [{service.Name}]"
            : $"0x{api:x8}";
    }
}
