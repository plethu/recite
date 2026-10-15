using System;
using System.Diagnostics;
using System.IO;
using System.Reflection;
using System.Linq;
using System.Threading;
using Recite.Unity.Editor;
using UnityEditor;

internal static class ReciteSchemaExporterTests
{
    internal static void Child(string[] args)
    {
        Console.Error.WriteLine(new string('é', 100000));
        var release = Environment.GetEnvironmentVariable("RECITE_EXPORT_RELEASE");
        var deadline = Stopwatch.StartNew();
        while (!File.Exists(release))
        {
            if (deadline.Elapsed > TimeSpan.FromSeconds(10)) Environment.Exit(52);
            Thread.Sleep(1);
        }
        var start = new ProcessStartInfo
        {
            FileName = Environment.GetEnvironmentVariable("RECITE_UNITY_CLI"),
            UseShellExecute = false
        };
        foreach (var arg in args) start.ArgumentList.Add(arg);
        using (var cli = Process.Start(start))
        {
            cli.WaitForExit();
            Environment.Exit(cli.ExitCode);
        }
    }

    internal static void Run(string directory)
    {
        var project = Path.Combine(directory, "menu-export");
        UnityEngine.Application.dataPath = Path.Combine(project, "Assets");
        Directory.CreateDirectory(UnityEngine.Application.dataPath);
        var executable = Path.ChangeExtension(Assembly.GetExecutingAssembly().Location, null);
        if (OperatingSystem.IsWindows()) executable += ".exe";
        var registration = new ReciteSchemaRegistration { reciteExecutable = executable, outputAssetPath = "Assets/schema.json" };
        registration.conditions = new ReciteSchemaCondition[1000];
        for (var index = 0; index < registration.conditions.Length; index++)
            registration.conditions[index] = new ReciteSchemaCondition { name = "gate_" + index.ToString("D4") };
        Selection.activeObject = registration;
        var output = Path.Combine(project, registration.outputAssetPath);
        var release = Path.Combine(directory, "export-release");
        Environment.SetEnvironmentVariable("RECITE_EXPORT_RELEASE", release);
        try
        {
            foreach (var mode in new[] { "success", "stale", "reload", "quit" })
            {
                File.WriteAllText(output, "previous manifest");
                var imports = AssetDatabase.Imports;
                var messages = UnityEngine.Debug.Messages.Count;
                ReciteSchemaExporter.ExportSelected();
                for (var frame = 0; frame < 100; frame++) EditorApplication.Tick();
                Require(File.ReadAllText(output) == "previous manifest", "menu published before child exit");
                ReciteSchemaExporter.ExportSelected();
                Require(UnityEngine.Debug.Messages.Count == messages + 1, "menu did not bound concurrent exports");
                if (mode == "reload") AssemblyReloadEvents.Reload();
                else if (mode == "quit") EditorApplication.Quit();
                else
                {
                    if (mode == "stale") registration.conditions = new[] { new ReciteSchemaCondition { name = "changed" } };
                    File.WriteAllText(release, "continue");
                    var deadline = Stopwatch.StartNew();
                    while (deadline.Elapsed < TimeSpan.FromSeconds(10))
                    {
                        EditorApplication.Tick();
                        if (AssetDatabase.Imports > imports || UnityEngine.Debug.Messages.Count > messages + 1) break;
                        Thread.Sleep(1);
                    }
                    Require(mode == "success" ? AssetDatabase.Imports == imports + 1 : UnityEngine.Debug.Messages.Count == messages + 2,
                        "menu job did not complete: " + string.Join("; ", UnityEngine.Debug.Messages));
                }
                if (mode == "success")
                {
                    using var manifest = System.Text.Json.JsonDocument.Parse(File.ReadAllText(output));
                    Require(manifest.RootElement.GetProperty("conditions").EnumerateObject().Count() == 1000,
                        "large schema export lost declarations");
                }
                if (mode != "success") Require(File.ReadAllText(output) == "previous manifest", "stale/cancelled menu export published");
                if (File.Exists(release)) File.Delete(release);
            }
            // Existing synchronous callers still receive the completed publication result.
            File.WriteAllText(release, "continue");
            Require(ReciteSchemaExporter.Export(registration), "synchronous automation export failed");
            var canonical = File.ReadAllText(output);
            Require(ReciteSchemaExporter.Export(registration) && File.ReadAllText(output) == canonical,
                "private staging paths changed canonical schema output");
        }
        finally
        {
            EditorApplication.Quit();
            Environment.SetEnvironmentVariable("RECITE_EXPORT_RELEASE", null);
            if (File.Exists(release)) File.Delete(release);
        }
    }

    private static void Require(bool condition, string message)
    {
        if (!condition) throw new Exception(message);
    }
}
