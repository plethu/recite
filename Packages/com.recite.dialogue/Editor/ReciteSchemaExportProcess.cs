using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Text;

namespace Recite.Unity.Editor
{
    // Owns one producer child and its private files. Polling never waits for exit or pipe EOF.
    internal sealed class ReciteSchemaExportProcess : IDisposable
    {
        private readonly Process process;
        private readonly string sourcePath;
        private readonly string stagedPath;
        private readonly Stopwatch clock = Stopwatch.StartNew();
        private readonly object gate = new object();
        private bool outputClosed;
        private bool errorClosed;
        private bool disposed;
        private bool started;
        private bool cleaned;
        private long cancelledAt;
        internal readonly List<string> Output = new List<string>();
        internal readonly List<string> Errors = new List<string>();
        internal string Failure { get; private set; }
        internal int ExitCode => process.ExitCode;

        internal ReciteSchemaExportProcess(ProcessStartInfo start, string sourcePath, string stagedPath, string sourceText)
        {
            this.sourcePath = sourcePath;
            this.stagedPath = stagedPath;
            process = new Process { StartInfo = start, EnableRaisingEvents = true };
            process.Exited += (_, __) =>
            {
                lock (gate) { if (disposed) Cleanup(); }
            };
            process.OutputDataReceived += (_, args) => Receive(Output, args.Data, true);
            process.ErrorDataReceived += (_, args) => Receive(Errors, args.Data, false);
            try
            {
                File.WriteAllText(sourcePath, sourceText, new UTF8Encoding(false));
                process.Start();
                started = true;
                process.BeginOutputReadLine();
                process.BeginErrorReadLine();
            }
            catch
            {
                Dispose();
                throw;
            }
        }

        private void Receive(List<string> lines, string text, bool stdout)
        {
            lock (gate)
            {
                if (text != null) lines.Add(text);
                else if (stdout) outputClosed = true;
                else errorClosed = true;
            }
        }

        internal bool Poll()
        {
            if (disposed) throw new ObjectDisposedException(nameof(ReciteSchemaExportProcess));
            if (clock.ElapsedMilliseconds >= 120000 && Failure == null)
                Cancel("Recite schema export timed out; its result could not be confirmed.");
            if (Failure != null && clock.ElapsedMilliseconds - cancelledAt >= 10000) return true;
            if (!process.HasExited) return false;
            lock (gate) return outputClosed && errorClosed;
        }

        internal void Cancel(string reason)
        {
            if (disposed) return;
            if (Failure == null) cancelledAt = clock.ElapsedMilliseconds;
            Failure = reason;
            try
            {
                if (!process.HasExited) process.Kill();
            }
            catch (InvalidOperationException) { }
            catch (System.ComponentModel.Win32Exception error)
            {
                Failure += " " + error.Message;
            }
        }

        public void Dispose()
        {
            if (disposed) return;
            Cancel("Recite schema export cancelled.");
            lock (gate)
            {
                disposed = true;
                // If kill has not completed, the exit event retains this owner and
                // cleans its private files later. Editor teardown never waits.
                if (!started || process.HasExited) Cleanup();
            }
        }

        private void Cleanup()
        {
            if (cleaned) return;
            cleaned = true;
            process.Dispose();
            try
            {
                if (File.Exists(sourcePath)) File.Delete(sourcePath);
                if (File.Exists(stagedPath)) File.Delete(stagedPath);
            }
            catch (Exception error) when (error is IOException || error is UnauthorizedAccessException)
            {
                Console.Error.WriteLine("Cannot remove Recite export staging: " + error.Message);
            }
        }
    }
}
