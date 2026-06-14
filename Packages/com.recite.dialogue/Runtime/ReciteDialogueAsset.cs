using System;

namespace Recite.Unity
{
    public sealed class ReciteDialogueAsset
    {
        public ReciteDialogueAsset(byte[] compiledBytes, string assetId = "", string compatibilityIdentity = "")
        {
            CompiledBytes = compiledBytes != null ? (byte[])compiledBytes.Clone() : throw new ArgumentNullException(nameof(compiledBytes));
            AssetId = assetId ?? string.Empty;
            CompatibilityIdentity = compatibilityIdentity ?? string.Empty;
        }

        public byte[] CompiledBytes { get; }

        public string AssetId { get; }

        public string CompatibilityIdentity { get; }
    }
}
