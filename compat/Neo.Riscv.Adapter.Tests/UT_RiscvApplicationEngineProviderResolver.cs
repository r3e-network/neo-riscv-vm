using Microsoft.VisualStudio.TestTools.UnitTesting;
using Neo.SmartContract.RiscV;
using System;
using System.IO;

namespace Neo.Riscv.Adapter.Tests;

[TestClass]
public class UT_RiscvApplicationEngineProviderResolver
{
    [TestMethod]
    public void ResolveLibraryPath_PrefersPluginFolder_WhenEnvVarUnset()
    {
        var previous = Environment.GetEnvironmentVariable(NativeRiscvVmBridge.LibraryPathEnvironmentVariable);
        try
        {
            Environment.SetEnvironmentVariable(NativeRiscvVmBridge.LibraryPathEnvironmentVariable, null);
            RiscvApplicationEngineProviderResolver.ResetForTesting();

            var fileName = GetPlatformFileName();
            var pluginRoot = Path.Combine(AppContext.BaseDirectory, "Plugins", "Neo.Riscv.Adapter");
            Directory.CreateDirectory(pluginRoot);
            var expected = Path.Combine(pluginRoot, fileName);

            File.WriteAllText(expected, string.Empty);
            try
            {
                var actual = RiscvApplicationEngineProviderResolver.ResolveLibraryPathForTesting();
                Assert.AreEqual(expected, actual);
            }
            finally
            {
                File.Delete(expected);
            }
        }
        finally
        {
            Environment.SetEnvironmentVariable(NativeRiscvVmBridge.LibraryPathEnvironmentVariable, previous);
            RiscvApplicationEngineProviderResolver.ResetForTesting();
        }
    }

    [TestMethod]
    public void ResolveLibraryPath_PrefersPluginFolder_WhenEnvVarPointsToAnotherExistingLibrary()
    {
        var previous = Environment.GetEnvironmentVariable(NativeRiscvVmBridge.LibraryPathEnvironmentVariable);
        var pluginRoot = Path.Combine(AppContext.BaseDirectory, "Plugins", "Neo.Riscv.Adapter");
        Directory.CreateDirectory(pluginRoot);
        var fileName = GetPlatformFileName();
        var bundled = Path.Combine(pluginRoot, fileName);
        var externalRoot = Path.Combine(Path.GetTempPath(), Guid.NewGuid().ToString("N"));
        var external = Path.Combine(externalRoot, fileName);

        try
        {
            Directory.CreateDirectory(externalRoot);
            File.WriteAllText(bundled, string.Empty);
            File.WriteAllText(external, "stale");
            Environment.SetEnvironmentVariable(NativeRiscvVmBridge.LibraryPathEnvironmentVariable, external);
            RiscvApplicationEngineProviderResolver.ResetForTesting();

            var actual = RiscvApplicationEngineProviderResolver.ResolveLibraryPathForTesting();
            Assert.AreEqual(bundled, actual);
        }
        finally
        {
            Environment.SetEnvironmentVariable(NativeRiscvVmBridge.LibraryPathEnvironmentVariable, previous);
            RiscvApplicationEngineProviderResolver.ResetForTesting();
            if (File.Exists(bundled))
                File.Delete(bundled);
            if (File.Exists(external))
                File.Delete(external);
            if (Directory.Exists(externalRoot))
                Directory.Delete(externalRoot, recursive: true);
        }
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
