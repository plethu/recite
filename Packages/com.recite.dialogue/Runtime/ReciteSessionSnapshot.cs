using System;

namespace Recite.Unity
{
    public sealed class ReciteSessionSnapshot
    {
        private readonly byte[] bytes;

        public ReciteSessionSnapshot(byte[] bytes)
        {
            this.bytes = bytes != null ? (byte[])bytes.Clone() : throw new ArgumentNullException(nameof(bytes));
        }

        public byte[] Bytes => (byte[])bytes.Clone();
    }
}
