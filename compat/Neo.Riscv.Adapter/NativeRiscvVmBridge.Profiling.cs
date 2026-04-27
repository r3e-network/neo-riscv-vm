using System;
using System.Collections.Concurrent;
using System.Diagnostics;
using System.Linq;
using System.Threading;

namespace Neo.SmartContract.RiscV
{
    public sealed partial class NativeRiscvVmBridge
    {
        private static void Trace(string message)
        {
            if (!TraceEnabled) return;
            Console.Error.WriteLine($"[neo-riscv] {message}");
        }

        private static void RecordHostProfile(uint api, string name, int inputItems, long readStackTicks, long handleTicks)
        {
            var stat = HostProfileStats.GetOrAdd(api, _ => new HostProfileStat { Name = name });
            Interlocked.Increment(ref stat.Count);
            Interlocked.Add(ref stat.InputItems, inputItems);
            Interlocked.Add(ref stat.ReadStackTicks, readStackTicks);
            Interlocked.Add(ref stat.HandleTicks, handleTicks);
        }

        private static void DumpHostProfile()
        {
            if (!ProfileEnabled || Interlocked.Exchange(ref s_profileDumped, 1) != 0)
                return;

            static double TicksToMicroseconds(long ticks) =>
                ticks * 1_000_000d / Stopwatch.Frequency;

            Console.Error.WriteLine("[neo-riscv][profile] Host callback profile:");
            foreach (var entry in HostProfileStats
                         .OrderByDescending(pair => pair.Value.ReadStackTicks + pair.Value.HandleTicks)
                         .Take(20))
            {
                var stat = entry.Value;
                var count = Math.Max(1, stat.Count);
                var readUs = TicksToMicroseconds(stat.ReadStackTicks);
                var handleUs = TicksToMicroseconds(stat.HandleTicks);
                Console.Error.WriteLine(
                    $"[neo-riscv][profile] api=0x{entry.Key:x8} name={stat.Name} count={stat.Count} avg_items={(double)stat.InputItems / count:F2} read_us={readUs:F3} handle_us={handleUs:F3} total_us={readUs + handleUs:F3}");
            }

            Console.Error.WriteLine("[neo-riscv][profile] System.Contract.Call phases:");
            foreach (var entry in CallContractPhaseStats.OrderByDescending(pair => pair.Value.Ticks).Take(20))
            {
                var count = Math.Max(1, entry.Value.Count);
                var totalUs = TicksToMicroseconds(entry.Value.Ticks);
                Console.Error.WriteLine(
                    $"[neo-riscv][profile] phase={entry.Key} count={entry.Value.Count} avg_us={totalUs / count:F3} total_us={totalUs:F3}");
            }
        }

        private static void RecordCallContractPhase(string phase, long ticks)
        {
            if (!ProfileEnabled || ticks <= 0)
                return;

            var stat = CallContractPhaseStats.GetOrAdd(phase, _ => new PhaseProfileStat());
            Interlocked.Increment(ref stat.Count);
            Interlocked.Add(ref stat.Ticks, ticks);
        }
    }
}
