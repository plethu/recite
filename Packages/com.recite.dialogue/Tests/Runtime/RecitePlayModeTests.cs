using System.Collections;
using System.Collections.Generic;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.SceneManagement;
using UnityEngine.TestTools;
using Recite.Unity.GameObjects;
#if UNITY_EDITOR
using UnityEditor.SceneManagement;
#endif

namespace Recite.Unity.Tests
{
    public sealed class RecitePlayModeTests
    {
#if UNITY_EDITOR
        [UnityTest]
        public IEnumerator SceneLoadedRunnerEndsOnDisableAndDisposesOnDestroy()
        {
            EditorSceneManager.LoadSceneInPlayMode("Assets/ReciteRunnerLifecycle.unity",
                new LoadSceneParameters(LoadSceneMode.Single));
            yield return null;
            var runner = Object.FindObjectOfType<ReciteDialogueRunner>();
            Assert.That(runner, Is.Not.Null);
            Assert.That(runner.CompiledAsset, Is.Not.Null);
            runner.Service.RegisterCondition("has_key", _ => false);
            runner.StartDialogue();
            Assert.That(runner.Service.HasActiveSession, Is.True);
            runner.enabled = false;
            Assert.That(runner.Service.HasActiveSession, Is.False);
            runner.enabled = true;
            runner.StartDialogue();
            Assert.That(runner.Service.HasActiveSession, Is.True);
            Object.Destroy(runner.gameObject);
            yield return null;
            Assert.Throws<System.ObjectDisposedException>(() => runner.Service.Start(new ReciteDialogueAsset(new byte[] { 1 })));
        }

        [UnityTest]
        public IEnumerator ReentrantChoiceOutputFollowsEntireStartBatch()
        {
            EditorSceneManager.LoadSceneInPlayMode("Assets/ReciteRunnerLifecycle.unity",
                new LoadSceneParameters(LoadSceneMode.Single));
            yield return null;
            var runner = Object.FindObjectOfType<ReciteDialogueRunner>();
            Assert.That(runner, Is.Not.Null);
            runner.Service.RegisterCondition("has_key", _ => false);
            var seen = new List<string>();
            runner.Output.AddListener(item =>
            {
                seen.Add(item.Kind);
                if (item is ReciteLineOutput line && line.Line.Text == "The relay wakes.")
                    runner.SelectChoice("10000000000000000003");
            });
            runner.StartDialogue();
            Assert.That(seen, Does.Contain("prompt"));
            Assert.That(seen, Does.Contain("effect"));
            Assert.That(seen.IndexOf("prompt"), Is.LessThan(seen.LastIndexOf("effect")),
                "nested choice output arrived before the start batch completed");
        }
#endif

        [UnityTest]
        public IEnumerator ImportedResourceRunsInPlayer()
        {
            var resource = Resources.Load<ReciteCompiledAsset>("Basic");
            Assert.That(resource, Is.Not.Null, "compiled package test resource did not load");
            var runner = new GameObject("Recite player runner").AddComponent<ReciteDialogueRunner>();
            runner.CompiledAsset = resource;
            runner.Service.RegisterCondition("has_key", _ => false);
            var seen = new List<string>();
            runner.Output.AddListener(item => seen.Add(item.Kind));
            runner.StartDialogue();
            Assert.That(seen, Does.Contain("prompt"));
            Assert.That(runner.Service.HasActiveSession, Is.True);
            Object.Destroy(runner.gameObject);
            yield return null;
        }
    }
}
