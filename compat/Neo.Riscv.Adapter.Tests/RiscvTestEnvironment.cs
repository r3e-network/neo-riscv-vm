#nullable enable

using Microsoft.VisualStudio.TestTools.UnitTesting;
using Neo.SmartContract;
using Neo.SmartContract.RiscV;
using System;
using System.IO;

namespace Neo.Riscv.Adapter.Tests;

[TestClass]
public static class RiscvTestEnvironment
{
    private static string? _previousLibraryPath;
    private static IApplicationEngineProvider? _previousProvider;

    [AssemblyInitialize]
    public static void Initialize(TestContext _)
    {
        _previousLibraryPath = Environment.GetEnvironmentVariable(NativeRiscvVmBridge.LibraryPathEnvironmentVariable);
        _previousProvider = ApplicationEngine.Provider;
        ApplicationEngine.Provider ??= new NeoVMHostApplicationEngineProvider();
        var libraryPath = ResolveWorkspaceLibraryPath();
        if (libraryPath is not null)
            Environment.SetEnvironmentVariable(NativeRiscvVmBridge.LibraryPathEnvironmentVariable, libraryPath);
    }

    [AssemblyCleanup]
    public static void Cleanup()
    {
        ApplicationEngine.Provider = _previousProvider;
        Environment.SetEnvironmentVariable(NativeRiscvVmBridge.LibraryPathEnvironmentVariable, _previousLibraryPath);
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
}
