#if !UNITY_EDITOR
using System;
using System.IO;
using NUnit.Framework.Interfaces;
using UnityEngine;
using UnityEngine.TestRunner;

[assembly: TestRunCallback(typeof(RecitePlayerTestResult))]

public sealed class RecitePlayerTestResult : ITestRunCallback
{
    public void RunStarted(ITest testsToRun) { }

    public void RunFinished(ITestResult result)
    {
        var args = Environment.GetCommandLineArgs();
        for (var index = 0; index + 1 < args.Length; index++)
        {
            if (args[index] != "-reciteResultPath")
            {
                continue;
            }

            var path = args[index + 1];
            File.WriteAllText(path, result.ToXml(true).OuterXml);
            Application.Quit();
            return;
        }

        Debug.LogError("Missing -reciteResultPath for standalone test player.");
        Application.Quit();
    }

    public void TestStarted(ITest test) { }
    public void TestFinished(ITestResult result) { }
}
#endif
