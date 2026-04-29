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

    internal static void RequireNativeRiscvProvider()
    {
        var libraryPath = ResolveNativeHostLibraryPath();
        if (string.IsNullOrWhiteSpace(libraryPath))
            Assert.Inconclusive($"{NativeRiscvVmBridge.LibraryPathEnvironmentVariable} is not set to a valid library.");

        Environment.SetEnvironmentVariable(NativeRiscvVmBridge.LibraryPathEnvironmentVariable, libraryPath);
        RiscvApplicationEngineProviderResolver.ResetForTesting();
        try
        {
            ApplicationEngine.Provider = RiscvApplicationEngineProviderResolver.ResolveRequiredProvider();
        }
        catch (InvalidOperationException ex)
        {
            Assert.Inconclusive($"Resolved RISC-V host library is not loadable in this test environment: {ex.Message}");
        }
    }

    internal static void RestoreManagedHostProvider()
    {
        RiscvApplicationEngineProviderResolver.ResetForTesting();
        ApplicationEngine.Provider = new NeoVMHostApplicationEngineProvider();
    }

    private static string? ResolveNativeHostLibraryPath()
    {
        var configured = Environment.GetEnvironmentVariable(NativeRiscvVmBridge.LibraryPathEnvironmentVariable);
        if (!string.IsNullOrWhiteSpace(configured) && File.Exists(configured))
            return configured;

        return ResolveWorkspaceLibraryPath();
    }

    private static string? ResolveWorkspaceLibraryPath()
    {
        var baseDirectory = AppContext.BaseDirectory;
        var fileName = GetPlatformFileName();
        var release = Path.GetFullPath(Path.Combine(baseDirectory, "..", "..", "..", "..", "..", "target", "release", fileName));
        if (File.Exists(release))
            return release;

        var debug = Path.GetFullPath(Path.Combine(baseDirectory, "..", "..", "..", "..", "..", "target", "debug", fileName));
        if (File.Exists(debug))
            return debug;

        return null;
    }

    private static string GetPlatformFileName()
    {
        if (OperatingSystem.IsWindows())
            return "neo_riscv_host.dll";
        if (OperatingSystem.IsMacOS())
            return "libneo_riscv_host.dylib";
        return "libneo_riscv_host.so";
    }
}
