using Microsoft.VisualStudio.TestTools.UnitTesting;
using Neo.Persistence.Providers;
using Neo.SmartContract;
using Neo.SmartContract.Native;
using Neo.SmartContract.RiscV;
using System;
using System.IO;
using System.Runtime.InteropServices;

namespace Neo.Riscv.Adapter.Tests;

[TestClass]
public class UT_RiscvApplicationEngineProvider
{
    private string? _previousLibraryPath;

    [TestInitialize]
    public void TestSetup()
    {
        _previousLibraryPath = Environment.GetEnvironmentVariable(NativeRiscvVmBridge.LibraryPathEnvironmentVariable);
        var libraryPath = ResolveWorkspaceLibraryPath();
        if (!string.IsNullOrWhiteSpace(libraryPath))
            Environment.SetEnvironmentVariable(NativeRiscvVmBridge.LibraryPathEnvironmentVariable, libraryPath);
        RiscvApplicationEngineProviderResolver.ResetForTesting();
        if (!string.IsNullOrWhiteSpace(libraryPath))
            ApplicationEngine.Provider = new RiscvApplicationEngineProvider(new NativeRiscvVmBridge(libraryPath));
    }

    [TestCleanup]
    public void TestCleanup()
    {
        Environment.SetEnvironmentVariable(NativeRiscvVmBridge.LibraryPathEnvironmentVariable, _previousLibraryPath);
        RiscvApplicationEngineProviderResolver.ResetForTesting();
        if (!string.IsNullOrWhiteSpace(_previousLibraryPath) && File.Exists(_previousLibraryPath))
            ApplicationEngine.Provider = new RiscvApplicationEngineProvider(new NativeRiscvVmBridge(_previousLibraryPath));
        else
        {
            var workspaceLibrary = ResolveWorkspaceLibraryPath();
            if (!string.IsNullOrWhiteSpace(workspaceLibrary))
            {
                Environment.SetEnvironmentVariable(NativeRiscvVmBridge.LibraryPathEnvironmentVariable, workspaceLibrary);
                ApplicationEngine.Provider = new RiscvApplicationEngineProvider(new NativeRiscvVmBridge(workspaceLibrary));
            }
        }
    }

    [TestMethod]
    public void TestConfiguredRiscvProviderBecomesDefault()
    {
        var libraryPath = Environment.GetEnvironmentVariable(NativeRiscvVmBridge.LibraryPathEnvironmentVariable);
        if (string.IsNullOrWhiteSpace(libraryPath) || !File.Exists(libraryPath))
            libraryPath = ResolveWorkspaceLibraryPath();
        if (string.IsNullOrWhiteSpace(libraryPath))
            Assert.Inconclusive($"{NativeRiscvVmBridge.LibraryPathEnvironmentVariable} is not set to a valid library.");
        Environment.SetEnvironmentVariable(NativeRiscvVmBridge.LibraryPathEnvironmentVariable, libraryPath);

        ApplicationEngine.Provider = RiscvApplicationEngineProviderResolver.ResolveRequiredProvider();
        using var system = new NeoSystem(AdapterTestProtocolSettings.Default, new MemoryStoreProvider());
        using var snapshot = system.GetSnapshotCache();
        using var appEngine = ApplicationEngine.Create(
            TriggerType.Application,
            null,
            snapshot,
            gas: 0,
            settings: AdapterTestProtocolSettings.Default);

        Assert.AreEqual(typeof(RiscvApplicationEngine).FullName, appEngine.GetType().FullName);
    }

    [TestMethod]
    public void TestConfiguredRiscvProviderBootstrapsNativeState()
    {
        var libraryPath = Environment.GetEnvironmentVariable(NativeRiscvVmBridge.LibraryPathEnvironmentVariable);
        if (string.IsNullOrWhiteSpace(libraryPath) || !File.Exists(libraryPath))
            libraryPath = ResolveWorkspaceLibraryPath();
        if (string.IsNullOrWhiteSpace(libraryPath))
            Assert.Inconclusive($"{NativeRiscvVmBridge.LibraryPathEnvironmentVariable} is not set to a valid library.");
        Environment.SetEnvironmentVariable(NativeRiscvVmBridge.LibraryPathEnvironmentVariable, libraryPath);

        ApplicationEngine.Provider = RiscvApplicationEngineProviderResolver.ResolveRequiredProvider();
        using var system = new NeoSystem(AdapterTestProtocolSettings.Default, new MemoryStoreProvider());
        using var snapshot = system.GetSnapshotCache();

        Assert.IsNotNull(NativeContract.ContractManagement.GetContract(snapshot, NativeContract.NEO.Hash));
        Assert.IsNotNull(NativeContract.ContractManagement.GetContract(snapshot, NativeContract.GAS.Hash));
        Assert.AreEqual(system.GenesisBlock.Hash, NativeContract.Ledger.CurrentHash(snapshot));
    }

    [TestMethod]
    public void TestExplicitRiscvProviderBootstrapsNativeState()
    {
        var libraryPath = Environment.GetEnvironmentVariable(NativeRiscvVmBridge.LibraryPathEnvironmentVariable);
        if (string.IsNullOrWhiteSpace(libraryPath) || !File.Exists(libraryPath))
            libraryPath = ResolveWorkspaceLibraryPath();
        if (string.IsNullOrWhiteSpace(libraryPath))
            Assert.Inconclusive($"{NativeRiscvVmBridge.LibraryPathEnvironmentVariable} is not set to a valid library.");
        Environment.SetEnvironmentVariable(NativeRiscvVmBridge.LibraryPathEnvironmentVariable, libraryPath);

        ApplicationEngine.Provider = new RiscvApplicationEngineProvider(new NativeRiscvVmBridge(libraryPath));
        using var system = new NeoSystem(AdapterTestProtocolSettings.Default, new MemoryStoreProvider());
        using var snapshot = system.GetSnapshotCache();

        Assert.IsNotNull(NativeContract.ContractManagement.GetContract(snapshot, NativeContract.NEO.Hash));
        Assert.IsNotNull(NativeContract.ContractManagement.GetContract(snapshot, NativeContract.GAS.Hash));
        Assert.AreEqual(system.GenesisBlock.Hash, NativeContract.Ledger.CurrentHash(snapshot));
    }

    [TestMethod]
    public void TestConfiguredRiscvProviderIgnoresBrokenBundledLibraryWhenEnvOverrideIsValid()
    {
        var libraryPath = Environment.GetEnvironmentVariable(NativeRiscvVmBridge.LibraryPathEnvironmentVariable);
        if (string.IsNullOrWhiteSpace(libraryPath) || !File.Exists(libraryPath))
            libraryPath = ResolveWorkspaceLibraryPath();
        if (string.IsNullOrWhiteSpace(libraryPath) || !File.Exists(libraryPath))
            Assert.Inconclusive($"{NativeRiscvVmBridge.LibraryPathEnvironmentVariable} is not set to a valid library.");
        if (!CanLoadLibrary(libraryPath))
            Assert.Inconclusive($"Resolved host library is not loadable in this test environment: {libraryPath}");

        Environment.SetEnvironmentVariable(NativeRiscvVmBridge.LibraryPathEnvironmentVariable, libraryPath);

        var pluginRoot = Path.Combine(AppContext.BaseDirectory, "Plugins", "Neo.Riscv.Adapter");
        Directory.CreateDirectory(pluginRoot);
        var bundledPath = Path.Combine(pluginRoot, "libneo_riscv_host.so");
        var hadBundled = File.Exists(bundledPath);
        byte[]? originalBytes = hadBundled ? File.ReadAllBytes(bundledPath) : null;

        try
        {
            File.WriteAllText(bundledPath, "broken");
            ApplicationEngine.Provider = RiscvApplicationEngineProviderResolver.ResolveRequiredProvider();

            using var system = new NeoSystem(AdapterTestProtocolSettings.Default, new MemoryStoreProvider());
            using var snapshot = system.GetSnapshotCache();
            Assert.IsNotNull(NativeContract.ContractManagement.GetContract(snapshot, NativeContract.NEO.Hash));
        }
        finally
        {
            if (hadBundled && originalBytes is not null)
                File.WriteAllBytes(bundledPath, originalBytes);
            else if (File.Exists(bundledPath))
                File.Delete(bundledPath);
        }
    }

    private static string? ResolveWorkspaceLibraryPath()
    {
        var baseDirectory = AppContext.BaseDirectory;
        var release = Path.GetFullPath(Path.Combine(baseDirectory, "..", "..", "..", "..", "..", "target", "release", "libneo_riscv_host.so"));
        if (File.Exists(release))
            return release;

        var debug = Path.GetFullPath(Path.Combine(baseDirectory, "..", "..", "..", "..", "..", "target", "debug", "libneo_riscv_host.so"));
        if (File.Exists(debug))
            return debug;

        return null;
    }

    private static bool CanLoadLibrary(string path)
    {
        try
        {
            var handle = NativeLibrary.Load(path);
            NativeLibrary.Free(handle);
            return true;
        }
        catch
        {
            return false;
        }
    }
}
