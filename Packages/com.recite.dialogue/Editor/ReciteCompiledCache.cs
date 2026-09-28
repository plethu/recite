using System;
using System.IO;
using UnityEditor;
using UnityEngine;

namespace Recite.Unity.Editor
{
    // Derived, project-local last-valid bytes for rejected reimports. The GUID
    // ties it to the Unity asset rather than the filename, which can change.
    internal static class ReciteCompiledCache
    {
        private static string PathFor(string assetPath)
        {
            var guid = AssetDatabase.AssetPathToGUID(assetPath);
            if (guid == null || guid.Length != 32)
                throw new IOException("compiled asset has no stable Unity GUID");
            foreach (var c in guid)
                if (!Uri.IsHexDigit(c)) throw new IOException("compiled asset GUID is malformed");
            var project = Directory.GetParent(Application.dataPath).FullName;
            return Path.Combine(project, "Library", "Recite", "CompiledCache", guid + ".recitec");
        }

        internal static void Save(string assetPath, byte[] bytes)
        {
            var path = PathFor(assetPath);
            Directory.CreateDirectory(Path.GetDirectoryName(path));
            var temporary = path + "." + Guid.NewGuid().ToString("N") + ".tmp";
            try
            {
                File.WriteAllBytes(temporary, bytes);
                if (File.Exists(path)) File.Replace(temporary, path, null);
                else File.Move(temporary, path);
            }
            finally
            {
                if (File.Exists(temporary)) File.Delete(temporary);
            }
        }

        internal static bool TryLoad(string assetPath, out byte[] bytes, out ReciteAssetInfo info)
        {
            bytes = null;
            info = null;
            try
            {
                var path = PathFor(assetPath);
                if (!File.Exists(path)) return false;
                var candidate = File.ReadAllBytes(path);
                var inspected = ReciteAssetInfo.Inspect(new ReciteDialogueAsset(candidate));
                bytes = candidate;
                info = inspected;
                return true;
            }
            catch (Exception error) when (error is IOException || error is UnauthorizedAccessException ||
                error is ReciteAdapterException || error is DllNotFoundException || error is EntryPointNotFoundException)
            {
                return false;
            }
        }
    }
}
