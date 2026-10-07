using System;
using System.Collections.Generic;
using Recite.Unity.Native;

namespace Recite.Unity
{
    public sealed class ReciteDialogueService : IDisposable
    {
        private readonly NativeSession native = new NativeSession();
        private readonly ConditionCallbacks conditions = new ConditionCallbacks();
        private string localeVariant;
        private IReadOnlyList<ReciteInterpolationValue> interpolationValues = Array.Empty<ReciteInterpolationValue>();
        private NativePoCatalog poCatalog;
        private bool disposed;

        public bool HasActiveSession => native.IsActive;
        public ReciteAssetInfo ActiveAssetInfo => native.ActiveInfo;
        public void SetPoCatalog(IReadOnlyList<RecitePoDocument> documents)
        {
            ThrowIfDisposed();
            if (documents == null)
            {
                if (HasActiveSession)
                    ReciteNativeBridge.ThrowIfError(ReciteNativeBridge.SessionClearLocaleProvider(native.Session));
                poCatalog?.Dispose();
                poCatalog = null;
                return;
            }

            var candidate = new NativePoCatalog(documents);
            try
            {
                if (HasActiveSession)
                    ReciteNativeBridge.ThrowIfError(ReciteNativeBridge.SessionSetCatalog(native.Session, candidate.Handle));
            }
            catch
            {
                candidate.Dispose();
                throw;
            }
            poCatalog?.Dispose();
            poCatalog = candidate;
        }

        public void SetLocaleVariant(string variant)
        {
            ThrowIfDisposed();
            var validated = ReciteStringValidation.Validate(variant, nameof(variant), allowNull: true, allowEmpty: true);
            var next = string.IsNullOrEmpty(validated) ? null : validated;
            if (HasActiveSession)
                ReciteNativeBridge.ThrowIfError(ReciteNativeBridge.SessionSetLocaleVariant(
                    native.Session, ReciteNativeBridge.ToUtf8NullTerminated(next)));
            localeVariant = next;
        }

        public void SetInterpolationValues(IReadOnlyList<ReciteInterpolationValue> values)
        {
            ThrowIfDisposed();
            var copied = CopyInterpolationValues(values);
            if (HasActiveSession)
            {
                using (var nativeValues = new ReciteNativeBridge.InterpolationValueBuffer(copied))
                {
                    ReciteNativeBridge.ThrowIfError(ReciteNativeBridge.SessionSetInterpolationValues(
                        native.Session,
                        nativeValues.Pointer,
                        nativeValues.Length));
                }
            }

            interpolationValues = copied;
        }

        public void RegisterCondition(string name, Func<IReadOnlyList<object>, bool> handler)
        {
            ThrowIfDisposed();
            conditions.RegisterCondition(name, handler);
        }
        public void RegisterConditionValue(string name, Func<IReadOnlyList<object>, ReciteConditionValue> handler)
        {
            ThrowIfDisposed();
            conditions.RegisterConditionValue(name, handler);
        }
        public void RegisterTypedCondition(string name, Func<IReadOnlyList<ReciteConditionArgument>, bool> handler)
        {
            ThrowIfDisposed();
            conditions.RegisterTypedCondition(name, handler);
        }
        public void RegisterTypedConditionValue(string name, Func<IReadOnlyList<ReciteConditionArgument>, ReciteConditionValue> handler)
        {
            ThrowIfDisposed();
            conditions.RegisterTypedConditionValue(name, handler);
        }

        public ReciteOutputBatch Start(ReciteDialogueAsset asset, string startBlock = null, string locale = null, string variant = null)
        {
            ThrowIfDisposed();
            if (HasActiveSession)
            {
                throw new ReciteAdapterException(ReciteStatus.SessionAlreadyActive, "a Recite session is already active");
            }

            if (!string.IsNullOrEmpty(locale))
            {
                ReciteStringValidation.ValidateLocale(locale, nameof(locale));
            }
            var previousVariant = localeVariant;
            var requestedVariant = variant == null ? previousVariant :
                ReciteStringValidation.Validate(variant, nameof(variant), allowEmpty: true);
            SetStoredVariant(string.IsNullOrEmpty(requestedVariant) ? null : requestedVariant);
            try
            {
                native.Load(asset);
                ReciteNativeBridge.ThrowIfError(ReciteNativeBridge.SessionCreate(native.Asset, ReciteNativeBridge.ToUtf8NullTerminated(startBlock), ReciteNativeBridge.ToUtf8NullTerminated(locale), out var createdSession));
                native.Attach(createdSession);
                return ConfigureAndBegin();
            }
            catch
            {
                End();
                SetStoredVariant(previousVariant);
                throw;
            }
        }

        public ReciteOutputBatch SelectChoice(string choiceId)
        {
            ThrowIfDisposed();
            native.RequireActive();
            ReciteStringValidation.Validate(choiceId, nameof(choiceId));
            ReciteNativeBridge.ReciteBuffer batch;
            try
            {
                ReciteNativeBridge.ThrowIfError(ReciteNativeBridge.SessionChoose(native.Session, ReciteNativeBridge.ToUtf8NullTerminated(choiceId), out batch));
            }
            finally
            {
                conditions.ReleaseBorrowedResults();
            }
            return OutputDecoder.DecodeBatch(ref batch);
        }

        public ReciteOutputBatch AcknowledgeEffect(string effectRequestId, bool completed = true, string failureReason = null)
        {
            ThrowIfDisposed();
            native.RequireActive();
            ReciteStringValidation.Validate(effectRequestId, nameof(effectRequestId));
            ReciteStringValidation.Validate(failureReason, nameof(failureReason), allowNull: true, allowEmpty: true);
            ReciteNativeBridge.ReciteBuffer batch;
            try
            {
                ReciteNativeBridge.ThrowIfError(ReciteNativeBridge.SessionAcknowledgeEffect(
                    native.Session,
                    ReciteNativeBridge.ToUtf8NullTerminated(effectRequestId),
                    completed ? (byte)1 : (byte)0,
                    ReciteNativeBridge.ToUtf8NullTerminated(failureReason),
                    out batch));
            }
            finally
            {
                conditions.ReleaseBorrowedResults();
            }
            return OutputDecoder.DecodeBatch(ref batch);
        }

        public ReciteSessionSnapshot Snapshot()
        {
            ThrowIfDisposed();
            native.RequireActive();
            ReciteNativeBridge.ThrowIfError(ReciteNativeBridge.SessionSnapshot(native.Session, out var buffer));
            return new ReciteSessionSnapshot(ReciteNativeBridge.CopyAndFree(ref buffer));
        }

        public ReciteOutputBatch Restore(ReciteDialogueAsset asset, ReciteSessionSnapshot snapshot, string variant = null)
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

            var previousVariant = localeVariant;
            var requestedVariant = variant == null ? previousVariant :
                ReciteStringValidation.Validate(variant, nameof(variant), allowEmpty: true);
            SetStoredVariant(string.IsNullOrEmpty(requestedVariant) ? null : requestedVariant);
            try
            {
                native.Load(asset);
                ReciteNativeBridge.ThrowIfError(ReciteNativeBridge.SessionPrepareRestore(
                    native.Asset, snapshot.Bytes, new UIntPtr((ulong)snapshot.Bytes.Length), out var restoredSession));
                native.Attach(restoredSession);
                return ConfigureAndBegin();
            }
            catch
            {
                End();
                SetStoredVariant(previousVariant);
                throw;
            }
        }

        private ReciteOutputBatch ConfigureAndBegin()
        {
            SetNativeInterpolationValues();
            if (localeVariant != null)
            {
                ReciteNativeBridge.ThrowIfError(ReciteNativeBridge.SessionSetLocaleVariant(
                    native.Session, ReciteNativeBridge.ToUtf8NullTerminated(localeVariant)));
            }
            conditions.Register(native.Session);
            if (poCatalog != null)
                ReciteNativeBridge.ThrowIfError(ReciteNativeBridge.SessionSetCatalog(native.Session, poCatalog.Handle));
            ReciteNativeBridge.ReciteBuffer batch;
            try
            {
                ReciteNativeBridge.ThrowIfError(ReciteNativeBridge.SessionBegin(native.Session, out batch));
            }
            finally
            {
                conditions.ReleaseBorrowedResults();
            }
            return OutputDecoder.DecodeBatch(ref batch);
        }

        public void End()
        {
            native.End();
            conditions.ReleaseBorrowedResults();
        }

        public void Dispose()
        {
            native.CheckThread();
            if (disposed) return;
            native.Dispose();
            conditions.Dispose();
            poCatalog?.Dispose();
            disposed = true;
        }

        private void SetNativeInterpolationValues()
        {
            using (var nativeValues = new ReciteNativeBridge.InterpolationValueBuffer(interpolationValues))
            {
                ReciteNativeBridge.ThrowIfError(ReciteNativeBridge.SessionSetInterpolationValues(
                    native.Session,
                    nativeValues.Pointer,
                    nativeValues.Length));
            }
        }

        private static IReadOnlyList<ReciteInterpolationValue> CopyInterpolationValues(
            IReadOnlyList<ReciteInterpolationValue> values)
        {
            if (values == null)
            {
                throw new ArgumentNullException(nameof(values));
            }

            var copied = new List<ReciteInterpolationValue>(values.Count);
            var names = new HashSet<string>(StringComparer.Ordinal);
            foreach (var value in values)
            {
                if (value == null)
                {
                    throw new ArgumentException("interpolation values cannot contain null entries", nameof(values));
                }
                if (!names.Add(value.Name))
                {
                    throw new ArgumentException("interpolation value names must be unique", nameof(values));
                }
                copied.Add(value);
            }
            return copied;
        }

        internal static ReciteOutputBatch DecodeBatchBytes(byte[] bytes) => OutputDecoder.DecodeBatchBytes(bytes);

        internal ReciteNativeBridge.ReciteConditionResult EvaluateCondition(IntPtr queryPtr, IntPtr userdata) => conditions.EvaluateCondition(queryPtr, userdata);
        private void SetStoredVariant(string value) { localeVariant = value; }

        private void ThrowIfDisposed()
        {
            native.CheckThread();
            if (disposed) throw new ObjectDisposedException(nameof(ReciteDialogueService));
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
