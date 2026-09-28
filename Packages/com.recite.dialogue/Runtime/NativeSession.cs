using System;
using System.Threading;
using Recite.Unity.Native;

namespace Recite.Unity
{
    internal sealed class NativeSession : IDisposable
    {
        private readonly int ownerThread = Thread.CurrentThread.ManagedThreadId;
        private ulong assetHandle;
        private ulong sessionHandle;
        private bool disposed;
        private ReciteAssetInfo loadedInfo;
        private ReciteAssetInfo activeInfo;

        internal ulong Asset { get { CheckThread(); return assetHandle; } }
        internal ulong Session { get { CheckThread(); return sessionHandle; } }
        internal bool IsActive { get { CheckThread(); return sessionHandle != 0; } }
        internal ReciteAssetInfo ActiveInfo { get { CheckThread(); return activeInfo; } }

        internal void Load(ReciteDialogueAsset asset)
        {
            CheckThread();
            if (asset == null) throw new ArgumentNullException(nameof(asset));
            End();
            ReciteNativeBridge.ThrowIfError(ReciteNativeBridge.AssetLoad(
                asset.BytesForNative, new UIntPtr((ulong)asset.BytesForNative.Length), out assetHandle));
            try { loadedInfo = ReciteAssetInfo.FromHandle(assetHandle); }
            catch { End(); throw; }
        }

        internal void Attach(ulong handle)
        {
            CheckThread();
            sessionHandle = handle;
            activeInfo = handle == 0 ? null : loadedInfo;
        }

        internal void RequireActive()
        {
            if (!IsActive) throw new ReciteAdapterException(ReciteStatus.NoActiveSession, "no Recite session is active");
        }

        internal void CheckThread()
        {
            if (Thread.CurrentThread.ManagedThreadId != ownerThread)
                throw new InvalidOperationException("Recite sessions must be used and disposed on their owner thread");
        }

        internal void End()
        {
            CheckThread();
            if (sessionHandle != 0) ReciteNativeBridge.SessionFree(sessionHandle);
            sessionHandle = 0;
            activeInfo = null;
            loadedInfo = null;
            if (assetHandle != 0) ReciteNativeBridge.AssetFree(assetHandle);
            assetHandle = 0;
        }

        public void Dispose()
        {
            CheckThread();
            if (disposed) return;
            End();
            disposed = true;
        }
    }
}
