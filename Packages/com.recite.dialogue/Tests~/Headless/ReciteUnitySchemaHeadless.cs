using System;
using System.IO;
using Recite.Unity.Editor;

internal static class ReciteUnitySchemaHeadless
{
    private static void Main(string[] args)
    {
        if (args.Length > 0 && args[0] == "export-schema")
        {
            ReciteSchemaExporterTests.Child(args);
            return;
        }
        if (args.Length == 2 && args[0] == "--export-child")
        {
            ReciteSchemaExportProcessTests.Child(args[1]);
            return;
        }
        ReciteSchemaExportProcessTests.Run(Path.GetDirectoryName(Path.GetFullPath(args[0])));
        ReciteSchemaExporterTests.Run(Path.GetDirectoryName(Path.GetFullPath(args[0])));
        var registration = new ReciteSchemaRegistration
        {
            enums = new[] { new ReciteSchemaSymbols { name = "mood", values = new[] { "calm", "tense" } } },
            registries = new[] { new ReciteSchemaSymbols { name = "item", values = new[] { "relay_key" } } },
            speakers = new[] { new ReciteSchemaSpeaker { name = "ada", displayName = "Ada \\\"Élan\\\"" } },
            conditions = new[] { new ReciteSchemaCondition
            {
                name = "has_key",
                returns = "bool",
                parameters = new[] { new ReciteSchemaParameter { name = "key", type = "registry:item" } }
            } },
            effects = new[] { new ReciteSchemaEffect
            {
                name = "grant_item", immediate = false, blocking = true,
                parameters = new[] { new ReciteSchemaParameter { name = "item", type = "registry:item" } }
            } }
        };
        var first = ReciteSchemaToml.Generate(registration, "unity:Assets/Dialogue/Schema.asset");
        var second = ReciteSchemaToml.Generate(registration, "unity:Assets/Dialogue/Schema.asset");
        if (first != second) throw new Exception("schema export is nondeterministic");
        File.WriteAllText(args[0], first);
        registration.conditions = new[] { registration.conditions[0], registration.conditions[0] };
        File.WriteAllText(args[1], ReciteSchemaToml.Generate(registration, "unity:Assets/Dialogue/Schema.asset"));
    }
}
