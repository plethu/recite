using System;

namespace Recite.Unity
{
    // Writer-owned PO bytes and the locale to which they belong.
    public sealed class RecitePoDocument
    {
        private readonly byte[] bytes;

        public RecitePoDocument(string locale, byte[] bytes)
        {
            Locale = ReciteStringValidation.ValidateLocale(locale, nameof(locale));
            this.bytes = bytes != null ? (byte[])bytes.Clone() : throw new ArgumentNullException(nameof(bytes));
        }

        public string Locale { get; }
        public byte[] Bytes => (byte[])bytes.Clone();
        internal byte[] BytesForNative => bytes;
    }
}
