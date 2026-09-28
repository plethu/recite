using System;
using UnityEngine;

namespace Recite.Unity
{
    // Published only after ScriptedImporter validates the compiled bytes with recite-ffi.
    public sealed class ReciteCompiledAsset : ScriptableObject
    {
        [SerializeField] private byte[] compiledBytes;
        [SerializeField] private string assetId;
        [SerializeField] private string compilerVersion;
        [SerializeField] private string sourceMapId;
        [SerializeField] private int formatVersion;
        [SerializeField] private int compilerCompatibilityVersion;
        [SerializeField] private string contentFingerprintAlgorithm;
        [SerializeField] private byte[] contentFingerprintDigest;
        [SerializeField] private string schemaFingerprintAlgorithm;
        [SerializeField] private byte[] schemaFingerprintDigest;

        [SerializeField] private bool isUsingRetainedRevision;
        [SerializeField] private int importStatus;
        [SerializeField] private string importMessage;

        public bool IsUsingRetainedRevision => isUsingRetainedRevision;
        public ReciteStatus ImportStatus => (ReciteStatus)importStatus;
        public string ImportMessage => importMessage;
        public string AssetId => assetId;
        public string CompilerVersion => compilerVersion;
        public string SourceMapId => sourceMapId;
        public ushort FormatVersion => (ushort)formatVersion;
        public ushort CompilerCompatibilityVersion => (ushort)compilerCompatibilityVersion;
        public string ContentFingerprintAlgorithm => contentFingerprintAlgorithm;
        public byte[] ContentFingerprintDigest => contentFingerprintDigest == null ? null : (byte[])contentFingerprintDigest.Clone();
        public string SchemaFingerprintAlgorithm => schemaFingerprintAlgorithm;
        public byte[] SchemaFingerprintDigest => schemaFingerprintDigest == null ? null : (byte[])schemaFingerprintDigest.Clone();

        public ReciteDialogueAsset ToDialogueAsset()
        {
            if (compiledBytes == null || compiledBytes.Length == 0)
                throw new ReciteAdapterException(ReciteStatus.AssetLoadOrDecode, "compiled Recite bytes are missing");
            return new ReciteDialogueAsset(compiledBytes);
        }

        internal void Initialize(byte[] bytes, ReciteAssetInfo info, bool retained = false,
            ReciteStatus status = ReciteStatus.Ok, string message = null)
        {
            compiledBytes = (byte[])bytes.Clone();
            isUsingRetainedRevision = retained;
            importStatus = (int)status;
            importMessage = message;
            assetId = info.AssetId;
            compilerVersion = info.CompilerVersion;
            sourceMapId = info.SourceMapId;
            formatVersion = info.FormatVersion;
            compilerCompatibilityVersion = info.CompilerCompatibilityVersion;
            contentFingerprintAlgorithm = info.ContentFingerprint.Algorithm;
            contentFingerprintDigest = info.ContentFingerprint.Digest;
            schemaFingerprintAlgorithm = info.SchemaFingerprint?.Algorithm;
            schemaFingerprintDigest = info.SchemaFingerprint?.Digest;
        }
    }
}
