using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using Recite.Unity.Native;

namespace Recite.Unity
{
    public sealed class ReciteDialogueService : IDisposable
    {
        private readonly Dictionary<string, Func<IReadOnlyList<object>, ReciteConditionValue>> conditions = new Dictionary<string, Func<IReadOnlyList<object>, ReciteConditionValue>>(StringComparer.Ordinal);
        private readonly Dictionary<string, Func<IReadOnlyList<ReciteConditionArgument>, ReciteConditionValue>> typedConditions = new Dictionary<string, Func<IReadOnlyList<ReciteConditionArgument>, ReciteConditionValue>>(StringComparer.Ordinal);
        private readonly ReciteNativeBridge.ReciteConditionFn conditionCallback;
        private GCHandle pinnedConditionValue;
        private GCHandle pinnedConditionError;
        private ulong assetHandle;
        private ulong sessionHandle;
        private bool disposed;

        public ReciteDialogueService()
        {
            conditionCallback = EvaluateCondition;
        }

        public bool HasActiveSession => sessionHandle != 0;

        public void RegisterCondition(string name, Func<IReadOnlyList<object>, bool> handler)
        {
            if (handler == null)
            {
                throw new ArgumentNullException(nameof(handler));
            }

            RegisterConditionValue(name, args => ReciteConditionValue.Bool(handler(args)));
        }

        public void RegisterConditionValue(string name, Func<IReadOnlyList<object>, ReciteConditionValue> handler)
        {
            if (string.IsNullOrWhiteSpace(name))
            {
                throw new ArgumentException("condition name is required", nameof(name));
            }

            conditions[name] = handler ?? throw new ArgumentNullException(nameof(handler));
            typedConditions.Remove(name);
        }

        public void RegisterTypedCondition(string name, Func<IReadOnlyList<ReciteConditionArgument>, bool> handler)
        {
            if (handler == null)
            {
                throw new ArgumentNullException(nameof(handler));
            }

            RegisterTypedConditionValue(name, args => ReciteConditionValue.Bool(handler(args)));
        }

        public void RegisterTypedConditionValue(string name, Func<IReadOnlyList<ReciteConditionArgument>, ReciteConditionValue> handler)
        {
            if (string.IsNullOrWhiteSpace(name))
            {
                throw new ArgumentException("condition name is required", nameof(name));
            }

            typedConditions[name] = handler ?? throw new ArgumentNullException(nameof(handler));
            conditions.Remove(name);
        }

        public ReciteOutputBatch Start(ReciteDialogueAsset asset, string startBlock = null, string locale = null)
        {
            ThrowIfDisposed();
            if (HasActiveSession)
            {
                throw new ReciteAdapterException(ReciteStatus.SessionAlreadyActive, "a Recite session is already active");
            }

            LoadAsset(asset);
            ReciteNativeBridge.ThrowIfError(ReciteNativeBridge.SessionCreate(assetHandle, ReciteNativeBridge.ToUtf8NullTerminated(startBlock), ReciteNativeBridge.ToUtf8NullTerminated(locale), out sessionHandle));
            try
            {
                RegisterNativeConditions();
                ReciteNativeBridge.ThrowIfError(ReciteNativeBridge.SessionBegin(sessionHandle, out var batch));
                return DecodeBatch(ref batch);
            }
            catch
            {
                End();
                throw;
            }
        }

        public ReciteOutputBatch SelectChoice(string choiceId)
        {
            ThrowIfDisposed();
            EnsureActive();
            ReciteNativeBridge.ThrowIfError(ReciteNativeBridge.SessionChoose(sessionHandle, ReciteNativeBridge.ToUtf8NullTerminated(choiceId), out var batch));
            return DecodeBatch(ref batch);
        }

        public ReciteOutputBatch AcknowledgeEffect(string effectRequestId, bool completed = true, string failureReason = null)
        {
            ThrowIfDisposed();
            EnsureActive();
            ReciteNativeBridge.ThrowIfError(ReciteNativeBridge.SessionAcknowledgeEffect(
                sessionHandle,
                ReciteNativeBridge.ToUtf8NullTerminated(effectRequestId),
                completed ? (byte)1 : (byte)0,
                ReciteNativeBridge.ToUtf8NullTerminated(failureReason),
                out var batch));
            return DecodeBatch(ref batch);
        }

        public ReciteSessionSnapshot Snapshot()
        {
            ThrowIfDisposed();
            EnsureActive();
            ReciteNativeBridge.ThrowIfError(ReciteNativeBridge.SessionSnapshot(sessionHandle, out var buffer));
            return new ReciteSessionSnapshot(ReciteNativeBridge.CopyAndFree(ref buffer));
        }

        public ReciteOutputBatch Restore(ReciteDialogueAsset asset, ReciteSessionSnapshot snapshot)
        {
            ThrowIfDisposed();
            if (snapshot == null)
            {
                throw new ArgumentNullException(nameof(snapshot));
            }

            if (HasActiveSession)
            {
                throw new ReciteAdapterException(ReciteStatus.SessionAlreadyActive, "a Recite session is already active");
            }

            LoadAsset(asset);
            ReciteNativeBridge.ThrowIfError(ReciteNativeBridge.SessionRestore(assetHandle, snapshot.Bytes, new UIntPtr((ulong)snapshot.Bytes.Length), out sessionHandle, out var batch));
            try
            {
                RegisterNativeConditions();
                return DecodeBatch(ref batch);
            }
            catch
            {
                ReciteNativeBridge.BufferFree(ref batch);
                End();
                throw;
            }
        }

        public void End()
        {
            if (sessionHandle != 0)
            {
                ReciteNativeBridge.SessionFree(sessionHandle);
                sessionHandle = 0;
            }

            FreeAsset();
        }

        public void Dispose()
        {
            if (disposed)
            {
                return;
            }

            End();
            if (pinnedConditionValue.IsAllocated)
            {
                pinnedConditionValue.Free();
            }

            if (pinnedConditionError.IsAllocated)
            {
                pinnedConditionError.Free();
            }

            disposed = true;
        }

        private void LoadAsset(ReciteDialogueAsset asset)
        {
            if (asset == null)
            {
                throw new ArgumentNullException(nameof(asset));
            }

            FreeAsset();
            ReciteNativeBridge.ThrowIfError(ReciteNativeBridge.AssetLoad(asset.CompiledBytes, new UIntPtr((ulong)asset.CompiledBytes.Length), out assetHandle));
        }

        private void FreeAsset()
        {
            if (assetHandle != 0)
            {
                ReciteNativeBridge.AssetFree(assetHandle);
                assetHandle = 0;
            }
        }

        private void RegisterNativeConditions()
        {
            var names = new HashSet<string>(conditions.Keys, StringComparer.Ordinal);
            names.UnionWith(typedConditions.Keys);
            var registeredNames = new List<string>(names);
            registeredNames.Sort(StringComparer.Ordinal);
            foreach (var name in registeredNames)
            {
                ReciteNativeBridge.ThrowIfError(ReciteNativeBridge.SessionRegisterCondition(sessionHandle, ReciteNativeBridge.ToUtf8NullTerminated(name), conditionCallback, IntPtr.Zero));
            }
        }

        private static ReciteOutputBatch DecodeBatch(ref ReciteNativeBridge.ReciteBuffer batch)
        {
            return DecodeBatchBytes(ReciteNativeBridge.CopyAndFree(ref batch));
        }

        internal static ReciteOutputBatch DecodeBatchBytes(byte[] bytes)
        {
            try
            {
                return ReciteMessagePack.DecodeOutputBatch(bytes);
            }
            catch (FormatException error)
            {
                throw new ReciteAdapterException(ReciteStatus.Validation, error.Message);
            }
            catch (OverflowException error)
            {
                throw new ReciteAdapterException(ReciteStatus.Validation, error.Message);
            }
            catch (InvalidCastException error)
            {
                throw new ReciteAdapterException(ReciteStatus.Validation, error.Message);
            }
            catch (System.Collections.Generic.KeyNotFoundException error)
            {
                throw new ReciteAdapterException(ReciteStatus.Validation, error.Message);
            }
            catch (ArgumentException error)
            {
                throw new ReciteAdapterException(ReciteStatus.Validation, error.Message);
            }
        }

        // Internal so the managed headless fixture can exercise the same callback
        // boundary used by the native bridge without requiring a Unity binary.
        internal ReciteNativeBridge.ReciteConditionResult EvaluateCondition(IntPtr queryPtr, IntPtr userdata)
        {
            if (queryPtr == IntPtr.Zero)
            {
                return ConditionFailure("condition query pointer was null");
            }

            var query = Marshal.PtrToStructure<ReciteNativeBridge.ReciteConditionQuery>(queryPtr);
            var name = Marshal.PtrToStringUTF8(query.FunctionName) ?? string.Empty;
            conditions.TryGetValue(name, out var handler);
            typedConditions.TryGetValue(name, out var typedHandler);
            if (handler == null && typedHandler == null)
            {
                return ConditionFailure("no Unity condition handler registered for `" + name + "`");
            }

            try
            {
                var value = typedHandler != null
                    ? typedHandler(ReciteNativeBridge.ReadTypedConditionArgs(query.ArgsMsgpack, query.ArgsLen))
                    : handler(ReciteNativeBridge.ReadConditionArgs(query.ArgsMsgpack, query.ArgsLen));
                var encoded = value != null && value.IsEnum
                    ? ReciteMessagePack.EncodeConditionEnum(value.EnumVariant)
                    : ReciteMessagePack.EncodeConditionBool(value != null && value.BoolValue);
                return ConditionSuccess(encoded);
            }
            catch (Exception ex)
            {
                return ConditionFailure(ex.Message);
            }
        }

        private ReciteNativeBridge.ReciteConditionResult ConditionSuccess(byte[] bytes)
        {
            if (pinnedConditionValue.IsAllocated)
            {
                pinnedConditionValue.Free();
            }

            pinnedConditionValue = GCHandle.Alloc(bytes, GCHandleType.Pinned);
            return new ReciteNativeBridge.ReciteConditionResult
            {
                Ok = 1,
                ValueMsgpack = pinnedConditionValue.AddrOfPinnedObject(),
                ValueLen = new UIntPtr((ulong)bytes.Length),
                ErrorMessage = IntPtr.Zero
            };
        }

        private ReciteNativeBridge.ReciteConditionResult ConditionFailure(string message)
        {
            var bytes = ReciteNativeBridge.ToUtf8NullTerminated(message ?? "Unity condition handler failed");
            if (pinnedConditionError.IsAllocated)
            {
                pinnedConditionError.Free();
            }

            pinnedConditionError = GCHandle.Alloc(bytes, GCHandleType.Pinned);
            return new ReciteNativeBridge.ReciteConditionResult
            {
                Ok = 0,
                ValueMsgpack = IntPtr.Zero,
                ValueLen = UIntPtr.Zero,
                ErrorMessage = pinnedConditionError.AddrOfPinnedObject()
            };
        }

        private void EnsureActive()
        {
            if (!HasActiveSession)
            {
                throw new ReciteAdapterException(ReciteStatus.NoActiveSession, "no Recite session is active");
            }
        }

        private void ThrowIfDisposed()
        {
            if (disposed)
            {
                throw new ObjectDisposedException(nameof(ReciteDialogueService));
            }
        }
    }

    public sealed class ReciteConditionValue
    {
        private ReciteConditionValue(bool boolValue, string enumVariant, bool isEnum)
        {
            BoolValue = boolValue;
            EnumVariant = enumVariant;
            IsEnum = isEnum;
        }

        public bool BoolValue { get; }

        public string EnumVariant { get; }

        public bool IsEnum { get; }

        public static ReciteConditionValue Bool(bool value)
        {
            return new ReciteConditionValue(value, null, false);
        }

        public static ReciteConditionValue Enum(string variant)
        {
            return new ReciteConditionValue(false, variant ?? string.Empty, true);
        }
    }
}
