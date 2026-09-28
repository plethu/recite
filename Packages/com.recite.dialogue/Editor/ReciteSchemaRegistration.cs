using System;
using UnityEngine;

namespace Recite.Unity.Editor
{
    [CreateAssetMenu(menuName = "Recite/Schema Registration", fileName = "ReciteSchema")]
    public sealed class ReciteSchemaRegistration : ScriptableObject
    {
        [Tooltip("recite executable on PATH, or an absolute path to it")]
        public string reciteExecutable = "recite";
        [Tooltip("Generated canonical manifest path inside Assets")]
        public string outputAssetPath = "Assets/Dialogue/recite-schema.json";
        public ReciteSchemaSymbols[] enums = Array.Empty<ReciteSchemaSymbols>();
        public ReciteSchemaSymbols[] registries = Array.Empty<ReciteSchemaSymbols>();
        public ReciteSchemaSpeaker[] speakers = Array.Empty<ReciteSchemaSpeaker>();
        public ReciteSchemaCondition[] conditions = Array.Empty<ReciteSchemaCondition>();
        public ReciteSchemaEffect[] effects = Array.Empty<ReciteSchemaEffect>();
    }

    [Serializable]
    public sealed class ReciteSchemaSymbols
    {
        public string name;
        public string[] values = Array.Empty<string>();
    }

    [Serializable]
    public sealed class ReciteSchemaSpeaker
    {
        public string name;
        public string displayName;
    }

    [Serializable]
    public sealed class ReciteSchemaParameter
    {
        public string name;
        [Tooltip("Recite type, such as speaker, int, enum:mood, or registry:item")]
        public string type;
    }

    [Serializable]
    public sealed class ReciteSchemaCondition
    {
        public string name;
        public string returns = "bool";
        public ReciteSchemaParameter[] parameters = Array.Empty<ReciteSchemaParameter>();
    }

    [Serializable]
    public sealed class ReciteSchemaEffect
    {
        public string name;
        public bool immediate = true;
        public bool blocking;
        public bool deferred;
        public ReciteSchemaParameter[] parameters = Array.Empty<ReciteSchemaParameter>();
    }
}
