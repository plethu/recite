using System;
using System.Collections.Generic;
using System.Threading;
using Recite.Unity.Native;

namespace Recite.Unity
{
    // A candidate is built completely before it replaces the service's current
    // native catalogue. Attached sessions retain their own immutable revision.
    internal sealed class NativePoCatalog : IDisposable
    {
        private readonly int ownerThread = Thread.CurrentThread.ManagedThreadId;
        private ulong handle;

        internal NativePoCatalog(IReadOnlyList<RecitePoDocument> documents)
        {
            if (documents == null) throw new ArgumentNullException(nameof(documents));
            ReciteNativeBridge.ThrowIfError(ReciteNativeBridge.CatalogCreate(out handle));
            try
            {
                foreach (var document in documents)
                {
                    if (document == null) throw new ArgumentException("PO documents cannot contain null", nameof(documents));
                    var bytes = document.BytesForNative;
                    ReciteNativeBridge.ThrowIfError(ReciteNativeBridge.CatalogAddPo(handle,
                        ReciteNativeBridge.ToUtf8NullTerminated(document.Locale), bytes,
                        new UIntPtr((ulong)bytes.Length)));
                }
            }
            catch
            {
                Dispose();
                throw;
            }
        }

        internal ulong Handle
        {
            get
            {
                CheckThread();
                if (handle == 0) throw new ObjectDisposedException(nameof(NativePoCatalog));
                return handle;
            }
        }

        private void CheckThread()
        {
            if (Thread.CurrentThread.ManagedThreadId != ownerThread)
                throw new InvalidOperationException("Recite PO catalogues must be used on their owner thread");
        }

        public void Dispose()
        {
            CheckThread();
            if (handle == 0) return;
            ReciteNativeBridge.CatalogFree(handle);
            handle = 0;
        }
    }
}
