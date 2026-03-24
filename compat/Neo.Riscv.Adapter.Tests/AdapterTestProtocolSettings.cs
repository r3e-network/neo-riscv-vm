using Neo.Cryptography.ECC;

namespace Neo.Riscv.Adapter.Tests;

internal static class AdapterTestProtocolSettings
{
    internal static readonly ProtocolSettings Default = ProtocolSettings.Default with
    {
        Network = 0x334F454Eu,
        StandbyCommittee =
        [
            ECPoint.Parse("0278ed78c917797b637a7ed6e7a9d94e8c408444c41ee4c0a0f310a256b9271eda", ECCurve.Secp256r1)
        ],
        ValidatorsCount = 1,
        SeedList =
        [
            "seed1.neo.org:10333",
            "seed2.neo.org:10333",
            "seed3.neo.org:10333",
            "seed4.neo.org:10333",
            "seed5.neo.org:10333"
        ],
    };
}
