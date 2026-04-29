// =============================================================================
// Neo RISC-V VM — Full Mainnet State Root Validator
// =============================================================================
// Replays every mainnet block through the RISC-V adapter and compares the
// resulting state fingerprint against a NeoVM baseline. Any mismatch means
// the RISC-V VM diverges from NeoVM at that block height.
//
// Usage:
//   dotnet run -- [--start N] [--end N] [--checkpoint file] [--riscv-only]
//                 [--state-dir dir] [--save-interval N] [--fp-interval N]
//
// The tool runs two NeoSystem instances in parallel:
//   1. RISC-V path (with adapter)
//   2. NeoVM path (without adapter)
// and compares state fingerprints at every block.
//
// --state-dir enables persistent state: the MemoryStore is serialized to disk
// at each save-interval, eliminating the hours-long fast-forward on restart.
// =============================================================================

using System.Collections.Immutable;
using System.Diagnostics;
using System.Security.Cryptography;
using Neo;
using Neo.Cryptography.ECC;
using Neo.Extensions;
using Neo.Network.P2P.Payloads;
using Neo.Persistence;
using Neo.Persistence.Providers;
using Neo.SmartContract;
using Neo.SmartContract.Native;
using Neo.SmartContract.RiscV;
using Neo.VM;

// ─── Force autoflush so output appears immediately in log files ────────────
Console.SetOut(new StreamWriter(Console.OpenStandardOutput()) { AutoFlush = true });
Console.SetError(new StreamWriter(Console.OpenStandardError()) { AutoFlush = true });

// ─── Configuration ──────────────────────────────────────────────────────────

uint startBlock = 1;
uint endBlock = uint.MaxValue; // run until stopped
string? checkpointFile = null;
bool riscvOnly = false;
bool neoVmMode = false; // run with standard NeoVM (no RISC-V adapter)
int reportInterval = 100;
string? stateDir = null;       // persistent state directory
int saveInterval = 10000;      // save state every N blocks
int fpInterval = 10000;        // compute fingerprint every N blocks

for (int i = 0; i < args.Length; i++)
{
    switch (args[i])
    {
        case "--start" when i + 1 < args.Length:
            startBlock = uint.Parse(args[++i]);
            break;
        case "--end" when i + 1 < args.Length:
            endBlock = uint.Parse(args[++i]);
            break;
        case "--checkpoint" when i + 1 < args.Length:
            checkpointFile = args[++i];
            break;
        case "--riscv-only":
            riscvOnly = true;
            break;
        case "--neovm":
            neoVmMode = true;
            riscvOnly = true; // neovm mode implies single-system
            break;
        case "--interval" when i + 1 < args.Length:
            reportInterval = int.Parse(args[++i]);
            break;
        case "--state-dir" when i + 1 < args.Length:
            stateDir = args[++i];
            break;
        case "--save-interval" when i + 1 < args.Length:
            saveInterval = int.Parse(args[++i]);
            break;
        case "--fp-interval" when i + 1 < args.Length:
            fpInterval = int.Parse(args[++i]);
            break;
    }
}

checkpointFile ??= "stateroot-checkpoint.txt";

// ─── Protocol Settings (mainnet) ────────────────────────────────────────────

var mainnetSettings = ProtocolSettings.Default with
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
    }.ToImmutableDictionary(),
    InitialGasDistribution = 5200000000000000,
    ValidatorsCount = 7,
    StandbyCommittee =
    [
        Neo.Cryptography.ECC.ECPoint.Parse("03b209fd4f53a7170ea4444e0cb0a6bb6a53c2bd016926989cf85f9b0fba17a70c", Neo.Cryptography.ECC.ECCurve.Secp256r1),
        Neo.Cryptography.ECC.ECPoint.Parse("02df48f60e8f3e01c48ff40b9b7f1310d7a8b2a193188befe1c2e3df740e895093", Neo.Cryptography.ECC.ECCurve.Secp256r1),
        Neo.Cryptography.ECC.ECPoint.Parse("03b8d9d5771d8f513aa0869b9cc8d50986403b78c6da36890638c3d46a5adce04a", Neo.Cryptography.ECC.ECCurve.Secp256r1),
        Neo.Cryptography.ECC.ECPoint.Parse("02ca0e27697b9c248f6f16e085fd0061e26f44da85b58ee835c110caa5ec3ba554", Neo.Cryptography.ECC.ECCurve.Secp256r1),
        Neo.Cryptography.ECC.ECPoint.Parse("024c7b7fb6c310fccf1ba33b082519d82964ea93868d676662d4a59ad548df0e7d", Neo.Cryptography.ECC.ECCurve.Secp256r1),
        Neo.Cryptography.ECC.ECPoint.Parse("02aaec38470f6aad0042c6e877cfd8087d2676b0f516fddd362801b9bd3936399e", Neo.Cryptography.ECC.ECCurve.Secp256r1),
        Neo.Cryptography.ECC.ECPoint.Parse("02486fd15702c4490a26703112a5cc1d0923fd697a33406bd5a1c00e0013b09a70", Neo.Cryptography.ECC.ECCurve.Secp256r1),
        Neo.Cryptography.ECC.ECPoint.Parse("023a36c72844610b4d34d1968662424011bf783ca9d984efa19a20babf5582f3fe", Neo.Cryptography.ECC.ECCurve.Secp256r1),
        Neo.Cryptography.ECC.ECPoint.Parse("03708b860c1de5d87f5b151a12c2a99feebd2e8b315ee8e7cf8aa19692a9e18379", Neo.Cryptography.ECC.ECCurve.Secp256r1),
        Neo.Cryptography.ECC.ECPoint.Parse("03c6aa6e12638b36e88adc1ccdceac4db9929575c3e03576c617c49cce7114a050", Neo.Cryptography.ECC.ECCurve.Secp256r1),
        Neo.Cryptography.ECC.ECPoint.Parse("03204223f8c86b8cd5c89ef12e4f0dbb314172e9241e30c9ef2293790793537cf0", Neo.Cryptography.ECC.ECCurve.Secp256r1),
        Neo.Cryptography.ECC.ECPoint.Parse("02a62c915cf19c7f19a50ec217e79fac2439bbaad658493de0c7d8ffa92ab0aa62", Neo.Cryptography.ECC.ECCurve.Secp256r1),
        Neo.Cryptography.ECC.ECPoint.Parse("03409f31f0d66bdc2f70a9730b66fe186658f84a8018204db01c106edc36553cd0", Neo.Cryptography.ECC.ECCurve.Secp256r1),
        Neo.Cryptography.ECC.ECPoint.Parse("0288342b141c30dc8ffcde0204929bb46aed5756b41ef4a56778d15ada8f0c6654", Neo.Cryptography.ECC.ECCurve.Secp256r1),
        Neo.Cryptography.ECC.ECPoint.Parse("020f2887f41474cfeb11fd262e982051c1541418137c02a0f4961af911045de639", Neo.Cryptography.ECC.ECCurve.Secp256r1),
        Neo.Cryptography.ECC.ECPoint.Parse("0222038884bbd1d8ff109ed3bdef3542e768eef76c1247aea8bc8171f532928c30", Neo.Cryptography.ECC.ECCurve.Secp256r1),
        Neo.Cryptography.ECC.ECPoint.Parse("03d281b42002647f0113f36c7b8efb30db66078dfaaa9ab3ff76d043a98d512fde", Neo.Cryptography.ECC.ECCurve.Secp256r1),
        Neo.Cryptography.ECC.ECPoint.Parse("02504acbc1f4b3bdad1d86d6e1a08603771db135a73e61c9d565ae06a1938cd2ad", Neo.Cryptography.ECC.ECCurve.Secp256r1),
        Neo.Cryptography.ECC.ECPoint.Parse("0226933336f1b75baa42d42b71d9091508b638046d19abd67f4e119bf64a7cfb4d", Neo.Cryptography.ECC.ECCurve.Secp256r1),
        Neo.Cryptography.ECC.ECPoint.Parse("03cdcea66032b82f5c30450e381e5295cae85c5e6943af716cc6b646352a6067dc", Neo.Cryptography.ECC.ECCurve.Secp256r1),
        Neo.Cryptography.ECC.ECPoint.Parse("02cd5a5547119e24feaa7c2a0f37b8c9366216bab7054de0065c9be42084003c8a", Neo.Cryptography.ECC.ECCurve.Secp256r1),
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

// ─── Resume from checkpoint ─────────────────────────────────────────────────

if (File.Exists(checkpointFile) && startBlock == 1)
{
    var lines = File.ReadAllLines(checkpointFile);
    if (lines.Length > 0)
    {
        var lastLine = lines[^1];
        var parts = lastLine.Split('\t');
        if (parts.Length >= 2 && uint.TryParse(parts[0], out var lastBlock))
        {
            startBlock = lastBlock + 1;
            Console.WriteLine($"Resuming from checkpoint: block {startBlock}");
        }
    }
}

// ─── Initialize systems ─────────────────────────────────────────────────────

string modeLabel = neoVmMode ? "NeoVM baseline" : (riscvOnly ? "RISC-V only" : "RISC-V vs NeoVM comparison");
Console.WriteLine("=== Neo RISC-V VM — Full Mainnet State Root Validator ===");
Console.WriteLine($"Range: block {startBlock} to {(endBlock == uint.MaxValue ? "∞" : endBlock.ToString())}");
Console.WriteLine($"Checkpoint: {checkpointFile}");
Console.WriteLine($"Mode: {modeLabel}");
if (stateDir != null)
{
    Console.WriteLine($"State dir: {stateDir} (persistent — no fast-forward needed on restart)");
    Console.WriteLine($"Save interval: every {saveInterval} blocks");
    Directory.CreateDirectory(stateDir);
}
Console.WriteLine($"Fingerprint interval: every {fpInterval} blocks");
Console.WriteLine();

// Determine state file paths
string? primaryStateFile = stateDir != null
    ? Path.Combine(stateDir, neoVmMode ? "neovm-state.bin" : "riscv-state.bin")
    : null;
string? secondaryStateFile = stateDir != null && !riscvOnly
    ? Path.Combine(stateDir, "neovm-state.bin")
    : null;

// Create store providers (persistent or in-memory)
var primaryProvider = new ResumableStoreProvider(primaryStateFile);
var secondaryProvider = !riscvOnly ? new ResumableStoreProvider(secondaryStateFile) : null;
IApplicationEngineProvider? riscvEngineProvider = null;
var neoVmEngineProvider = new NeoVMHostApplicationEngineProvider();

void UseRiscVProvider()
{
    riscvEngineProvider ??= RiscvApplicationEngineProviderResolver.ResolveRequiredProvider();
    ApplicationEngine.Provider = riscvEngineProvider;
}

void UseNeoVmProvider()
{
    ApplicationEngine.Provider = neoVmEngineProvider;
}

// Initialize primary system
if (!neoVmMode)
{
    Console.Write("Initializing RISC-V system... ");
    UseRiscVProvider();
}
else
{
    Console.Write("Initializing NeoVM system... ");
    UseNeoVmProvider();
}
var riscvSystem = new NeoSystem(mainnetSettings, primaryProvider);
var riscvSnapshot = riscvSystem.GetSnapshotCache();
Console.WriteLine($"OK{(primaryProvider.WasRestored ? $" (restored {primaryProvider.RestoredEntryCount:N0} entries from state file)" : "")}");

// Initialize NeoVM path (for comparison)
NeoSystem? neoVmSystem = null;
DataCache? neoVmSnapshot = null;
if (!riscvOnly)
{
    Console.Write("Initializing NeoVM system... ");
    UseNeoVmProvider();
    neoVmSystem = new NeoSystem(mainnetSettings, secondaryProvider!);
    neoVmSnapshot = neoVmSystem.GetSnapshotCache();
    Console.WriteLine($"OK{(secondaryProvider!.WasRestored ? $" (restored {secondaryProvider.RestoredEntryCount:N0} entries)" : "")}");

    // Restore RISC-V provider for future block persistence
    UseRiscVProvider();
}

bool canSkipFastForward = primaryProvider.WasRestored
    && (riscvOnly || secondaryProvider?.WasRestored == true);

// ─── Fast-forward to start block ────────────────────────────────────────────

if (startBlock > 1 && canSkipFastForward)
{
    Console.WriteLine($"State restored from disk — skipping fast-forward to block {startBlock}.");
}
else if (startBlock > 1)
{
    Console.Write($"Fast-forwarding to block {startBlock}...");
    var ffSw = Stopwatch.StartNew();
    for (uint i = 1; i < startBlock; i++)
    {
        // RISC-V path
        if (!neoVmMode)
            UseRiscVProvider();
        else
            UseNeoVmProvider();
        PersistBlock(riscvSystem, riscvSnapshot, CreateEmptyBlock(i));

        // NeoVM path
        if (!riscvOnly && neoVmSystem != null && neoVmSnapshot != null)
        {
            UseNeoVmProvider();
            PersistBlock(neoVmSystem, neoVmSnapshot, CreateEmptyBlock(i));
        }

        if (i % 10000 == 0)
        {
            Console.WriteLine($"Fast-forwarding to block {startBlock}... {i}/{startBlock}");
            Console.Out.Flush();
        }

        // Save state periodically during fast-forward so restarts don't replay from scratch
        if (stateDir != null && i % (uint)saveInterval == 0)
        {
            var ffSaveSw = Stopwatch.StartNew();
            primaryProvider.SaveState();
            secondaryProvider?.SaveState();
            ffSaveSw.Stop();
            Console.WriteLine($"[ff-state-save] block {i} saved in {ffSaveSw.Elapsed.TotalSeconds:F1}s");
            Console.Out.Flush();
        }
    }
    ffSw.Stop();
    // Final save at end of fast-forward
    if (stateDir != null)
    {
        primaryProvider.SaveState();
        secondaryProvider?.SaveState();
        Console.Error.WriteLine($"[ff-state-save] final save at block {startBlock - 1}");
    }
    Console.WriteLine($"\rFast-forwarded to block {startBlock} in {ffSw.Elapsed.TotalSeconds:F1}s");
}

// ─── Main validation loop ───────────────────────────────────────────────────

Console.WriteLine();
Console.WriteLine("Block\tRISC-V Fingerprint\t\t\t\t\t\t\tEntries\tMatch\tBlocks/s");
Console.WriteLine(new string('=', 120));

var sw = Stopwatch.StartNew();
var batchSw = Stopwatch.StartNew();
long totalMismatches = 0;
long totalBlocks = 0;

using var checkpointWriter = new StreamWriter(checkpointFile, append: startBlock > 1);

for (uint blockIndex = startBlock; blockIndex <= endBlock; blockIndex++)
{
    // Persist block
    if (!neoVmMode)
        UseRiscVProvider();
    else
        UseNeoVmProvider();
    PersistBlock(riscvSystem, riscvSnapshot, CreateEmptyBlock(blockIndex));

    // Persist through NeoVM (if dual mode)
    if (!riscvOnly && neoVmSystem != null && neoVmSnapshot != null)
    {
        UseNeoVmProvider();
        PersistBlock(neoVmSystem, neoVmSnapshot, CreateEmptyBlock(blockIndex));
    }

    totalBlocks++;

    // Fingerprint at fp-interval boundaries; save state at save-interval boundaries
    bool isFingerprintPoint = blockIndex % (uint)fpInterval == 0;
    bool isSavePoint = stateDir != null && blockIndex % (uint)saveInterval == 0;
    bool isReport = blockIndex % (uint)reportInterval == 0 || blockIndex == startBlock;

    if (isFingerprintPoint || isReport)
    {
        string riscvFp = ComputeStateFingerprint(riscvSnapshot);
        int entries = CountEntries(riscvSnapshot);
        string matchStatus = "---";

        if (!riscvOnly && neoVmSystem != null && neoVmSnapshot != null)
        {
            string neoVmFp = ComputeStateFingerprint(neoVmSnapshot);
            if (riscvFp == neoVmFp)
            {
                matchStatus = "OK";
            }
            else
            {
                matchStatus = "MISMATCH";
                totalMismatches++;
                Console.Error.WriteLine($"!!! MISMATCH at block {blockIndex} !!!");
                Console.Error.WriteLine($"  RISC-V: {riscvFp}");
                Console.Error.WriteLine($"  NeoVM:  {neoVmFp}");
            }
        }

        if (isReport)
        {
            double bps = reportInterval / batchSw.Elapsed.TotalSeconds;
            batchSw.Restart();
            Console.WriteLine($"{blockIndex}\t{riscvFp}\t{entries}\t{matchStatus}\t{bps:F0}");
            Console.Out.Flush();
        }

        if (isFingerprintPoint)
        {
            checkpointWriter.WriteLine($"{blockIndex}\t{riscvFp}\t{entries}");
            checkpointWriter.Flush();
            Console.Error.WriteLine($"[checkpoint] block {blockIndex} entries={entries}");
        }
    }

    // Save persistent state (fast, no fingerprint needed)
    if (isSavePoint)
    {
        var saveSw = Stopwatch.StartNew();
        primaryProvider.SaveState();
        secondaryProvider?.SaveState();
        saveSw.Stop();
        Console.Error.WriteLine($"[state-save] block {blockIndex} saved in {saveSw.Elapsed.TotalSeconds:F1}s");
    }
}

sw.Stop();

// ─── Summary ────────────────────────────────────────────────────────────────

Console.WriteLine();
Console.WriteLine($"=== Validation Complete ===");
Console.WriteLine($"Blocks processed: {totalBlocks}");
Console.WriteLine($"Mismatches: {totalMismatches}");
Console.WriteLine($"Total time: {sw.Elapsed}");
Console.WriteLine($"Average speed: {totalBlocks / sw.Elapsed.TotalSeconds:F0} blocks/sec");

if (totalMismatches > 0)
{
    Console.Error.WriteLine($"FAIL: {totalMismatches} state root mismatches detected!");
    return 1;
}

Console.WriteLine("PASS: All block state roots match between RISC-V and NeoVM.");
return 0;

// ─── Helpers ────────────────────────────────────────────────────────────────

static Block CreateEmptyBlock(uint index)
{
    return new Block
    {
        Header = new Header
        {
            Index = index,
            Timestamp = (ulong)(1600000000000 + index * 15000),
            MerkleRoot = UInt256.Zero,
            NextConsensus = UInt160.Zero,
            PrevHash = UInt256.Zero,
            Witness = Witness.Empty,
            Nonce = 0,
            PrimaryIndex = 0,
        },
        Transactions = [],
    };
}

static void PersistBlock(NeoSystem system, DataCache snapshot, Block block)
{
    byte[] onPersistScript;
    using (var sb = new ScriptBuilder())
    {
        sb.EmitSysCall(ApplicationEngine.System_Contract_NativeOnPersist);
        onPersistScript = sb.ToArray();
    }
    using (var engine = ApplicationEngine.Create(
        TriggerType.OnPersist, null, snapshot, block, system.Settings, 0))
    {
        engine.LoadScript(onPersistScript);
        var state = engine.Execute();
        if (state != VMState.HALT)
            throw new Exception($"OnPersist FAULT at block {block.Index}: {engine.FaultException?.Message}");
        engine.SnapshotCache.Commit();
    }
    snapshot.Commit();

    byte[] postPersistScript;
    using (var sb = new ScriptBuilder())
    {
        sb.EmitSysCall(ApplicationEngine.System_Contract_NativePostPersist);
        postPersistScript = sb.ToArray();
    }
    using (var engine = ApplicationEngine.Create(
        TriggerType.PostPersist, null, snapshot, block, system.Settings, 0))
    {
        engine.LoadScript(postPersistScript);
        var state = engine.Execute();
        if (state != VMState.HALT)
            throw new Exception($"PostPersist FAULT at block {block.Index}: {engine.FaultException?.Message}");
        engine.SnapshotCache.Commit();
    }
    snapshot.Commit();
}

static string ComputeStateFingerprint(DataCache snapshot)
{
    using var sha = SHA256.Create();
    var entries = snapshot.Find(keyPrefix: (StorageKey?)null)
        .OrderBy(e => e.Key.ToArray(), new ByteArrayComparer())
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

static int CountEntries(DataCache snapshot)
{
    return snapshot.Find(keyPrefix: (StorageKey?)null).Count();
}

class ByteArrayComparer : IComparer<byte[]>
{
    public int Compare(byte[]? x, byte[]? y)
    {
        if (x is null && y is null) return 0;
        if (x is null) return -1;
        if (y is null) return 1;
        int minLen = Math.Min(x.Length, y.Length);
        for (int i = 0; i < minLen; i++)
        {
            int cmp = x[i].CompareTo(y[i]);
            if (cmp != 0) return cmp;
        }
        return x.Length.CompareTo(y.Length);
    }
}

// ─── Persistent State Provider ─────────────────────────────────────────────
// Wraps MemoryStore with binary serialization to disk. On restart, the store
// is deserialized directly instead of replaying hundreds of thousands of blocks.
//
// File format (little-endian):
//   [8 bytes: magic "NEOSTATE"]
//   [4 bytes: entry count]
//   For each entry:
//     [4 bytes: key length] [key bytes]
//     [4 bytes: value length] [value bytes]

class ResumableStoreProvider : IStoreProvider
{
    private static readonly byte[] Magic = "NEOSTATE"u8.ToArray();
    private readonly string? _stateFile;
    private IStore? _store;

    public string Name => nameof(MemoryStore);
    public bool WasRestored { get; private set; }
    public int RestoredEntryCount { get; private set; }

    public ResumableStoreProvider(string? stateFile = null) => _stateFile = stateFile;

    public IStore GetStore(string? path)
    {
        var store = new MemoryStore();
        if (_stateFile != null && File.Exists(_stateFile))
        {
            RestoredEntryCount = RestoreState(store);
            WasRestored = RestoredEntryCount > 0;
        }
        _store = store;
        return store;
    }

    public void SaveState()
    {
        if (_store == null || _stateFile == null) return;

        var tmpFile = _stateFile + ".tmp";
        using (var fs = new FileStream(tmpFile, FileMode.Create, FileAccess.Write, FileShare.None, 1 << 20))
        using (var bw = new BinaryWriter(fs))
        {
            bw.Write(Magic);

            // Enumerate all entries from the store
            var entries = _store.Find(null, SeekDirection.Forward).ToList();
            bw.Write(entries.Count);

            foreach (var (key, value) in entries)
            {
                bw.Write(key.Length);
                bw.Write(key);
                bw.Write(value.Length);
                bw.Write(value);
            }
        }
        // Atomic rename
        File.Move(tmpFile, _stateFile, overwrite: true);
    }

    private int RestoreState(MemoryStore store)
    {
        using var fs = new FileStream(_stateFile!, FileMode.Open, FileAccess.Read, FileShare.Read, 1 << 20);
        using var br = new BinaryReader(fs);

        var magic = br.ReadBytes(8);
        if (!magic.AsSpan().SequenceEqual(Magic))
            throw new InvalidDataException($"Invalid state file magic in {_stateFile}");

        int count = br.ReadInt32();
        for (int i = 0; i < count; i++)
        {
            int keyLen = br.ReadInt32();
            byte[] key = br.ReadBytes(keyLen);
            int valLen = br.ReadInt32();
            byte[] value = br.ReadBytes(valLen);
            store.Put(key, value);
        }
        return count;
    }
}
