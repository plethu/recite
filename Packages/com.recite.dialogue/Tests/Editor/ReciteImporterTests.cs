using System;
using System.IO;
using NUnit.Framework;
using UnityEditor;
using UnityEditor.SceneManagement;
using UnityEngine;
using UnityEngine.TestTools;
using UnityEngine.SceneManagement;
using System.Text.RegularExpressions;
using Recite.Unity.GameObjects;

namespace Recite.Unity.Editor.Tests
{
    public sealed class ReciteImporterTests
    {
        private const string TestPath = "Assets/ReciteImporterTest.recitec";
        private const string LegacyAssetPath = "Assets/ReciteLegacyTest.recitec";
        private const string LegacyScenePath = "Assets/ReciteLegacyTest.unity";

        [TearDown]
        public void Cleanup()
        {
            var guid = AssetDatabase.AssetPathToGUID(TestPath);
            AssetDatabase.DeleteAsset(TestPath);
            AssetDatabase.DeleteAsset(LegacyAssetPath);
            AssetDatabase.DeleteAsset(LegacyScenePath);
            if (!string.IsNullOrEmpty(guid))
            {
                var cache = Path.Combine(Application.dataPath, "../Library/Recite/CompiledCache", guid + ".recitec");
                if (File.Exists(cache)) File.Delete(cache);
            }
        }

        [Test]
        public void LegacyTextAssetSceneReferenceMigratesToImportedObject()
        {
            var package = Path.Combine(Application.dataPath, "../Packages/com.recite.dialogue");
            File.Copy(Path.Combine(package, "Tests/Runtime/Resources/Basic.recitec"),
                Path.Combine(Application.dataPath, "ReciteLegacyTest.recitec"), true);
            File.Copy(Path.Combine(package, "Tests~/Legacy/basic.recitec.meta"),
                Path.Combine(Application.dataPath, "ReciteLegacyTest.recitec.meta"), true);
            File.Copy(Path.Combine(package, "Tests~/Legacy/BasicDialogue.unity"),
                Path.Combine(Application.dataPath, "ReciteLegacyTest.unity"), true);
            // The fixture preserves the previous importer/local-ID shape, but
            // receives its own GUID to avoid colliding with the package sample.
            const string fixtureGuid = "9bde5e8e5ec74888a78b0c1827a94dcf";
            var testGuid = Guid.NewGuid().ToString("N");
            var metaPath = Path.Combine(Application.dataPath, "ReciteLegacyTest.recitec.meta");
            var scenePath = Path.Combine(Application.dataPath, "ReciteLegacyTest.unity");
            File.WriteAllText(metaPath, File.ReadAllText(metaPath).Replace(fixtureGuid, testGuid));
            File.WriteAllText(scenePath, File.ReadAllText(scenePath).Replace(fixtureGuid, testGuid));
            AssetDatabase.ImportAsset(LegacyAssetPath, ImportAssetOptions.ForceUpdate);
            Assert.That(ReciteLegacyReferenceMigration.MigrateFile(
                Path.Combine(Application.dataPath, "ReciteLegacyTest.unity")), Is.True);
            AssetDatabase.ImportAsset(LegacyScenePath, ImportAssetOptions.ForceUpdate);
            var yaml = File.ReadAllText(Path.Combine(Application.dataPath, "ReciteLegacyTest.unity"));
            Assert.That(yaml, Does.Not.Contain("compiledAsset: {fileID: 4900000"));
            EditorSceneManager.OpenScene(LegacyScenePath, OpenSceneMode.Single);
            var runner = UnityEngine.Object.FindObjectOfType<ReciteDialogueRunner>();
            Assert.That(runner, Is.Not.Null);
            Assert.That(runner.CompiledAsset, Is.Not.Null,
                "migrated scene must resolve the imported compiled asset");
            Assert.That(runner.CompiledAsset.AssetId, Is.EqualTo(
                AssetDatabase.LoadAssetAtPath<ReciteCompiledAsset>(LegacyAssetPath).AssetId));
        }

        [Test]
        public void UninitializedResourceReportsStructuredLoadError()
        {
            var resource = ScriptableObject.CreateInstance<ReciteCompiledAsset>();
            try
            {
                Assert.That(resource.ContentFingerprintDigest, Is.Null);
                var error = Assert.Throws<ReciteAdapterException>(() => resource.ToDialogueAsset());
                Assert.That(error.Status, Is.EqualTo(ReciteStatus.AssetLoadOrDecode));
            }
            finally
            {
                UnityEngine.Object.DestroyImmediate(resource);
            }
        }

        [Test]
        public void RejectedReimportKeepsLastValidRevisionAndRecoveryClearsError()
        {
            var source = Path.Combine(Application.dataPath, "../Packages/com.recite.dialogue/Tests/Runtime/Resources/Basic.recitec");
            var validBytes = File.ReadAllBytes(source);
            File.WriteAllBytes(Path.Combine(Application.dataPath, "ReciteImporterTest.recitec"), validBytes);
            AssetDatabase.ImportAsset(TestPath, ImportAssetOptions.ForceUpdate);
            var first = AssetDatabase.LoadAssetAtPath<ReciteCompiledAsset>(TestPath);
            Assert.That(first, Is.Not.Null);
            Assert.That(first.IsUsingRetainedRevision, Is.False);
            var id = first.AssetId;
            var digest = first.ContentFingerprintDigest;

            File.WriteAllBytes(Path.Combine(Application.dataPath, "ReciteImporterTest.recitec"), new byte[] { 0, 1, 2, 3 });
            LogAssert.Expect(LogType.Error, new Regex("Recite compiled asset import failed"));
            AssetDatabase.ImportAsset(TestPath, ImportAssetOptions.ForceUpdate);
            var retained = AssetDatabase.LoadAssetAtPath<ReciteCompiledAsset>(TestPath);
            Assert.That(retained, Is.Not.Null);
            Assert.That(retained.IsUsingRetainedRevision, Is.True);
            Assert.That(retained.ImportStatus, Is.Not.EqualTo(ReciteStatus.Ok));
            Assert.That(retained.AssetId, Is.EqualTo(id));
            Assert.That(retained.ContentFingerprintDigest, Is.EqualTo(digest));
            using (var service = new ReciteDialogueService())
            {
                service.RegisterCondition("has_key", _ => false);
                Assert.That(service.Start(retained.ToDialogueAsset()).Events.Count, Is.GreaterThan(0));
            }

            File.WriteAllBytes(Path.Combine(Application.dataPath, "ReciteImporterTest.recitec"), validBytes);
            AssetDatabase.ImportAsset(TestPath, ImportAssetOptions.ForceUpdate);
            var recovered = AssetDatabase.LoadAssetAtPath<ReciteCompiledAsset>(TestPath);
            Assert.That(recovered, Is.Not.Null);
            Assert.That(recovered.IsUsingRetainedRevision, Is.False);
            Assert.That(recovered.ImportStatus, Is.EqualTo(ReciteStatus.Ok));
        }
    }
}
