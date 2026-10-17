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
        private static PendingExport active;

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
            if (active != null)
            {
                Debug.LogWarning("A Recite schema export is already running.", registration);
                return;
            }
            try
            {
                active = Prepare(registration);
                EditorApplication.update += PollExport;
                EditorApplication.quitting += CancelExport;
                AssemblyReloadEvents.beforeAssemblyReload += CancelExport;
            }
            catch (Exception error) when (error is System.ComponentModel.Win32Exception || error is IOException)
            {
                Debug.LogError("Cannot start Recite schema export: " + error.Message, registration);
            }
        }

        // Synchronous automation entry point; the editor menu uses the polled job above.
        public static bool Export(ReciteSchemaRegistration registration)
        {
            try
            {
                using (var pending = Prepare(registration))
                {
                    while (!pending.Process.Poll()) System.Threading.Thread.Sleep(1);
                    return Complete(pending);
                }
            }
            catch (Exception error) when (error is System.ComponentModel.Win32Exception || error is IOException)
            {
                Debug.LogError("Cannot complete Recite schema export: " + error.Message, registration);
                return false;
            }
        }

        private static void PollExport()
        {
            if (active == null || !active.Process.Poll()) return;
            var pending = active;
            try { Complete(pending); }
            catch (Exception error) when (error is IOException || error is UnauthorizedAccessException)
            {
                Debug.LogError("Cannot complete Recite schema export: " + error.Message, pending.Registration);
            }
            finally { CancelExport(); }
        }

        private static void CancelExport()
        {
            EditorApplication.update -= PollExport;
            EditorApplication.quitting -= CancelExport;
            AssemblyReloadEvents.beforeAssemblyReload -= CancelExport;
            var pending = active;
            active = null;
            pending?.Dispose();
        }

        private static PendingExport Prepare(ReciteSchemaRegistration registration)
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
            var sourcePath = Path.Combine(sourceDir, producerId + "." + Guid.NewGuid().ToString("N") + ".toml");
            var sourceText = ReciteSchemaToml.Generate(registration, producerId);
            Directory.CreateDirectory(Path.GetDirectoryName(outputPath));
            var stagedOutputPath = Path.Combine(Path.GetDirectoryName(outputPath), ".recite-schema-" + Guid.NewGuid().ToString("N") + ".json");

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
                StandardOutputEncoding = Encoding.UTF8,
                StandardErrorEncoding = Encoding.UTF8,
                CreateNoWindow = true
            };
            return new PendingExport
            {
                Registration = registration,
                ProducerId = producerId,
                SourceText = sourceText,
                OutputAssetPath = output,
                OutputPath = outputPath,
                StagedPath = stagedOutputPath,
                Process = new ReciteSchemaExportProcess(start, sourcePath, stagedOutputPath, sourceText)
            };
        }

        private static bool Complete(PendingExport pending)
        {
            if (pending.Process.Failure != null)
            {
                Debug.LogError(pending.Process.Failure, pending.Registration);
                return false;
            }
            if (!ReportResult(pending.Process.Output, pending.Process.Errors,
                pending.Process.ExitCode, pending.Registration)) return false;
            var registration = pending.Registration;
            if (registration == null || registration.outputAssetPath != pending.OutputAssetPath ||
                AssetDatabase.AssetPathToGUID(AssetDatabase.GetAssetPath(registration)) != pending.ProducerId ||
                ReciteSchemaToml.Generate(registration, pending.ProducerId) != pending.SourceText)
            {
                Debug.LogError("Recite schema registration changed during export; export it again.", registration);
                return false;
            }
            if (!File.Exists(pending.StagedPath))
                throw new IOException("Recite reported success without a manifest artifact");
            if (File.Exists(pending.OutputPath)) File.Replace(pending.StagedPath, pending.OutputPath, null);
            else File.Move(pending.StagedPath, pending.OutputPath);
            AssetDatabase.ImportAsset(pending.OutputAssetPath, ImportAssetOptions.ForceUpdate);
            return true;
        }

        private sealed class PendingExport : IDisposable
        {
            internal ReciteSchemaRegistration Registration;
            internal string ProducerId;
            internal string SourceText;
            internal string OutputAssetPath;
            internal string OutputPath;
            internal string StagedPath;
            internal ReciteSchemaExportProcess Process;
            public void Dispose() => Process.Dispose();
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

        [Serializable]
        private sealed class ProtocolRecord
        {
            public int version = 0;
            public int sequence = 0;
            public string @event = null;
            public string command = null;
            public string status = null;
            public ProtocolData data = null;
            public ProtocolError error = null;
        }
        [Serializable] private sealed class ProtocolData { public ProtocolDiagnostic[] diagnostics = null; }
        [Serializable] private sealed class ProtocolDiagnostic { public string code = null; public string compatibility_message = null; }
        [Serializable] private sealed class ProtocolError { public string code = null; public string operation = null; }
    }
}
