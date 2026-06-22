using System;

namespace Neo.SmartContract.RiscV
{
    /// <summary>
    /// Selects which execution path a contract takes inside the RISC-V bridge.
    /// </summary>
    /// <remarks>
    /// The neovm→riscvvm replacement supports two execution modes, chosen per
    /// contract at call time by <see cref="RiscvExecutionDispatcher"/>:
    /// <list type="bullet">
    /// <item><term><see cref="NeoVmCompatibilityContract"/></term><description>
    /// Legacy NeoVM contracts (whose script is NeoVM bytecode, not a PolkaVM
    /// binary). These run through the cached <c>neo-vm-rs</c> interpreter
    /// inside the guest, with syscalls forwarded to the .NET host — guaranteeing
    /// byte-for-byte state-root compatibility with upstream NeoVM.</description></item>
    /// <item><term><see cref="NativeRiscvDirect"/></term><description>
    /// True RISC-V contracts (compiled PolkaVM binaries). These execute directly
    /// in the PolkaVM engine without the NeoVM interpreter layer.</description></item>
    /// </list>
    /// </remarks>
    internal enum RiscvExecutionKind : byte
    {
        /// <summary>
        /// Run a legacy NeoVM contract through the in-guest <c>neo-vm-rs</c>
        /// compatibility interpreter (script is NeoVM bytecode).
        /// </summary>
        NeoVmCompatibilityContract = 0,
        /// <summary>
        /// Run a native RISC-V contract directly in PolkaVM (script is a
        /// PolkaVM binary, detected by the <c>"PVM\0"</c> magic header).
        /// </summary>
        NativeRiscvDirect = 1,
    }

    /// <summary>
    /// Resolves which <see cref="RiscvExecutionKind"/> a contract should use,
    /// based on its declared <see cref="ContractType"/> and the actual script
    /// payload bytes.
    /// </summary>
    /// <remarks>
    /// This is the single decision point that routes each contract invocation
    /// to the correct execution backend. The routing logic cross-checks the
    /// manifest-declared <see cref="ContractType"/> against the script's magic
    /// bytes so that a mislabeled contract fails loudly rather than silently
    /// executing under the wrong engine (which would diverge the state root).
    /// </remarks>
    internal static class RiscvExecutionDispatcher
    {
        /// <summary>
        /// Resolve the execution kind for a contract.
        /// </summary>
        /// <param name="contractType">The manifest-declared contract backend.</param>
        /// <param name="script">The contract's raw script/payload bytes.</param>
        /// <returns>The resolved execution kind.</returns>
        /// <exception cref="InvalidOperationException">
        /// Thrown when the declared type and script payload are inconsistent
        /// (e.g. a NeoVM-typed contract carrying a PolkaVM binary, or a
        /// RISC-V-typed contract carrying NeoVM bytecode).
        /// </exception>
        internal static RiscvExecutionKind Resolve(ContractType contractType, ReadOnlyMemory<byte> script)
        {
            var isPvmBinary = IsPvmBinary(script);

            return contractType switch
            {
                ContractType.NeoVM when !isPvmBinary => RiscvExecutionKind.NeoVmCompatibilityContract,
                ContractType.RiscV when isPvmBinary => RiscvExecutionKind.NativeRiscvDirect,
                ContractType.NeoVM => throw new InvalidOperationException("NeoVM compatibility contracts cannot use a PolkaVM binary payload."),
                ContractType.RiscV => throw new InvalidOperationException("RISC-V contract must use a PolkaVM binary payload."),
                _ => throw new InvalidOperationException($"Unsupported contract execution kind: {contractType}."),
            };
        }

        /// <summary>
        /// Detect whether a script payload is a PolkaVM binary by checking for
        /// the <c>"PVM\0"</c> (0x50 0x56 0x4D 0x00) magic header.
        /// </summary>
        /// <remarks>
        /// This is the <b>single canonical</b> PolkaVM magic-byte check for the
        /// adapter project. Other partial classes (<c>NativeRiscvVmBridge.RuntimeInterop</c>,
        /// <c>RiscvApplicationEngine</c>) delegate here instead of duplicating
        /// the byte comparison, so the magic is defined in exactly one place.
        /// </remarks>
        /// <param name="script">The script bytes to inspect.</param>
        /// <returns><see langword="true"/> if the payload starts with the PolkaVM magic.</returns>
        internal static bool IsPvmBinary(ReadOnlyMemory<byte> script)
        {
            var span = script.Span;
            return span.Length >= 4
                && span[0] == 0x50
                && span[1] == 0x56
                && span[2] == 0x4D
                && span[3] == 0x00;
        }
    }
}
