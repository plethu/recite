using System;

namespace Recite.Unity
{
    // A revision of compiled bytes. Public callers cannot mutate a loaded revision.
    public sealed class ReciteDialogueAsset
    {
        private readonly byte[] compiledBytes;

        public ReciteDialogueAsset(byte[] compiledBytes)
        {
            this.compiledBytes = compiledBytes != null
                ? (byte[])compiledBytes.Clone()
                : throw new ArgumentNullException(nameof(compiledBytes));
        }

        public byte[] CompiledBytes => (byte[])compiledBytes.Clone();

        internal byte[] BytesForNative => compiledBytes;
    }
}
