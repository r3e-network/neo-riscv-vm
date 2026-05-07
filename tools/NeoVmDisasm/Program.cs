using System.Text;
using System.Text.Json;
using Neo.VM;
using Neo.VM.Types;

static async Task<JsonElement> Rpc(string method, string parametersJson)
{
    using var http = new HttpClient { Timeout = TimeSpan.FromSeconds(30) };
    using var response = await http.PostAsync(
        "http://seed1.neo.org:10332",
        new StringContent($$"""{"jsonrpc":"2.0","method":"{{method}}","params":{{parametersJson}},"id":1}""", Encoding.UTF8, "application/json"));
    response.EnsureSuccessStatusCode();
    using var document = JsonDocument.Parse(await response.Content.ReadAsStringAsync());
    return document.RootElement.GetProperty("result").Clone();
}

static string Operand(Instruction instruction)
{
    var bytes = instruction.Operand.ToArray();
    if (bytes.Length == 0) return "";
    var hex = Convert.ToHexString(bytes).ToLowerInvariant();
    if (instruction.OpCode is >= OpCode.PUSHDATA1 and <= OpCode.PUSHDATA4)
    {
        try
        {
            return $"{hex} \"{instruction.TokenString}\"";
        }
        catch
        {
            return hex;
        }
    }
    return hex;
}

static string Describe(StackItem item)
{
    return item switch
    {
        Neo.VM.Types.Boolean b => $"Boolean({b.GetBoolean()})",
        Integer i => $"Integer({i.GetInteger()})",
        ByteString b => $"ByteString({Convert.ToHexString(b.GetSpan())})",
        Null => "Null",
        _ => item.GetType().Name,
    };
}

static void RunSemantics(string name, byte[] bytes)
{
    using var engine = new ExecutionEngine();
    engine.LoadScript(bytes);
    var state = engine.Execute();
    Console.WriteLine($"{name}: state={state} stack=[{string.Join(", ", engine.ResultStack.ToArray().Select(Describe))}]");
}

if (args.ElementAtOrDefault(0) == "semantics")
{
    RunSemantics("PUSH1 ISTYPE Integer", [(byte)OpCode.PUSH1, (byte)OpCode.ISTYPE, 0x21, (byte)OpCode.RET]);
    RunSemantics("PUSHNULL ISNULL", [(byte)OpCode.PUSHNULL, (byte)OpCode.ISNULL, (byte)OpCode.RET]);
    RunSemantics("PUSH1 ISNULL", [(byte)OpCode.PUSH1, (byte)OpCode.ISNULL, (byte)OpCode.RET]);
    RunSemantics("PUSH0 SIZE", [(byte)OpCode.PUSH0, (byte)OpCode.SIZE, (byte)OpCode.RET]);
    RunSemantics("PUSH1 SIZE", [(byte)OpCode.PUSH1, (byte)OpCode.SIZE, (byte)OpCode.RET]);
    RunSemantics("PUSHINT8 -1 SIZE", [(byte)OpCode.PUSHINT8, unchecked((byte)-1), (byte)OpCode.SIZE, (byte)OpCode.RET]);
    RunSemantics("PUSHT SIZE", [(byte)OpCode.PUSHT, (byte)OpCode.SIZE, (byte)OpCode.RET]);
    RunSemantics("PUSHF SIZE", [(byte)OpCode.PUSHF, (byte)OpCode.SIZE, (byte)OpCode.RET]);
    RunSemantics("PUSHNULL SIZE", [(byte)OpCode.PUSHNULL, (byte)OpCode.SIZE, (byte)OpCode.RET]);
    return;
}

JsonElement contract;
string hash;
int center;
int radius;
if (args.ElementAtOrDefault(0) is "--json")
{
    var path = args.ElementAtOrDefault(1) ?? throw new ArgumentException("--json requires a contract JSON file");
    using var document = JsonDocument.Parse(File.ReadAllText(path));
    contract = document.RootElement.TryGetProperty("result", out var result)
        ? result.Clone()
        : document.RootElement.Clone();
    hash = contract.TryGetProperty("hash", out var hashElement)
        ? hashElement.GetString() ?? path
        : path;
    center = args.Length > 2 ? int.Parse(args[2]) : 3740;
    radius = args.Length > 3 ? int.Parse(args[3]) : 120;
}
else
{
    hash = args.ElementAtOrDefault(0) ?? "0x76a8f8a7a901b29a33013b469949f4b08db15756";
    center = args.Length > 1 ? int.Parse(args[1]) : 3740;
    radius = args.Length > 2 ? int.Parse(args[2]) : 120;
    contract = await Rpc("getcontractstate", $$"""["{{hash}}"]""");
}
var scriptBase64 = contract.GetProperty("nef").GetProperty("script").GetString()
    ?? throw new InvalidOperationException("Missing nef.script");
var scriptBytes = Convert.FromBase64String(scriptBase64);
var script = new Script(scriptBytes);

Console.WriteLine($"contract={hash} scriptLen={scriptBytes.Length} center={center} radius={radius}");
Console.WriteLine("ABI methods:");
foreach (var method in contract.GetProperty("manifest").GetProperty("abi").GetProperty("methods").EnumerateArray())
{
    var name = method.GetProperty("name").GetString();
    var offset = method.GetProperty("offset").GetInt32();
    if (Math.Abs(offset - center) <= radius * 4 || offset <= center)
        Console.WriteLine($"{offset,5} {name}");
}

Console.WriteLine();
Console.WriteLine("Instructions:");
var ip = 0;
while (ip < script.Length)
{
    var instruction = script.GetInstruction(ip);
    var next = ip + instruction.Size;
    if (ip >= center - radius && ip <= center + radius)
    {
        var marker = ip == center ? "=>" : "  ";
        Console.WriteLine($"{marker} {ip,5} {instruction.OpCode,-14} {Operand(instruction)}");
    }
    ip = next;
}
