using System;
using System.IO;
using UnityEditor.AssetImporters;
using UnityEngine;

namespace Recite.Unity.Editor
{
    [ScriptedImporter(1, "recitec")]
    public sealed class ReciteCompiledImporter : ScriptedImporter
    {
        public override void OnImportAsset(AssetImportContext context)
        {
            byte[] bytes;
            ReciteAssetInfo info;
            try
            {
                bytes = File.ReadAllBytes(context.assetPath);
                info = ReciteAssetInfo.Inspect(new ReciteDialogueAsset(bytes));
                ReciteCompiledCache.Save(context.assetPath, bytes);
                Publish(context, bytes, info, false, ReciteStatus.Ok, null);
                return;
            }
            catch (Exception error) when (error is IOException || error is UnauthorizedAccessException ||
                error is ReciteAdapterException || error is DllNotFoundException || error is EntryPointNotFoundException)
            {
                var status = error is ReciteAdapterException adapter
                    ? adapter.Status : ReciteStatus.AssetLoadOrDecode;
                context.LogImportError("Recite compiled asset import failed (" + status + "): " + error.Message);
                if (ReciteCompiledCache.TryLoad(context.assetPath, out bytes, out info))
                    Publish(context, bytes, info, true, status, error.Message);
            }
        }

        private static void Publish(AssetImportContext context, byte[] bytes, ReciteAssetInfo info,
            bool retained, ReciteStatus status, string message)
        {
            var asset = ScriptableObject.CreateInstance<ReciteCompiledAsset>();
            asset.Initialize(bytes, info, retained, status, message);
            context.AddObjectToAsset("dialogue", asset);
            context.SetMainObject(asset);
        }
    }
}
