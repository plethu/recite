using System.IO;
using UnityEditor;
using UnityEditor.SceneManagement;
using UnityEngine;
using UnityEngine.SceneManagement;
using Recite.Unity.GameObjects;

namespace Recite.Unity.Editor.Tests
{
    public static class ReciteTestSceneBootstrap
    {
        public const string ScenePath = "Assets/ReciteRunnerLifecycle.unity";
        public const string AssetPath = "Assets/ReciteRunnerLifecycle.recitec";

        public static void Create()
        {
            var fixture = Path.Combine(Application.dataPath,
                "../Packages/com.recite.dialogue/Tests/Runtime/Resources/Basic.recitec");
            File.WriteAllBytes(Path.Combine(Application.dataPath, "ReciteRunnerLifecycle.recitec"),
                File.ReadAllBytes(fixture));
            AssetDatabase.ImportAsset(AssetPath, ImportAssetOptions.ForceUpdate);
            var compiled = AssetDatabase.LoadAssetAtPath<ReciteCompiledAsset>(AssetPath);
            if (compiled == null) throw new IOException("Recite test fixture did not import");
            var scene = EditorSceneManager.NewScene(NewSceneSetup.EmptyScene, NewSceneMode.Single);
            var runner = new GameObject("Recite scene runner").AddComponent<ReciteDialogueRunner>();
            runner.CompiledAsset = compiled;
            if (!EditorSceneManager.SaveScene(scene, ScenePath))
                throw new IOException("Could not save Recite lifecycle test scene");
        }
    }
}
