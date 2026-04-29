// =============================================================================
// Neo RISC-V VM — Full Block Validator (with real transaction replay)
// =============================================================================
// Fetches real mainnet blocks from an RPC node and replays every transaction
// through both the RISC-V adapter and standard NeoVM. Compares state
// fingerprints at configurable intervals to detect any divergence.
//
// Usage:
//   dotnet run -- --rpc <url> [--start N] [--end N] [--checkpoint file]
//                 [--riscv-only] [--neovm] [--state-dir dir]
//                 [--save-interval N] [--fp-interval N] [--batch-size N]
//
// The tool follows the exact Blockchain.Persist() flow:
//   1. Fetch real block from RPC (with transactions)
//   2. OnPersist — registers block and transactions in ledger
//   3. Execute each transaction script (Application trigger)
//   4. PostPersist — updates current block pointer
//   5. Final commit
// =============================================================================

using System.Collections.Immutable;
using System.Diagnostics;
using System.Net.Http;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using Neo;
using ECCurve = Neo.Cryptography.ECC.ECCurve;
using ECPoint = Neo.Cryptography.ECC.ECPoint;
using Neo.Extensions;
using Neo.IO;
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

string rpcUrl = "http://seed1.neo.org:10332";
uint startBlock = 1;
uint endBlock = uint.MaxValue;
string? checkpointFile = null;
bool riscvOnly = false;
bool neoVmMode = false;
int reportInterval = 100;
string? stateDir = null;
int saveInterval = 10000;
int fpInterval = 10000;
int batchSize = 100; // fetch blocks in parallel batches for performance
bool skipFaultyBlocks = false; // skip blocks that crash instead of aborting
var skipBlocks = new HashSet<uint>(); // specific block heights to skip entirely

for (int i = 0; i < args.Length; i++)
{
    switch (args[i])
    {
        case "--skip-block" when i + 1 < args.Length:
            foreach (var part in args[++i].Split(','))
                skipBlocks.Add(uint.Parse(part));
            break;
        case "--rpc" when i + 1 < args.Length:
            rpcUrl = args[++i];
            break;
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
            riscvOnly = true;
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
        case "--batch-size" when i + 1 < args.Length:
            batchSize = int.Parse(args[++i]);
            break;
        case "--skip-faulty":
            skipFaultyBlocks = true;
            break;
    }
}

checkpointFile ??= "fullblock-checkpoint.txt";

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

// ─── Precompile persistence scripts ─────────────────────────────────────────

byte[] onPersistScript;
using (var sb = new ScriptBuilder())
{
    sb.EmitSysCall(ApplicationEngine.System_Contract_NativeOnPersist);
    onPersistScript = sb.ToArray();
}

byte[] postPersistScript;
using (var sb = new ScriptBuilder())
{
    sb.EmitSysCall(ApplicationEngine.System_Contract_NativePostPersist);
    postPersistScript = sb.ToArray();
}

// ─── Initialize systems ─────────────────────────────────────────────────────

string modeLabel = neoVmMode ? "NeoVM baseline" : (riscvOnly ? "RISC-V only" : "RISC-V vs NeoVM comparison");
Console.WriteLine("=== Neo RISC-V VM — Full Block Validator (with transaction replay) ===");
Console.WriteLine($"RPC: {rpcUrl}");
Console.WriteLine($"Range: block {startBlock} to {(endBlock == uint.MaxValue ? "∞" : endBlock.ToString())}");
Console.WriteLine($"Checkpoint: {checkpointFile}");
Console.WriteLine($"Mode: {modeLabel}");
Console.WriteLine($"Batch size: {batchSize}");
if (stateDir != null)
{
    Console.WriteLine($"State dir: {stateDir} (persistent)");
    Console.WriteLine($"Save interval: every {saveInterval} blocks");
    Directory.CreateDirectory(stateDir);
}
Console.WriteLine($"Fingerprint interval: every {fpInterval} blocks");
Console.WriteLine();

// Test RPC connectivity
using var httpHandler = new HttpClientHandler { UseProxy = false };
using var httpClient = new HttpClient(httpHandler) { Timeout = TimeSpan.FromSeconds(120) };
try
{
    var testResult = await RpcCall(httpClient, rpcUrl, "getblockcount", "[]");
    var blockCount = testResult.RootElement.GetProperty("result").GetInt64();
    Console.WriteLine($"RPC connected: {rpcUrl} (height={blockCount:N0})");
    if (endBlock == uint.MaxValue)
        endBlock = (uint)(blockCount - 1);
}
catch (Exception ex)
{
    Console.Error.WriteLine($"ERROR: RPC {rpcUrl} failed: {ex.Message}");
    return 1;
}

// State file paths
string? primaryStateFile = stateDir != null
    ? Path.Combine(stateDir, neoVmMode ? "neovm-state.bin" : "riscv-state.bin")
    : null;
string? secondaryStateFile = stateDir != null && !riscvOnly
    ? Path.Combine(stateDir, "neovm-state.bin")
    : null;

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

void UsePrimaryProvider()
{
    if (neoVmMode)
        UseNeoVmProvider();
    else
        UseRiscVProvider();
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
var primarySystem = new NeoSystem(mainnetSettings, primaryProvider);
var primarySnapshot = primarySystem.GetSnapshotCache();
Console.WriteLine($"OK{(primaryProvider.WasRestored ? $" (restored {primaryProvider.RestoredEntryCount:N0} entries)" : "")}");

// Initialize secondary system (comparison mode)
NeoSystem? secondarySystem = null;
DataCache? secondarySnapshot = null;
if (!riscvOnly)
{
    Console.Write("Initializing NeoVM system... ");
    UseNeoVmProvider();
    secondarySystem = new NeoSystem(mainnetSettings, secondaryProvider!);
    secondarySnapshot = secondarySystem.GetSnapshotCache();
    Console.WriteLine($"OK{(secondaryProvider!.WasRestored ? $" (restored {secondaryProvider.RestoredEntryCount:N0} entries)" : "")}");
}

bool canSkipFastForward = primaryProvider.WasRestored
    && (riscvOnly || secondaryProvider?.WasRestored == true);

// If state was restored from an earlier block than the checkpoint, adjust startBlock
// to fast-forward from the state's block height instead of skipping.
uint stateBlock = primaryProvider.RestoredBlockHeight;
if (canSkipFastForward && stateBlock > 0 && stateBlock + 1 < startBlock)
{
    Console.WriteLine($"State restored from block {stateBlock}, checkpoint at block {startBlock - 1}. Fast-forwarding gap...");
    canSkipFastForward = false;
    // Fast-forward only the gap (state block+1 to startBlock-1), not from genesis
    uint ffFrom = stateBlock + 1;
    var ffSw = Stopwatch.StartNew();
    for (uint i = ffFrom; i < startBlock; i += (uint)batchSize)
    {
        uint batchEnd = Math.Min(i + (uint)batchSize, startBlock);
        Block[] ffBlocks;
        try
        {
            ffBlocks = await FetchBlockBatch(httpClient, rpcUrl, i, batchEnd);
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"[rpc-error] gap-forward batch {i}-{batchEnd}: {ex.Message}");
            Console.Error.WriteLine("Retrying in 5s...");
            await Task.Delay(5000);
            i -= (uint)batchSize; // retry
            continue;
        }
        foreach (var ffBlock in ffBlocks)
        {
            var primaryClone = primarySnapshot.CloneCache();
            UsePrimaryProvider();
            PersistBlockWithTransactions(primarySystem, primaryClone, ffBlock, onPersistScript, postPersistScript);
            primaryClone.Commit();
            if (!riscvOnly && secondarySystem != null && secondarySnapshot != null)
            {
                var secondaryClone = secondarySnapshot.CloneCache();
                UseNeoVmProvider();
                PersistBlockWithTransactions(secondarySystem, secondaryClone, ffBlock, onPersistScript, postPersistScript);
                secondaryClone.Commit();
            }
        }
        if (batchEnd % 1000 < (uint)batchSize)
            Console.WriteLine($"  gap-forward: {batchEnd}/{startBlock} ({ffSw.Elapsed.TotalSeconds:F0}s)");
        // Periodic flush to prevent unbounded DataCache accumulation
        if (batchEnd % (uint)saveInterval < (uint)batchSize && stateDir != null)
        {
            primarySnapshot.Commit();
            (primarySnapshot as IDisposable)?.Dispose();
            secondarySnapshot?.Commit();
            (secondarySnapshot as IDisposable)?.Dispose();
            primaryProvider.SaveState(batchEnd);
            secondaryProvider?.SaveState(batchEnd);
            primarySnapshot = primarySystem.GetSnapshotCache();
            secondarySnapshot = secondarySystem?.GetSnapshotCache();
            Console.Error.WriteLine($"[gap-forward-save] block {batchEnd} state flushed");
        }
    }
    ffSw.Stop();
    // Final flush before entering main loop
    primarySnapshot.Commit();
    (primarySnapshot as IDisposable)?.Dispose();
    secondarySnapshot?.Commit();
    (secondarySnapshot as IDisposable)?.Dispose();
    primaryProvider.SaveState(startBlock - 1);
    secondaryProvider?.SaveState(startBlock - 1);
    primarySnapshot = primarySystem.GetSnapshotCache();
    secondarySnapshot = secondarySystem?.GetSnapshotCache();
    Console.WriteLine($"Gap-forwarded {startBlock - 1 - stateBlock} blocks in {ffSw.Elapsed.TotalSeconds:F1}s");
    canSkipFastForward = true; // Mark as done
}

// ─── Fast-forward to start block ────────────────────────────────────────────

if (startBlock > 1 && canSkipFastForward)
{
    Console.WriteLine($"State restored from disk — skipping fast-forward to block {startBlock}.");
}
else if (startBlock > 1)
{
    Console.WriteLine($"Fast-forwarding to block {startBlock} using real blocks from RPC...");
    var ffSw = Stopwatch.StartNew();

    for (uint i = 1; i < startBlock; i += (uint)batchSize)
    {
        uint batchEnd = Math.Min(i + (uint)batchSize, startBlock);
        var blocks = await FetchBlockBatch(httpClient, rpcUrl, i, batchEnd);

        foreach (var block in blocks)
        {
            var primaryClone = primarySnapshot.CloneCache();
            UsePrimaryProvider();
            PersistBlockWithTransactions(primarySystem, primaryClone, block, onPersistScript, postPersistScript);
            primaryClone.Commit();

            if (!riscvOnly && secondarySystem != null && secondarySnapshot != null)
            {
                var secondaryClone = secondarySnapshot.CloneCache();
                UseNeoVmProvider();
                PersistBlockWithTransactions(secondarySystem, secondaryClone, block, onPersistScript, postPersistScript);
                secondaryClone.Commit();
            }
        }

        if (batchEnd % 10000 < (uint)batchSize)
        {
            Console.WriteLine($"  fast-forward: {batchEnd}/{startBlock} ({ffSw.Elapsed.TotalSeconds:F0}s)");
            if (stateDir != null)
            {
                primarySnapshot.Commit();
                (primarySnapshot as IDisposable)?.Dispose();
                secondarySnapshot?.Commit();
                (secondarySnapshot as IDisposable)?.Dispose();
                primaryProvider.SaveState(batchEnd);
                secondaryProvider?.SaveState(batchEnd);
                primarySnapshot = primarySystem.GetSnapshotCache();
                secondarySnapshot = secondarySystem?.GetSnapshotCache();
            }
        }
    }

    ffSw.Stop();
    if (stateDir != null)
    {
        primaryProvider.SaveState(startBlock - 1);
        secondaryProvider?.SaveState(startBlock - 1);
    }
    Console.WriteLine($"Fast-forwarded to block {startBlock} in {ffSw.Elapsed.TotalSeconds:F1}s");
}

// ─── Main validation loop ───────────────────────────────────────────────────

Console.WriteLine();
Console.WriteLine("Block\tTxs\tFingerprint\t\t\t\t\t\t\tEntries\tMatch\tBlocks/s");
Console.WriteLine(new string('=', 130));

var sw = Stopwatch.StartNew();
var batchSw = Stopwatch.StartNew();
long totalMismatches = 0;
long totalDivergences = 0;
long totalBlocks = 0;
long totalTxs = 0;
long totalFaults = 0;

using var checkpointWriter = new StreamWriter(checkpointFile, append: startBlock > 1);

for (uint batchStart = startBlock; batchStart <= endBlock; batchStart += (uint)batchSize)
{
    uint batchEnd = Math.Min(batchStart + (uint)batchSize, endBlock + 1);

    // Fetch batch of blocks from RPC
    Block[] blocks;
    try
    {
        blocks = await FetchBlockBatch(httpClient, rpcUrl, batchStart, batchEnd);
    }
    catch (Exception ex)
    {
        Console.Error.WriteLine($"[rpc-error] batch {batchStart}-{batchEnd}: {ex.Message}");
        Console.Error.WriteLine("Retrying in 5s...");
        await Task.Delay(5000);
        batchStart -= (uint)batchSize; // retry
        continue;
    }

    foreach (var block in blocks)
    {
        uint blockIndex = block.Index;
        int txCount = block.Transactions.Length;

        if (skipBlocks.Contains(blockIndex))
        {
            Console.WriteLine($"[skip-block] block {blockIndex} — skipped via --skip-block; state not applied");
            totalBlocks++;
            continue;
        }

        // Heartbeat so we can tell which block is active if the engine hangs.
        try { File.WriteAllText("fullblock-current-block.txt", $"{blockIndex} txs={txCount} phase=start {DateTime.UtcNow:O}\n"); } catch { }

        try
        {
            // Clone snapshot before each block so failures can be discarded
            // without corrupting the accumulated state.
            var primaryClone = primarySnapshot.CloneCache();

            // Persist through primary engine (RISC-V or NeoVM)
            UsePrimaryProvider();
            var (primaryFaults, primaryExceptions) = PersistBlockWithTransactions(
                primarySystem, primaryClone, block, onPersistScript, postPersistScript);
            primaryClone.Commit(); // merge into parent snapshot

            // Persist through secondary engine (NeoVM, comparison mode)
            int secondaryFaults = 0;
            List<(UInt256, string)> secondaryExceptions = new();
            if (!riscvOnly && secondarySystem != null && secondarySnapshot != null)
            {
                var secondaryClone = secondarySnapshot.CloneCache();
                UseNeoVmProvider();
                (secondaryFaults, secondaryExceptions) = PersistBlockWithTransactions(
                    secondarySystem, secondaryClone, block, onPersistScript, postPersistScript);
                secondaryClone.Commit(); // merge into parent snapshot
            }

            totalBlocks++;
            totalTxs += txCount;
            totalFaults += primaryFaults;

        // Check for fault count divergence
        if (!riscvOnly && primaryFaults != secondaryFaults)
        {
            totalDivergences++;
            Console.Error.WriteLine($"!!! FAULT DIVERGENCE #{totalDivergences} at block {blockIndex}: RISC-V faults={primaryFaults}, NeoVM faults={secondaryFaults}");
            foreach (var (hash, msg) in primaryExceptions)
                Console.Error.WriteLine($"  RISC-V fault: {hash}: {msg}");
            foreach (var (hash, msg) in secondaryExceptions)
                Console.Error.WriteLine($"  NeoVM fault: {hash}: {msg}");
        }

        // Log transactions with faults
        if (primaryExceptions.Count > 0)
        {
            foreach (var (txHash, ex) in primaryExceptions)
            {
                Console.Error.WriteLine($"[tx-fault] block {blockIndex} tx {txHash}: {ex}");
            }
        }

        // Fingerprint and reporting
        bool isFingerprintPoint = blockIndex % (uint)fpInterval == 0;
        bool isSavePoint = stateDir != null && blockIndex % (uint)saveInterval == 0;
        bool isReport = blockIndex % (uint)reportInterval == 0 || blockIndex == startBlock;

        if (isFingerprintPoint || isReport)
        {
            string primaryFp = ComputeStateFingerprint(primarySnapshot);
            int entries = CountEntries(primarySnapshot);
            string matchStatus = "---";

            if (!riscvOnly && secondarySnapshot != null)
            {
                string secondaryFp = ComputeStateFingerprint(secondarySnapshot);
                if (primaryFp == secondaryFp)
                {
                    matchStatus = "OK";
                }
                else
                {
                    matchStatus = "MISMATCH";
                    totalMismatches++;
                    Console.Error.WriteLine($"!!! MISMATCH at block {blockIndex} — resyncing RISC-V state from NeoVM to continue validation");
                    Console.Error.WriteLine($"  RISC-V: {primaryFp}");
                    Console.Error.WriteLine($"  NeoVM:  {secondaryFp}");
                    // Resync: copy NeoVM store data into RISC-V store (same provider, no new system)
                    if (primaryProvider.Store is MemoryStore primaryStore && secondaryProvider?.Store is MemoryStore secondaryStore)
                    {
                        foreach (var (key, _) in primaryStore.Find(null, SeekDirection.Forward).ToList())
                            primaryStore.Delete(key);
                        foreach (var (key, value) in secondaryStore.Find(null, SeekDirection.Forward))
                            primaryStore.Put(key, value);
                        // Refresh snapshot from the updated store
                        primarySnapshot = primarySystem.GetSnapshotCache();
                        Console.Error.WriteLine($"  Resynced RISC-V store from NeoVM at block {blockIndex}");
                    }
                    primaryFp = secondaryFp;
                    entries = CountEntries(primarySnapshot);
                }
            }

            if (isReport)
            {
                double bps = reportInterval / batchSw.Elapsed.TotalSeconds;
                batchSw.Restart();
                Console.WriteLine($"{blockIndex}\t{txCount}\t{primaryFp}\t{entries}\t{matchStatus}\t{bps:F1}");
            }

            if (isFingerprintPoint)
            {
                checkpointWriter.WriteLine($"{blockIndex}\t{primaryFp}\t{entries}");
                checkpointWriter.Flush();
                Console.Error.WriteLine($"[checkpoint] block {blockIndex} entries={entries} txs_total={totalTxs} faults_total={totalFaults}");
            }
        }

        if (isSavePoint)
        {
            var saveSw = Stopwatch.StartNew();
            // Flush accumulated DataCache changes to the store, then recreate
            // fresh snapshots. This prevents unbounded state accumulation in the
            // DataCache dictionary which causes OnPersist failures after ~150K blocks.
            primarySnapshot.Commit();
            (primarySnapshot as IDisposable)?.Dispose();
            secondarySnapshot?.Commit();
            (secondarySnapshot as IDisposable)?.Dispose();

            primaryProvider.SaveState(blockIndex);
            secondaryProvider?.SaveState(blockIndex);

            // Fresh snapshots from the updated store
            primarySnapshot = primarySystem.GetSnapshotCache();
            secondarySnapshot = secondarySystem?.GetSnapshotCache();
            saveSw.Stop();
            Console.Error.WriteLine($"[state-save] block {blockIndex} saved+refreshed in {saveSw.Elapsed.TotalSeconds:F1}s");
        }
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"[block-fatal] block {blockIndex} CRASHED: {ex.Message}");
            Console.Error.WriteLine($"  Stack: {ex.StackTrace}");
            // The clone was discarded (never committed), so primarySnapshot
            // still has clean state up to the previous block. A NeoVM replay here
            // is diagnostic only; committing it into the primary path would hide
            // the RISC-V divergence this validator is meant to catch.
            int? retryFaults = null;
            Exception? retryFailure = null;
            try
            {
                Console.Error.WriteLine($"[block-fatal] Running NeoVM diagnostic replay for block {blockIndex}...");

                var retryClone = primarySnapshot.CloneCache();
                UseNeoVmProvider();
                var (faults, _) = PersistBlockWithTransactions(
                    primarySystem, retryClone, block, onPersistScript, postPersistScript);
                retryFaults = faults;

                if (!riscvOnly && secondarySystem != null && secondarySnapshot != null)
                {
                    var secondaryRetryClone = secondarySnapshot.CloneCache();
                    UseNeoVmProvider();
                    PersistBlockWithTransactions(
                        secondarySystem, secondaryRetryClone, block, onPersistScript, postPersistScript);
                }
            }
            catch (Exception retryEx)
            {
                retryFailure = retryEx;
            }

            if (retryFailure is null)
            {
                Console.Error.WriteLine($"[block-fatal] NeoVM diagnostic replay succeeded for block {blockIndex} (faults={retryFaults}); RISC-V state was not advanced");
                if (skipFaultyBlocks)
                {
                    totalBlocks++;
                    UsePrimaryProvider();
                    Console.Error.WriteLine($"[skip-faulty] block {blockIndex} skipped after primary failure; state not applied");
                    continue;
                }

                throw new InvalidOperationException(
                    $"RISC-V validation crashed at block {blockIndex}; NeoVM diagnostic replay succeeded, but primary state was not advanced.",
                    ex);
            }
            else
            {
                Console.Error.WriteLine($"[block-fatal] NeoVM retry also failed at block {blockIndex}: {retryFailure.Message}");
                Console.Error.WriteLine($"  Stack: {retryFailure.StackTrace}");
                if (skipFaultyBlocks)
                {
                    totalBlocks++;
                    UsePrimaryProvider();
                    Console.Error.WriteLine($"[skip-faulty] block {blockIndex} skipped after primary and NeoVM retry failures; state not applied");
                    continue;
                }
                throw new InvalidOperationException(
                    $"RISC-V validation crashed at block {blockIndex}; NeoVM diagnostic replay also failed.",
                    retryFailure);
            }
        }
    }
}

sw.Stop();

Console.WriteLine();
Console.WriteLine($"=== Validation Complete ===");
Console.WriteLine($"Blocks processed: {totalBlocks:N0}");
Console.WriteLine($"Transactions executed: {totalTxs:N0}");
Console.WriteLine($"Transaction faults: {totalFaults:N0}");
Console.WriteLine($"State mismatches: {totalMismatches}");
Console.WriteLine($"Total time: {sw.Elapsed}");
Console.WriteLine($"Average speed: {totalBlocks / sw.Elapsed.TotalSeconds:F0} blocks/sec");

if (totalMismatches > 0)
{
    Console.Error.WriteLine($"FAIL: {totalMismatches} state root mismatches detected!");
    return 1;
}

Console.WriteLine("PASS: All block state roots match.");
return 0;

// ─── Block Persistence (exact Blockchain.Persist flow) ─────────────────────

static void WriteHeartbeat(uint blockIndex, string phase)
{
    try { File.WriteAllText("fullblock-current-block.txt", $"{blockIndex} phase={phase} {DateTime.UtcNow:O}\n"); } catch { }
}

static (int faults, List<(UInt256, string)> exceptions) PersistBlockWithTransactions(
    NeoSystem system, DataCache snapshot, Block block,
    byte[] onPersistScript, byte[] postPersistScript)
{
    int faults = 0;
    var exceptions = new List<(UInt256, string)>();

    // Phase 1: OnPersist — registers block and transactions in ledger
    WriteHeartbeat(block.Index, "onpersist");
    TransactionState[]? transactionStates = null;
    using (var engine = ApplicationEngine.Create(
        TriggerType.OnPersist, null, snapshot, block, system.Settings, 0))
    {
        engine.LoadScript(onPersistScript);
        var state = engine.Execute();
        if (state != VMState.HALT)
            throw new Exception($"OnPersist FAULT at block {block.Index}: {engine.FaultException?.Message}");
        transactionStates = engine.GetState<TransactionState[]>();
    }

    // Phase 2: Execute each transaction (Application trigger)
    if (transactionStates != null && transactionStates.Length > 0)
    {
        var clonedSnapshot = snapshot.CloneCache();
        foreach (var transactionState in transactionStates)
        {
            var tx = transactionState.Transaction!;
            WriteHeartbeat(block.Index, $"tx={tx.Hash}");
            using var engine = ApplicationEngine.Create(
                TriggerType.Application, tx, clonedSnapshot, block, system.Settings, tx.SystemFee);
            engine.LoadScript(tx.Script);
            transactionState.State = engine.Execute();

            // RISC-V Trap fallback is handled internally by the RiscvApplicationEngine.
            // Traps on transactions are recorded as FAULTs.

            if (transactionState.State == VMState.HALT)
            {
                clonedSnapshot.Commit();
            }
            else
            {
                faults++;
                if (engine.FaultException != null)
                {
                    exceptions.Add((tx.Hash, engine.FaultException.Message));
                    Console.Error.WriteLine($"[tx-fault-trace] block {block.Index} tx {tx.Hash}:");
                    Console.Error.WriteLine($"  Exception: {engine.FaultException.GetType().FullName}: {engine.FaultException.Message}");
                    Console.Error.WriteLine($"  Stack: {engine.FaultException.StackTrace}");
                    if (engine.FaultException.InnerException != null)
                        Console.Error.WriteLine($"  Inner: {engine.FaultException.InnerException.GetType().FullName}: {engine.FaultException.InnerException.Message}\n  InnerStack: {engine.FaultException.InnerException.StackTrace}");
                }
                clonedSnapshot = snapshot.CloneCache();
            }
        }
    }

    // Phase 3: PostPersist — updates current block pointer
    WriteHeartbeat(block.Index, "postpersist");
    using (var engine = ApplicationEngine.Create(
        TriggerType.PostPersist, null, snapshot, block, system.Settings, 0))
    {
        engine.LoadScript(postPersistScript);
        var state = engine.Execute();
        if (state != VMState.HALT)
            throw new Exception($"PostPersist FAULT at block {block.Index}: {engine.FaultException?.Message}");
    }

    // Phase 4: Final commit to store
    snapshot.Commit();

    return (faults, exceptions);
}

// ─── RPC Block Fetching ───────────────────────────────────────────────────

static async Task<Block[]> FetchBlockBatch(HttpClient client, string rpcUrl, uint start, uint end)
{
    // Parallel fetch with concurrency limit to avoid overwhelming the RPC node
    const int maxConcurrency = 8;
    var semaphore = new SemaphoreSlim(maxConcurrency);
    var tasks = new List<(uint index, Task<Block> task)>();

    for (uint i = start; i < end; i++)
    {
        var idx = i;
        tasks.Add((idx, FetchWithSemaphore(client, rpcUrl, idx, semaphore)));
    }

    await Task.WhenAll(tasks.Select(t => t.task));
    return tasks.OrderBy(t => t.index).Select(t => t.task.Result).ToArray();
}

static async Task<Block> FetchWithSemaphore(HttpClient client, string rpcUrl, uint index, SemaphoreSlim semaphore)
{
    await semaphore.WaitAsync();
    try { return await FetchSingleBlock(client, rpcUrl, index); }
    finally { semaphore.Release(); }
}

static async Task<Block> FetchSingleBlock(HttpClient client, string rpcUrl, uint index)
{
    for (int attempt = 0; attempt < 5; attempt++)
    {
        try
        {
            var body = $"{{\"jsonrpc\":\"2.0\",\"method\":\"getblock\",\"params\":[{index},false],\"id\":1}}";
            var resp = await client.PostAsync(rpcUrl, new StringContent(body, Encoding.UTF8, "application/json"));
            resp.EnsureSuccessStatusCode();
            var json = JsonDocument.Parse(await resp.Content.ReadAsStringAsync());
            if (json.RootElement.TryGetProperty("error", out var err))
                throw new Exception($"RPC error: {err.GetProperty("message").GetString()}");
            var base64 = json.RootElement.GetProperty("result").GetString()!;
            return Convert.FromBase64String(base64).AsSerializable<Block>();
        }
        catch (Exception ex)
        {
            if (attempt == 4) throw new Exception($"RPC failed for block {index}: {ex.Message}", ex);
            await Task.Delay(1000 * (attempt + 1));
        }
    }
    throw new Exception($"RPC failed for block {index}");
}

static async Task<JsonDocument> RpcCall(HttpClient client, string rpcUrl, string method, string paramsJson)
{
    var body = $"{{\"jsonrpc\":\"2.0\",\"method\":\"{method}\",\"params\":{paramsJson},\"id\":1}}";
    var content = new StringContent(body, Encoding.UTF8, "application/json");
    var response = await client.PostAsync(rpcUrl, content);
    response.EnsureSuccessStatusCode();
    var json = await response.Content.ReadAsStringAsync();
    return JsonDocument.Parse(json);
}

// ─── Fingerprinting ────────────────────────────────────────────────────────

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

class ResumableStoreProvider : IStoreProvider
{
    private static readonly byte[] MagicV1 = "NEOSTATE"u8.ToArray();
    private static readonly byte[] MagicV2 = "NEOSTV02"u8.ToArray();
    private readonly string? _stateFile;
    private IStore? _store;

    public string Name => nameof(MemoryStore);
    public IStore? Store => _store;
    public bool WasRestored { get; private set; }
    public int RestoredEntryCount { get; private set; }
    public uint RestoredBlockHeight { get; private set; }

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

    public void SaveState(uint blockHeight = 0)
    {
        if (_store == null || _stateFile == null) return;

        var tmpFile = _stateFile + ".tmp";
        using (var fs = new FileStream(tmpFile, FileMode.Create, FileAccess.Write, FileShare.None, 1 << 20))
        using (var bw = new BinaryWriter(fs))
        {
            bw.Write(MagicV2);
            bw.Write(blockHeight);
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
        File.Move(tmpFile, _stateFile, overwrite: true);
    }

    private int RestoreState(MemoryStore store)
    {
        using var fs = new FileStream(_stateFile!, FileMode.Open, FileAccess.Read, FileShare.Read, 1 << 20);
        using var br = new BinaryReader(fs);

        var magic = br.ReadBytes(8);
        if (magic.AsSpan().SequenceEqual(MagicV2))
        {
            RestoredBlockHeight = br.ReadUInt32();
        }
        else if (!magic.AsSpan().SequenceEqual(MagicV1))
        {
            throw new InvalidDataException($"Invalid state file magic in {_stateFile}");
        }

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
