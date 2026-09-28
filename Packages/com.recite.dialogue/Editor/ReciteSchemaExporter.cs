using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Text;
using UnityEditor;
using UnityEngine;
using Debug = UnityEngine.Debug;

namespace Recite.Unity.Editor
{
    public static class ReciteSchemaExporter
    {
        [MenuItem("Tools/Recite/Export Schema")]
        public static void ExportSelected()
        {
            var registration = Selection.activeObject as ReciteSchemaRegistration;
            if (registration == null)
            {
                var ids = AssetDatabase.FindAssets("t:ReciteSchemaRegistration");
                if (ids.Length == 1)
                    registration = AssetDatabase.LoadAssetAtPath<ReciteSchemaRegistration>(AssetDatabase.GUIDToAssetPath(ids[0]));
            }
            if (registration == null)
            {
                Debug.LogError("Select a ReciteSchemaRegistration asset, or keep exactly one in the project.");
                return;
            }
            Export(registration);
        }

        public static bool Export(ReciteSchemaRegistration registration)
        {
            if (registration == null) throw new ArgumentNullException(nameof(registration));
            var registrationPath = AssetDatabase.GetAssetPath(registration);
            if (string.IsNullOrEmpty(registrationPath))
                throw new ArgumentException("Save the schema registration asset before export", nameof(registration));
            var producerId = AssetDatabase.AssetPathToGUID(registrationPath);
            if (producerId == null || producerId.Length != 32)
                throw new ArgumentException("Schema registration has no stable Unity GUID", nameof(registration));
            foreach (var digit in producerId)
                if (!Uri.IsHexDigit(digit))
                    throw new ArgumentException("Schema registration GUID is malformed", nameof(registration));
            var output = registration.outputAssetPath;
            if (string.IsNullOrWhiteSpace(output) || !output.StartsWith("Assets/", StringComparison.Ordinal) ||
                !output.EndsWith(".json", StringComparison.OrdinalIgnoreCase) || output.Contains(".."))
                throw new ArgumentException("Schema output must be a JSON path inside Assets", nameof(registration));

            var projectRoot = Directory.GetParent(Application.dataPath).FullName;
            var outputPath = Path.GetFullPath(Path.Combine(projectRoot, output));
            if (!outputPath.StartsWith(Application.dataPath + Path.DirectorySeparatorChar, StringComparison.Ordinal))
                throw new ArgumentException("Schema output escapes Assets", nameof(registration));
            var sourceDir = Path.Combine(projectRoot, "Library", "Recite");
            Directory.CreateDirectory(sourceDir);
            var sourcePath = Path.Combine(sourceDir, producerId + ".toml");
            File.WriteAllText(sourcePath, ReciteSchemaToml.Generate(registration,
                producerId), new UTF8Encoding(false));
            Directory.CreateDirectory(Path.GetDirectoryName(outputPath));
            var stagedOutputPath = outputPath + "." + Guid.NewGuid().ToString("N") + ".json";

            var executable = string.IsNullOrWhiteSpace(registration.reciteExecutable)
                ? "recite" : registration.reciteExecutable;
            var start = new ProcessStartInfo
            {
                FileName = executable,
                Arguments = "export-schema --schema " + QuoteArgument(sourcePath) +
                    " --output " + QuoteArgument(stagedOutputPath) +
                    " --producer-kind unity --producer-id " + QuoteArgument(producerId) +
                    " --output-format structured",
                WorkingDirectory = projectRoot,
                UseShellExecute = false,
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                CreateNoWindow = true
            };
            var outputLines = new List<string>();
            var errorLines = new List<string>();
            var childMayWrite = false;
            try
            {
                using (var process = new Process { StartInfo = start })
                {
                    process.OutputDataReceived += (_, args) => { if (args.Data != null) outputLines.Add(args.Data); };
                    process.ErrorDataReceived += (_, args) => { if (args.Data != null) errorLines.Add(args.Data); };
                    process.Start();
                    process.BeginOutputReadLine();
                    process.BeginErrorReadLine();
                    if (!process.WaitForExit(120000))
                    {
                        try
                        {
                            process.Kill();
                            childMayWrite = !process.WaitForExit(10000);
                        }
                        catch (InvalidOperationException)
                        {
                            childMayWrite = !process.HasExited;
                        }
                        catch (System.ComponentModel.Win32Exception)
                        {
                            childMayWrite = !process.HasExited;
                        }
                        Debug.LogError("Recite schema export timed out; its result could not be confirmed.", registration);
                        return false;
                    }
                    process.WaitForExit();
                    if (ReportResult(outputLines, errorLines, process.ExitCode, registration))
                    {
                        if (!File.Exists(stagedOutputPath))
                            throw new IOException("Recite reported success without a manifest artifact");
                        if (File.Exists(outputPath)) File.Replace(stagedOutputPath, outputPath, null);
                        else File.Move(stagedOutputPath, outputPath);
                        AssetDatabase.ImportAsset(output, ImportAssetOptions.ForceUpdate);
                        return true;
                    }
                }
            }
            catch (Exception error) when (error is System.ComponentModel.Win32Exception || error is IOException)
            {
                Debug.LogError("Cannot complete Recite schema export: " + error.Message, registration);
            }
            finally
            {
                if (!childMayWrite && File.Exists(stagedOutputPath)) File.Delete(stagedOutputPath);
            }
            return false;
        }

        private static bool ReportResult(List<string> output, List<string> stderr, int exitCode,
            ReciteSchemaRegistration registration)
        {
            var records = new List<ProtocolRecord>();
            foreach (var line in output)
            {
                if (string.IsNullOrWhiteSpace(line)) continue;
                try
                {
                    var record = JsonUtility.FromJson<ProtocolRecord>(line);
                    if (record == null) throw new ArgumentException("empty record");
                    records.Add(record);
                }
                catch (ArgumentException)
                {
                    Debug.LogError("Recite returned malformed structured output.", registration);
                    return false;
                }
            }
            if (records.Count != 2 || records[0].version != 1 || records[0].sequence != 0 ||
                records[0].command != "export-schema" || records[0].@event != "command.started" ||
                records[1].version != 1 || records[1].sequence != 1 ||
                records[1].command != "export-schema" ||
                (records[1].@event != "command.result" && records[1].@event != "command.error"))
            {
                Debug.LogError("Recite returned an unsupported structured export response.", registration);
                return false;
            }
            var terminal = records[1];
            if (terminal.@event == "command.result" && terminal.status == "success" && exitCode == 0)
                return true;
            if (terminal.@event == "command.result" && terminal.status == "content_diagnostics" &&
                terminal.data?.diagnostics != null)
            {
                foreach (var diagnostic in terminal.data.diagnostics)
                    Debug.LogError(diagnostic.code + ": " + diagnostic.compatibility_message, registration);
            }
            else if (terminal.@event == "command.error" && terminal.error != null)
                Debug.LogError("Recite export failed (" + terminal.error.code + ", " +
                    terminal.error.operation + ")", registration);
            else
                Debug.LogError("Recite schema export failed: " + string.Join("\n", stderr), registration);
            return false;
        }

        // Project asset paths must not include literal quotes; surrounding quotes preserve spaces.
        private static string QuoteArgument(string path)
        {
            if (path.IndexOf('"') >= 0) throw new ArgumentException("Recite paths cannot contain quotes", nameof(path));
            return "\"" + path + "\"";
        }

        [Serializable] private sealed class ProtocolRecord
        {
            public int version;
            public int sequence;
            public string @event;
            public string command;
            public string status;
            public ProtocolData data;
            public ProtocolError error;
        }
        [Serializable] private sealed class ProtocolData { public ProtocolDiagnostic[] diagnostics; }
        [Serializable] private sealed class ProtocolDiagnostic { public string code; public string compatibility_message; }
        [Serializable] private sealed class ProtocolError { public string code; public string operation; }
    }
}
