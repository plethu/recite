using System;
using System.Diagnostics;
using System.IO;
using System.Reflection;
using System.Threading;
using Recite.Unity.Editor;

internal static class ReciteSchemaExportProcessTests
{
    internal static void Child(string release)
    {
        Console.WriteLine(new string('é', 100000) + "😀 stdout");
        Console.Error.WriteLine(new string('x', 100000) + " stderr");
        var deadline = Stopwatch.StartNew();
        while (!File.Exists(release))
        {
            if (deadline.Elapsed > TimeSpan.FromSeconds(10)) Environment.Exit(52);
            Thread.Sleep(1);
        }
    }

    internal static void Run(string directory)
    {
        var failedSource = Path.Combine(directory, "failed-start.toml");
        var failedStage = Path.Combine(directory, "failed-start.json");
        try
        {
            using var unexpected = new ReciteSchemaExportProcess(new ProcessStartInfo
            {
                FileName = Path.Combine(directory, "missing-cli"),
                UseShellExecute = false
            }, failedSource, failedStage, "private input");
            throw new Exception("missing executable unexpectedly started");
        }
        catch (System.ComponentModel.Win32Exception) { }
        Require(!File.Exists(failedSource) && !File.Exists(failedStage), "failed start retained private files");
        foreach (var cancelled in new[] { false, true })
        {
            var token = Guid.NewGuid().ToString("N");
            var source = Path.Combine(directory, token + ".toml");
            var staged = Path.Combine(directory, token + ".json");
            var release = Path.Combine(directory, token + ".release");
            File.WriteAllText(staged, "unpublished staging");
            var start = new ProcessStartInfo
            {
                FileName = Process.GetCurrentProcess().MainModule.FileName,
                Arguments = "\"" + Assembly.GetExecutingAssembly().Location + "\" --export-child \"" + release + "\"",
                UseShellExecute = false,
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                CreateNoWindow = true
            };
            using (var job = new ReciteSchemaExportProcess(start, source, staged, "schema input"))
            {
                // The child cannot exit until explicitly released. Every poll must
                // return to the owner, independently of scheduling or CPU speed.
                for (var frame = 0; frame < 100; frame++)
                    Require(!job.Poll(), "poll blocked or completed a gated child");
                Require(File.ReadAllText(staged) == "unpublished staging", "running export published data");
                if (cancelled) job.Cancel("owner unloaded");
                else File.WriteAllText(release, "continue");
                var deadline = Stopwatch.StartNew();
                while (!job.Poll() && deadline.Elapsed < TimeSpan.FromSeconds(10)) Thread.Sleep(1);
                Require(job.Poll(), "child did not complete and drain its pipes");
                if (cancelled) Require(job.Failure == "owner unloaded", "cancellation was lost");
                else
                {
                    Require(job.ExitCode == 0 && job.Failure == null, "successful child rejected");
                    Require(job.Output.Count == 1 && job.Output[0].EndsWith("😀 stdout"), "stdout lost or truncated");
                    Require(job.Errors.Count == 1 && job.Errors[0].EndsWith(" stderr"), "stderr not drained");
                }
            }
            Require(!File.Exists(source) && !File.Exists(staged), "job retained private files after exit");
            if (File.Exists(release)) File.Delete(release);
        }
    }

    private static void Require(bool condition, string message)
    {
        if (!condition) throw new Exception(message);
    }
}
