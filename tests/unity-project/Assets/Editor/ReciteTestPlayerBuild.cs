using System;
using System.Linq;
using UnityEditor;
using UnityEditor.Build;
using UnityEditor.TestTools;
using UnityEngine.TestTools;

[assembly: TestPlayerBuildModifier(typeof(ReciteTestPlayerBuild))]
[assembly: PostBuildCleanup(typeof(ReciteTestPlayerBuild))]

public sealed class ReciteTestPlayerBuild : ITestPlayerBuildModifier, IPostBuildCleanup
{
    public BuildPlayerOptions ModifyOptions(BuildPlayerOptions options)
    {
        if (options.target == BuildTarget.StandaloneLinux64)
        {
            var expectedName = Argument("-recitePlayerBackend");
            var playerPath = Argument("-recitePlayerPath");
            if (string.IsNullOrEmpty(expectedName) || string.IsNullOrEmpty(playerPath)
                || !Enum.TryParse(expectedName, out ScriptingImplementation expected)
                || !Enum.IsDefined(typeof(ScriptingImplementation), expected))
            {
                throw new InvalidOperationException("The Linux test player needs an available explicit scripting backend and output path.");
            }

            var actual = PlayerSettings.GetScriptingBackend(NamedBuildTarget.Standalone);
            if (actual != expected)
            {
                throw new InvalidOperationException($"Test player backend is {actual}, expected {expected}.");
            }

            options.options &= ~BuildOptions.AutoRunPlayer;
            options.locationPathName = playerPath;
        }

        return options;
    }

    public void Cleanup()
    {
        if (!string.IsNullOrEmpty(Argument("-recitePlayerPath"))
            && Environment.GetCommandLineArgs().Contains("-runTests"))
        {
            EditorApplication.update += ExitAfterBuild;
        }
    }

    private static void ExitAfterBuild()
    {
        EditorApplication.update -= ExitAfterBuild;
        EditorApplication.Exit(0);
    }

    private static string Argument(string name)
    {
        var args = Environment.GetCommandLineArgs();
        for (var index = 0; index + 1 < args.Length; index++)
        {
            if (args[index] == name)
            {
                return args[index + 1];
            }
        }

        return null;
    }
}

public static class RecitePlayerBackend
{
    public static void ConfigureCoreClr()
    {
        if (!Enum.TryParse("CoreCLR", out ScriptingImplementation backend)
            || !Enum.IsDefined(typeof(ScriptingImplementation), backend))
        {
            throw new NotSupportedException("This Unity Editor does not offer the CoreCLR scripting backend.");
        }

        PlayerSettings.SetScriptingBackend(NamedBuildTarget.Standalone, backend);
        if (PlayerSettings.GetScriptingBackend(NamedBuildTarget.Standalone) != backend)
        {
            throw new InvalidOperationException("Unity did not select the CoreCLR scripting backend.");
        }
    }
}
