using System;
using System.Collections.Generic;
using Recite.Unity.Native;

namespace Recite.Unity
{
    public sealed class ReciteFingerprint
    {
        private readonly byte[] digest;

        internal ReciteFingerprint(string algorithm, byte[] digest)
        {
            Algorithm = algorithm;
            this.digest = (byte[])digest.Clone();
        }

        public string Algorithm { get; }
        public byte[] Digest => (byte[])digest.Clone();
    }

    // Metadata is projected from a compiled asset accepted by the native decoder.
    // Session compatibility is still decided by native start/restore, not this view.
    public sealed class ReciteAssetInfo
    {
        private ReciteAssetInfo(string assetId, ReciteFingerprint contentFingerprint,
            ReciteFingerprint schemaFingerprint, ushort formatVersion,
            ushort compilerCompatibilityVersion, string compilerVersion, string sourceMapId)
        {
            AssetId = assetId;
            ContentFingerprint = contentFingerprint;
            SchemaFingerprint = schemaFingerprint;
            FormatVersion = formatVersion;
            CompilerCompatibilityVersion = compilerCompatibilityVersion;
            CompilerVersion = compilerVersion;
            SourceMapId = sourceMapId;
        }

        public string AssetId { get; }
        public ReciteFingerprint ContentFingerprint { get; }
        public ReciteFingerprint SchemaFingerprint { get; }
        public ushort FormatVersion { get; }
        public ushort CompilerCompatibilityVersion { get; }
        public string CompilerVersion { get; }
        public string SourceMapId { get; }

        public static ReciteAssetInfo Inspect(ReciteDialogueAsset asset)
        {
            if (asset == null) throw new ArgumentNullException(nameof(asset));
            var bytes = asset.BytesForNative;
            ReciteNativeBridge.ThrowIfError(ReciteNativeBridge.AssetLoad(bytes,
                new UIntPtr((ulong)bytes.Length), out var handle));
            try
            {
                return FromHandle(handle);
            }
            finally
            {
                ReciteNativeBridge.AssetFree(handle);
            }
        }

        internal static ReciteAssetInfo FromHandle(ulong handle)
        {
            ReciteNativeBridge.ThrowIfError(ReciteNativeBridge.AssetInfo(handle, out var buffer));
            return Decode(ReciteNativeBridge.CopyAndFree(ref buffer));
        }

        internal static ReciteAssetInfo Decode(byte[] bytes)
        {
            try
            {
                var reader = new ReciteMessagePackReader(bytes);
                var map = reader.ReadMap();
                reader.EnsureEnd();
                if (map.Count != 8 || Number(map, "asset_info_format_version") != 0)
                    throw new FormatException("unsupported Recite asset info format");
                var schema = map.TryGetValue("schema_fingerprint", out var rawSchema) && rawSchema != null
                    ? Fingerprint(rawSchema, "schema_fingerprint") : null;
                if (!map.ContainsKey("schema_fingerprint"))
                    throw new FormatException("missing schema_fingerprint");
                return new ReciteAssetInfo(
                    String(map, "asset_id"), Fingerprint(Value(map, "content_fingerprint"), "content_fingerprint"),
                    schema, Number(map, "format_version"), Number(map, "compiler_compatibility_version"),
                    String(map, "compiler_version"), String(map, "source_map_id"));
            }
            catch (Exception error) when (error is FormatException || error is OverflowException || error is InvalidCastException)
            {
                throw new ReciteAdapterException(ReciteStatus.Validation, error.Message);
            }
        }

        private static object Value(IReadOnlyDictionary<string, object> map, string key)
        {
            if (!map.TryGetValue(key, out var value)) throw new FormatException("missing " + key);
            return value;
        }

        private static string String(IReadOnlyDictionary<string, object> map, string key)
        {
            if (!(Value(map, key) is string value)) throw new FormatException("invalid " + key);
            return value;
        }

        private static ushort Number(IReadOnlyDictionary<string, object> map, string key)
        {
            if (!(Value(map, key) is long value) || value < 0 || value > ushort.MaxValue)
                throw new FormatException("invalid " + key);
            return (ushort)value;
        }

        private static ReciteFingerprint Fingerprint(object raw, string key)
        {
            if (!(raw is IReadOnlyDictionary<string, object> map) || map.Count != 2 ||
                !map.TryGetValue("algorithm", out var algorithm) || !(algorithm is string name) ||
                !map.TryGetValue("digest", out var digest) || !(digest is byte[] bytes))
                throw new FormatException("invalid " + key);
            return new ReciteFingerprint(name, bytes);
        }
    }
}
