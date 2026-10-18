using System;
using System.Collections.Generic;
using System.Text.Json;

namespace UnityEngine
{
    public static class Application { public static string dataPath; }
    public static class Debug
    {
        public static readonly List<string> Messages = new List<string>();
        public static void LogError(string message, object context = null) => Messages.Add(message);
        public static void LogWarning(string message, object context = null) => Messages.Add(message);
    }
    public static class JsonUtility
    {
        public static T FromJson<T>(string text) => JsonSerializer.Deserialize<T>(text,
            new JsonSerializerOptions { IncludeFields = true });
    }
}

namespace UnityEditor
{
    [AttributeUsage(AttributeTargets.Method)]
    public sealed class MenuItem : Attribute { public MenuItem(string path) { } }
    public static class Selection { public static object activeObject; }
    public enum ImportAssetOptions { ForceUpdate }
    public static class AssetDatabase
    {
        public const string Guid = "0123456789abcdef0123456789abcdef";
        public static int Imports;
        public static string[] FindAssets(string filter) => new[] { Guid };
        public static string GUIDToAssetPath(string guid) => "Assets/Schema.asset";
        public static T LoadAssetAtPath<T>(string path) where T : class => Selection.activeObject as T;
        public static string GetAssetPath(object asset) => "Assets/Schema.asset";
        public static string AssetPathToGUID(string path) => Guid;
        public static void ImportAsset(string path, ImportAssetOptions options) { Imports++; }
    }
    public static class EditorApplication
    {
        public static event Action update;
        public static event Action quitting;
        public static void Tick() => update?.Invoke();
        public static void Quit() => quitting?.Invoke();
    }
    public static class AssemblyReloadEvents
    {
        public static event Action beforeAssemblyReload;
        public static void Reload() => beforeAssemblyReload?.Invoke();
    }
}
