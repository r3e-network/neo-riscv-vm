using Neo;
using Neo.Extensions;
using Neo.Cryptography.ECC;
using Neo.VM;
using Neo.VM.Types;
using Neo.Network.P2P.Payloads;
using Neo.Persistence;
using Neo.Persistence.Providers;
using Neo.SmartContract;
using Neo.SmartContract.Native;
using Neo.SmartContract.RiscV;
using System.Text.Json;
using System.Text;

static void EmitPushBytes(ScriptBuilder sb, string value)
{
    sb.EmitPush(Encoding.UTF8.GetBytes(value));
}

static byte[] BuildPermissionMapKeysScript()
{
    using var script = new ScriptBuilder();
    script.Emit(OpCode.NEWMAP);
    foreach (var key in PermissionKeys())
    {
        script.Emit(OpCode.DUP);
        EmitPushBytes(script, key);
        script.EmitPush(true);
        script.Emit(OpCode.SETITEM);
    }
    script.Emit(OpCode.KEYS);
    script.Emit(OpCode.RET);
    return script.ToArray();
}

static byte[] BuildPermissionMapSerializeScript()
{
    using var script = new ScriptBuilder();
    script.Emit(OpCode.NEWMAP);
    foreach (var key in PermissionKeys())
    {
        script.Emit(OpCode.DUP);
        EmitPushBytes(script, key);
        script.EmitPush(true);
        script.Emit(OpCode.SETITEM);
    }

    script.EmitPush(1);
    script.Emit(OpCode.PACK);
    script.EmitPush((int)CallFlags.All);
    script.EmitPush("serialize");
    script.EmitPush(NativeContract.StdLib.Hash.GetSpan().ToArray());
    script.EmitSysCall(ApplicationEngine.System_Contract_Call);
    script.Emit(OpCode.RET);
    return script.ToArray();
}

static string[] PermissionKeys() =>
[
    "offline_mint",
    "contract_upgrade",
    "set_mint_fee",
    "create_epoch",
    "set_permissions",
];

static string Describe(StackItem item)
{
    return item switch
    {
        ByteString bytes => Convert.ToHexString(bytes.GetSpan()),
        Integer integer => integer.GetInteger().ToString(),
        Neo.VM.Types.Boolean boolean => boolean.GetBoolean().ToString(),
        Neo.VM.Types.Array array => "[" + string.Join(", ", Enumerable.Range(0, array.Count).Select(i => Describe(array[i]))) + "]",
        _ => item.GetType().Name,
    };
}

static StackItem[] RunApplication(byte[] script, IApplicationEngineProvider provider)
{
    ApplicationEngine.Provider = provider;
    var settings = ProtocolSettings.Default with
    {
        Network = 0x334F454Eu,
        StandbyCommittee =
        [
            ECPoint.Parse("0278ed78c917797b637a7ed6e7a9d94e8c408444c41ee4c0a0f310a256b9271eda", ECCurve.Secp256r1)
        ],
        ValidatorsCount = 1,
    };
    using var system = new NeoSystem(settings, new MemoryStoreProvider());
    using var snapshot = system.GetSnapshotCache();
    using var engine = ApplicationEngine.Create(
        TriggerType.Application,
        new Transaction
        {
            Signers = [],
            Attributes = [],
            Script = script,
            Witnesses = [],
        },
        snapshot,
        system.GenesisBlock,
        system.Settings,
        2000_00000000);
    engine.LoadScript(script);
    var state = engine.Execute();
    Console.WriteLine($"application state={state}");
    if (state != VMState.HALT)
        throw engine.FaultException ?? new InvalidOperationException("FAULT");
    return engine.ResultStack.ToArray();
}

static async Task<Block> FetchBlock(uint index)
{
    using var http = new HttpClient { Timeout = TimeSpan.FromSeconds(30) };
    var payload = $$"""{"jsonrpc":"2.0","method":"getblock","params":[{{index}},false],"id":1}""";
    using var response = await http.PostAsync(
        "http://seed1.neo.org:10332",
        new StringContent(payload, Encoding.UTF8, "application/json"));
    response.EnsureSuccessStatusCode();
    using var document = JsonDocument.Parse(await response.Content.ReadAsStringAsync());
    var base64 = document.RootElement.GetProperty("result").GetString()
        ?? throw new InvalidOperationException("RPC returned no block payload.");
    return Convert.FromBase64String(base64).AsSerializable<Block>();
}

static string ComputeStateFingerprint(DataCache snapshot)
{
    using var sha = System.Security.Cryptography.SHA256.Create();
    var entries = snapshot.Find(keyPrefix: (StorageKey?)null)
        .OrderBy(e => e.Key.ToArray(), ByteArrayComparer.Instance)
        .ToList();

    foreach (var (key, value) in entries)
    {
        var keyBytes = key.ToArray();
        var valueBytes = value.Value.ToArray();
        sha.TransformBlock(BitConverter.GetBytes(keyBytes.Length), 0, 4, null, 0);
        sha.TransformBlock(keyBytes, 0, keyBytes.Length, null, 0);
        sha.TransformBlock(BitConverter.GetBytes(valueBytes.Length), 0, 4, null, 0);
        sha.TransformBlock(valueBytes, 0, valueBytes.Length, null, 0);
    }

    sha.TransformFinalBlock([], 0, 0);
    return Convert.ToHexString(sha.Hash!).ToLowerInvariant();
}

static (VMState State, string Fingerprint, string? Fault, Dictionary<string, string> PuppetStorage) RunDeployTransaction(
    Block block,
    IApplicationEngineProvider provider)
{
    ApplicationEngine.Provider = provider;
    var settings = ProtocolSettings.Default with
    {
        Network = 860833102u,
        AddressVersion = 53,
        MillisecondsPerBlock = 15000,
        MaxTransactionsPerBlock = 512,
        MemoryPoolMaxTransactions = 50000,
        MaxTraceableBlocks = 2102400,
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
        ],
        SeedList = [],
    };
    using var system = new NeoSystem(settings, new MemoryStoreProvider());
    using var snapshot = system.GetSnapshotCache();
    var tx = block.Transactions.Single();
    using var engine = ApplicationEngine.Create(
        TriggerType.Application,
        tx,
        snapshot,
        block,
        system.Settings,
        2000_00000000);
    engine.LoadScript(tx.Script);
    var state = engine.Execute();
    if (state == VMState.HALT)
    {
        engine.SnapshotCache.Commit();
        snapshot.Commit();
    }

    var storage = new Dictionary<string, string>(StringComparer.Ordinal);
    var deployed = NativeContract.ContractManagement.ListContracts(snapshot)
        .SingleOrDefault(contract => contract.Manifest.Name == "puppet");
    if (deployed is not null)
    {
        foreach (var (key, value) in snapshot.Find(new StorageKey { Id = deployed.Id }))
            storage[Convert.ToHexString(key.Key.Span)] = Convert.ToHexString(value.Value.Span);
    }

    return (state, ComputeStateFingerprint(snapshot), engine.FaultException?.ToString(), storage);
}

Console.WriteLine("NeoVM KEYS order:");
using (var engine = new ExecutionEngine())
{
    engine.LoadScript(BuildPermissionMapKeysScript());
    var state = engine.Execute();
    Console.WriteLine($"state={state}");
    var keys = (Neo.VM.Types.Array)engine.ResultStack.Pop();
    for (var i = 0; i < keys.Count; i++)
    {
        var bytes = ((ByteString)keys[i]).GetSpan().ToArray();
        Console.WriteLine($"{i}: {Encoding.UTF8.GetString(bytes)}");
    }
}

var serializeScript = BuildPermissionMapSerializeScript();
Console.WriteLine("NeoVM serialize:");
var neoVmStack = RunApplication(serializeScript, new NeoVMHostApplicationEngineProvider());
foreach (var item in neoVmStack) Console.WriteLine(Describe(item));

Console.WriteLine("RISC-V serialize:");
var hostLib = Environment.GetEnvironmentVariable(NativeRiscvVmBridge.LibraryPathEnvironmentVariable)
    ?? throw new InvalidOperationException($"{NativeRiscvVmBridge.LibraryPathEnvironmentVariable} is not set.");
using var riscvProvider = new RiscvApplicationEngineProvider(new NativeRiscvVmBridge(hostLib));
var riscvStack = RunApplication(serializeScript, riscvProvider);
foreach (var item in riscvStack) Console.WriteLine(Describe(item));

if (Describe(neoVmStack[0]) != Describe(riscvStack[0]))
{
    Console.WriteLine("SERIALIZE_MISMATCH");
}
else
{
    Console.WriteLine("SERIALIZE_MATCH");
}

Console.WriteLine("Fetching block 1267715 for fresh deploy replay...");
var block1267715 = await FetchBlock(1267715);
var neoDeploy = RunDeployTransaction(block1267715, new NeoVMHostApplicationEngineProvider());
Console.WriteLine($"NeoVM deploy state={neoDeploy.State} fp={neoDeploy.Fingerprint} storage={neoDeploy.PuppetStorage.Count}");
if (neoDeploy.Fault is not null) Console.WriteLine(neoDeploy.Fault);
var riscvDeploy = RunDeployTransaction(block1267715, riscvProvider);
Console.WriteLine($"RISC-V deploy state={riscvDeploy.State} fp={riscvDeploy.Fingerprint} storage={riscvDeploy.PuppetStorage.Count}");
if (riscvDeploy.Fault is not null) Console.WriteLine(riscvDeploy.Fault);
Console.WriteLine(neoDeploy.Fingerprint == riscvDeploy.Fingerprint ? "FRESH_DEPLOY_MATCH" : "FRESH_DEPLOY_MISMATCH");
foreach (var key in neoDeploy.PuppetStorage.Keys.Concat(riscvDeploy.PuppetStorage.Keys).Distinct().OrderBy(k => k, StringComparer.Ordinal))
{
    neoDeploy.PuppetStorage.TryGetValue(key, out var neoValue);
    riscvDeploy.PuppetStorage.TryGetValue(key, out var riscvValue);
    if (neoValue != riscvValue)
    {
        Console.WriteLine($"storage diff key={key} neo={neoValue ?? "<missing>"} riscv={riscvValue ?? "<missing>"}");
    }
}

sealed class ByteArrayComparer : IComparer<byte[]>
{
    public static readonly ByteArrayComparer Instance = new();

    public int Compare(byte[]? x, byte[]? y)
    {
        if (x is null && y is null) return 0;
        if (x is null) return -1;
        if (y is null) return 1;
        var minLen = Math.Min(x.Length, y.Length);
        for (var i = 0; i < minLen; i++)
        {
            var cmp = x[i].CompareTo(y[i]);
            if (cmp != 0) return cmp;
        }
        return x.Length.CompareTo(y.Length);
    }
}
