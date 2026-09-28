using System;
using System.IO;
using System.Text;
using System.Text.RegularExpressions;
using UnityEditor;
using UnityEngine;

namespace Recite.Unity.Editor
{
    // TextAsset .recitec references used fileID 4900000 before ScriptedImporter.
    // Migrate only ReciteDialogueRunner components and only resolvable assets.
    public static class ReciteLegacyReferenceMigration
    {
        private static readonly Regex component = new Regex(@"(?ms)^--- !u!114 &.*?(?=^--- !u!|\z)");
        private static readonly Regex oldAsset = new Regex(@"(?m)^(\s*compiledAsset: \{fileID: )4900000(, guid: ([0-9a-f]{32}), type: 3\})$");

        [MenuItem("Tools/Recite/Migrate Legacy Compiled Asset References")]
        public static void MigrateProject()
        {
            var root = Application.dataPath;
            var changed = 0;
            foreach (var path in Directory.GetFiles(root, "*.unity", SearchOption.AllDirectories))
                if (MigrateFile(path)) changed++;
            foreach (var path in Directory.GetFiles(root, "*.prefab", SearchOption.AllDirectories))
                if (MigrateFile(path)) changed++;
            AssetDatabase.Refresh();
            Debug.Log("Recite migrated " + changed + " scene/prefab files. Original files are in Library/Recite/MigrationBackups.");
        }

        public static bool MigrateFile(string path)
        {
            var runnerGuid = AssetDatabase.AssetPathToGUID(
                "Packages/com.recite.dialogue/Runtime/GameObjects/ReciteDialogueRunner.cs");
            if (string.IsNullOrEmpty(runnerGuid))
                throw new InvalidOperationException("ReciteDialogueRunner script GUID is unavailable");
            var original = File.ReadAllText(path);
            var changed = false;
            var migrated = component.Replace(original, block =>
            {
                if (!block.Value.Contains("m_Script: {fileID: 11500000, guid: " + runnerGuid + ","))
                    return block.Value;
                return oldAsset.Replace(block.Value, match =>
                {
                    var assetPath = AssetDatabase.GUIDToAssetPath(match.Groups[3].Value);
                    if (string.IsNullOrEmpty(assetPath) ||
                        !assetPath.EndsWith(".recitec", StringComparison.OrdinalIgnoreCase))
                        return match.Value;
                    var asset = AssetDatabase.LoadAssetAtPath<ReciteCompiledAsset>(assetPath);
                    if (asset == null || !AssetDatabase.TryGetGUIDAndLocalFileIdentifier(
                        asset, out string resolvedGuid, out long localId) || resolvedGuid != match.Groups[3].Value)
                        return match.Value;
                    changed = true;
                    return match.Groups[1].Value + localId + match.Groups[2].Value;
                });
            });
            if (!changed) return false;
            var project = Directory.GetParent(Application.dataPath).FullName;
            var relative = path.Substring(project.Length).TrimStart(Path.DirectorySeparatorChar);
            var backup = Path.Combine(project, "Library", "Recite", "MigrationBackups", relative +
                "." + Guid.NewGuid().ToString("N") + ".bak");
            Directory.CreateDirectory(Path.GetDirectoryName(backup));
            File.Copy(path, backup);
            var temporary = path + ".recite-migrating";
            try
            {
                File.WriteAllText(temporary, migrated, new UTF8Encoding(false));
                File.Replace(temporary, path, null);
            }
            finally
            {
                if (File.Exists(temporary)) File.Delete(temporary);
            }
            return true;
        }
    }
}
