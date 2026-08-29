using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
#if UNITY_2022_3_OR_NEWER
using AOT;
#endif
using Recite.Unity.Native;

namespace Recite.Unity
{
    public sealed class ReciteDialogueService : IDisposable
    {
        private readonly Dictionary<string, Func<IReadOnlyList<object>, ReciteConditionValue>> conditions = new Dictionary<string, Func<IReadOnlyList<object>, ReciteConditionValue>>(StringComparer.Ordinal);
        private readonly Dictionary<string, Func<IReadOnlyList<ReciteConditionArgument>, ReciteConditionValue>> typedConditions = new Dictionary<string, Func<IReadOnlyList<ReciteConditionArgument>, ReciteConditionValue>>(StringComparer.Ordinal);
        private static readonly ReciteNativeBridge.ReciteConditionFn conditionCallback = ConditionCallbackEntry;
        private static readonly ReciteNativeBridge.ReciteLocaleFn localeCallback = LocaleCallbackEntry;
        private IReadOnlyList<ReciteInterpolationValue> interpolationValues = Array.Empty<ReciteInterpolationValue>();
        private ReciteLocaleCatalog localeCatalog;
        private string localeVariant;
        // Locale callback result trees must survive callback return until the
        // enclosing native traversal call returns. Each call frees this owner
        // list in a finally block; End/Dispose is the rollback safety net.
        private readonly List<IntPtr> localeCallbackAllocations = new List<IntPtr>();
        private GCHandle pinnedConditionValue;
        private GCHandle pinnedConditionError;
        private ulong assetHandle;
        private ulong sessionHandle;
        private readonly GCHandle contextHandle;
        private readonly IntPtr contextPointer;
        private bool disposed;

        public ReciteDialogueService()
        {
            contextHandle = GCHandle.Alloc(this, GCHandleType.Normal);
            contextPointer = GCHandle.ToIntPtr(contextHandle);
        }

        public bool HasActiveSession => sessionHandle != 0;

        internal int LocaleCallbackAllocationCount => localeCallbackAllocations.Count;

        public void SetLocaleCatalog(ReciteLocaleCatalog catalog)
        {
            ThrowIfDisposed();
            var copied = catalog != null ? catalog.Clone() : null;
            copied?.ValidatePluralRules((locale, armCount, header) =>
            {
                var actual = ReciteNativeBridge.ValidatePluralRule(header);
                if (actual != armCount)
                {
                    throw new ReciteAdapterException(
                        ReciteStatus.Localisation,
                        "plural rule arm count does not match the catalogue entry");
                }
            });
            var previous = localeCatalog;
            localeCatalog = copied;
            try
            {
                if (HasActiveSession && copied != null)
                {
                    ReciteNativeBridge.ThrowIfError(ReciteNativeBridge.SessionSetLocaleProvider(
                        sessionHandle, localeCallback, contextPointer));
                }
                else if (HasActiveSession)
                {
                    ReciteNativeBridge.ThrowIfError(ReciteNativeBridge.SessionClearLocaleProvider(sessionHandle));
                }
            }
            catch
            {
                localeCatalog = previous;
                throw;
            }
        }

        public void SetLocaleVariant(string variant)
        {
            ThrowIfDisposed();
            var validated = ReciteStringValidation.Validate(
                variant, nameof(variant), allowNull: true, allowEmpty: true);
            var previous = localeVariant;
            localeVariant = string.IsNullOrEmpty(validated) ? null : validated;
            try
            {
                if (HasActiveSession)
                {
                    ReciteNativeBridge.ThrowIfError(ReciteNativeBridge.SessionSetLocaleVariant(
                        sessionHandle, ReciteNativeBridge.ToUtf8NullTerminated(localeVariant)));
                }
            }
            catch
            {
                localeVariant = previous;
                throw;
            }
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
                        sessionHandle,
                        nativeValues.Pointer,
                        nativeValues.Length));
                }
            }

            interpolationValues = copied;
        }

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
            ReciteStringValidation.Validate(name, nameof(name));
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
            ReciteStringValidation.Validate(name, nameof(name));
            if (string.IsNullOrWhiteSpace(name))
            {
                throw new ArgumentException("condition name is required", nameof(name));
            }

            typedConditions[name] = handler ?? throw new ArgumentNullException(nameof(handler));
            conditions.Remove(name);
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
            var requestedVariant = ReciteStringValidation.Validate(variant, nameof(variant), allowNull: true, allowEmpty: true);
            var previousVariant = localeVariant;
            localeVariant = string.IsNullOrEmpty(requestedVariant) ? null : requestedVariant;
            try
            {
                LoadAsset(asset);
                ReciteNativeBridge.ThrowIfError(ReciteNativeBridge.SessionCreate(assetHandle, ReciteNativeBridge.ToUtf8NullTerminated(startBlock), ReciteNativeBridge.ToUtf8NullTerminated(locale), out sessionHandle));
                SetNativeInterpolationValues();
                if (localeVariant != null)
                {
                    ReciteNativeBridge.ThrowIfError(ReciteNativeBridge.SessionSetLocaleVariant(
                        sessionHandle, ReciteNativeBridge.ToUtf8NullTerminated(localeVariant)));
                }
                RegisterNativeConditions();
                RegisterNativeLocaleProvider();
                ReciteNativeBridge.ReciteBuffer batch = default;
                try
                {
                    ReciteNativeBridge.ThrowIfError(ReciteNativeBridge.SessionBegin(sessionHandle, out batch));
                }
                finally
                {
                    FreeLocaleCallbackAllocations();
                }
                return DecodeBatch(ref batch);
            }
            catch
            {
                End();
                localeVariant = previousVariant;
                throw;
            }
        }

        public ReciteOutputBatch SelectChoice(string choiceId)
        {
            ThrowIfDisposed();
            EnsureActive();
            ReciteStringValidation.Validate(choiceId, nameof(choiceId));
            ReciteNativeBridge.ReciteBuffer batch = default;
            try
            {
                ReciteNativeBridge.ThrowIfError(ReciteNativeBridge.SessionChoose(sessionHandle, ReciteNativeBridge.ToUtf8NullTerminated(choiceId), out batch));
            }
            finally
            {
                FreeLocaleCallbackAllocations();
            }
            return DecodeBatch(ref batch);
        }

        public ReciteOutputBatch AcknowledgeEffect(string effectRequestId, bool completed = true, string failureReason = null)
        {
            ThrowIfDisposed();
            EnsureActive();
            ReciteStringValidation.Validate(effectRequestId, nameof(effectRequestId));
            ReciteStringValidation.Validate(failureReason, nameof(failureReason), allowNull: true, allowEmpty: true);
            ReciteNativeBridge.ReciteBuffer batch = default;
            try
            {
                ReciteNativeBridge.ThrowIfError(ReciteNativeBridge.SessionAcknowledgeEffect(
                    sessionHandle,
                    ReciteNativeBridge.ToUtf8NullTerminated(effectRequestId),
                    completed ? (byte)1 : (byte)0,
                    ReciteNativeBridge.ToUtf8NullTerminated(failureReason),
                    out batch));
            }
            finally
            {
                FreeLocaleCallbackAllocations();
            }
            return DecodeBatch(ref batch);
        }

        public ReciteSessionSnapshot Snapshot()
        {
            ThrowIfDisposed();
            EnsureActive();
            ReciteNativeBridge.ThrowIfError(ReciteNativeBridge.SessionSnapshot(sessionHandle, out var buffer));
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

            var requestedVariant = ReciteStringValidation.Validate(variant, nameof(variant), allowNull: true, allowEmpty: true);
            var previousVariant = localeVariant;
            localeVariant = string.IsNullOrEmpty(requestedVariant) ? null : requestedVariant;
            try
            {
                LoadAsset(asset);
                using (var nativeValues = new ReciteNativeBridge.InterpolationValueBuffer(interpolationValues))
                {
                    ReciteNativeBridge.ReciteBuffer batch;
                    try
                    {
                        if (localeCatalog != null)
                        {
                            if (localeVariant != null)
                            {
                                ReciteNativeBridge.ThrowIfError(ReciteNativeBridge.SessionRestoreWithValuesAndLocaleProviderAndVariant(
                                    assetHandle,
                                    snapshot.Bytes,
                                    new UIntPtr((ulong)snapshot.Bytes.Length),
                                    nativeValues.Pointer,
                                    nativeValues.Length,
                                    ReciteNativeBridge.ToUtf8NullTerminated(localeVariant),
                                    localeCallback,
                                    contextPointer,
                                    out sessionHandle,
                                    out batch));
                            }
                            else
                            {
                                ReciteNativeBridge.ThrowIfError(ReciteNativeBridge.SessionRestoreWithValuesAndLocaleProvider(
                                    assetHandle,
                                    snapshot.Bytes,
                                    new UIntPtr((ulong)snapshot.Bytes.Length),
                                    nativeValues.Pointer,
                                    nativeValues.Length,
                                    localeCallback,
                                    contextPointer,
                                    out sessionHandle,
                                    out batch));
                            }
                        }
                        else
                        {
                            ReciteNativeBridge.ThrowIfError(ReciteNativeBridge.SessionRestoreWithValues(
                                assetHandle,
                                snapshot.Bytes,
                                new UIntPtr((ulong)snapshot.Bytes.Length),
                                nativeValues.Pointer,
                                nativeValues.Length,
                                out sessionHandle,
                                out batch));
                            if (localeVariant != null)
                            {
                                ReciteNativeBridge.ThrowIfError(ReciteNativeBridge.SessionSetLocaleVariant(
                                    sessionHandle, ReciteNativeBridge.ToUtf8NullTerminated(localeVariant)));
                            }
                        }
                    }
                    finally
                    {
                        FreeLocaleCallbackAllocations();
                    }
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
            }
            catch
            {
                End();
                localeVariant = previousVariant;
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
            FreeLocaleCallbackAllocations();
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

            if (contextHandle.IsAllocated)
            {
                contextHandle.Free();
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
                ReciteStringValidation.Validate(name, nameof(name));
                ReciteNativeBridge.ThrowIfError(ReciteNativeBridge.SessionRegisterCondition(sessionHandle, ReciteNativeBridge.ToUtf8NullTerminated(name), conditionCallback, contextPointer));
            }
        }

        private void RegisterNativeLocaleProvider()
        {
            if (localeCatalog != null)
            {
                ReciteNativeBridge.ThrowIfError(ReciteNativeBridge.SessionSetLocaleProvider(
                    sessionHandle, localeCallback, contextPointer));
            }
        }

        private void SetNativeInterpolationValues()
        {
            using (var nativeValues = new ReciteNativeBridge.InterpolationValueBuffer(interpolationValues))
            {
                ReciteNativeBridge.ThrowIfError(ReciteNativeBridge.SessionSetInterpolationValues(
                    sessionHandle,
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
            return EvaluateConditionCore(queryPtr);
        }

        #if UNITY_2022_3_OR_NEWER
        [MonoPInvokeCallback(typeof(ReciteNativeBridge.ReciteConditionFn))]
        #endif
        private static ReciteNativeBridge.ReciteConditionResult ConditionCallbackEntry(IntPtr queryPtr, IntPtr userdata)
        {
            ReciteDialogueService service = null;
            try
            {
                service = ServiceFromContext(userdata);
                return service == null
                    ? InvalidConditionCallbackResult()
                    : service.EvaluateConditionCore(queryPtr);
            }
            catch (Exception error)
            {
                return service?.ConditionFailure(error.Message) ?? InvalidConditionCallbackResult();
            }
        }

        private ReciteNativeBridge.ReciteConditionResult EvaluateConditionCore(IntPtr queryPtr)
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

        // Internal so the managed headless fixture can exercise the same
        // callback boundary used by the native bridge without a native binary.
        internal ReciteNativeBridge.ReciteLocaleResult EvaluateLocale(IntPtr queryPtr, IntPtr userdata)
        {
            return EvaluateLocaleCore(queryPtr);
        }

        #if UNITY_2022_3_OR_NEWER
        [MonoPInvokeCallback(typeof(ReciteNativeBridge.ReciteLocaleFn))]
        #endif
        private static ReciteNativeBridge.ReciteLocaleResult LocaleCallbackEntry(IntPtr queryPtr, IntPtr userdata)
        {
            ReciteDialogueService service = null;
            try
            {
                service = ServiceFromContext(userdata);
                return service == null
                    ? InvalidLocaleCallbackResult()
                    : service.EvaluateLocaleCore(queryPtr);
            }
            catch (Exception error)
            {
                return service?.LocaleFailure(error.Message) ?? InvalidLocaleCallbackResult();
            }
        }

        private ReciteNativeBridge.ReciteLocaleResult EvaluateLocaleCore(IntPtr queryPtr)
        {
            if (queryPtr == IntPtr.Zero)
            {
                return LocaleFailure("locale query pointer was null");
            }
            if (localeCatalog == null)
            {
                return LocaleFailure("no locale catalogue is configured");
            }

            try
            {
                var query = Marshal.PtrToStructure<ReciteNativeBridge.ReciteLocaleQuery>(queryPtr);
                var id = ReadLocaleString(query.Id, "locale ID");
                var sourceText = ReadLocaleString(query.SourceText, "locale source text");
                var locale = ReadLocaleString(query.Locale, "locale");
                var variant = query.Variant == IntPtr.Zero ? null : ReadLocaleString(query.Variant, "locale variant");
                if (query.Domain > (uint)ReciteLocaleTextDomain.PresentationLabel)
                {
                    return LocaleFailure("locale query has an unknown text domain");
                }
                var domain = (ReciteLocaleTextDomain)query.Domain;
                if (query.Kind == 0)
                {
                    return LocaleSuccess(localeCatalog.Lookup(id, sourceText, domain, locale, variant), null, null, null, null, null);
                }
                if (query.Kind != 1 || query.PluralSourceText == IntPtr.Zero)
                {
                    return LocaleFailure("locale query has an unknown request kind");
                }

                var sourcePlural = ReadLocaleString(query.PluralSourceText, "locale plural source text");
                var resolution = localeCatalog.ResolvePlural(id, sourceText, sourcePlural, query.Count, domain, locale, variant);
                var result = LocaleSuccess(
                    resolution.Text,
                    resolution.SelectedArm,
                    resolution.MatchedLocale,
                    resolution.MatchedContext,
                    resolution.MatchedKey,
                    resolution.Attempts);
                return result;
            }
            catch (Exception error)
            {
                return LocaleFailure(error.Message);
            }
        }

        private ReciteNativeBridge.ReciteLocaleResult LocaleSuccess(
            string text,
            int? selectedArm,
            string matchedLocale,
            string matchedContext,
            string matchedKey,
            IReadOnlyList<ReciteManagedPluralAttempt> attempts)
        {
            var nativeAttempts = IntPtr.Zero;
            var attemptCount = UIntPtr.Zero;
            if (attempts != null && attempts.Count > 0)
            {
                var size = Marshal.SizeOf<ReciteNativeBridge.ReciteLocaleAttempt>();
                nativeAttempts = Marshal.AllocHGlobal(checked(size * attempts.Count));
                localeCallbackAllocations.Add(nativeAttempts);
                for (var index = 0; index < attempts.Count; index++)
                {
                    var attempt = attempts[index];
                    var native = new ReciteNativeBridge.ReciteLocaleAttempt
                    {
                        Locale = AllocateLocaleString(attempt.Locale),
                        Context = AllocateLocaleString(attempt.Context),
                        Key = AllocateLocaleString(attempt.Key),
                        SelectedArm = attempt.SelectedArm ?? -1,
                        Outcome = OutcomeNumber(attempt.Outcome)
                    };
                    Marshal.StructureToPtr(native, IntPtr.Add(nativeAttempts, index * size), false);
                }
                attemptCount = new UIntPtr((ulong)attempts.Count);
            }
            return new ReciteNativeBridge.ReciteLocaleResult
            {
                Ok = 1,
                Text = AllocateLocaleString(text),
                SelectedArm = selectedArm ?? -1,
                MatchedLocale = AllocateLocaleString(matchedLocale),
                MatchedContext = AllocateLocaleString(matchedContext),
                MatchedKey = AllocateLocaleString(matchedKey),
                Attempts = nativeAttempts,
                AttemptsLen = attemptCount,
                ErrorMessage = IntPtr.Zero
            };
        }

        private ReciteNativeBridge.ReciteLocaleResult LocaleFailure(string message)
        {
            return new ReciteNativeBridge.ReciteLocaleResult
            {
                Ok = 0,
                ErrorMessage = AllocateLocaleString(message ?? "Unity locale callback failed")
            };
        }

        private IntPtr AllocateLocaleString(string value)
        {
            if (value == null) return IntPtr.Zero;
            value = ReciteStringValidation.Validate(value, "locale callback string", allowEmpty: true);
            var bytes = System.Text.Encoding.UTF8.GetBytes(value);
            var pointer = Marshal.AllocHGlobal(checked(bytes.Length + 1));
            Marshal.Copy(bytes, 0, pointer, bytes.Length);
            Marshal.WriteByte(pointer, bytes.Length, 0);
            localeCallbackAllocations.Add(pointer);
            return pointer;
        }

        private void FreeLocaleCallbackAllocations()
        {
            foreach (var pointer in localeCallbackAllocations)
            {
                Marshal.FreeHGlobal(pointer);
            }
            localeCallbackAllocations.Clear();
        }

        private static string ReadLocaleString(IntPtr pointer, string name)
        {
            if (pointer == IntPtr.Zero) throw new FormatException(name + " pointer was null");
            return ReciteStringValidation.Validate(
                Marshal.PtrToStringUTF8(pointer) ?? throw new FormatException(name + " was not valid UTF-8"),
                name);
        }

        private static uint OutcomeNumber(string outcome)
        {
            switch (outcome)
            {
                case "missing_plural_forms": return 0;
                case "missing_entry": return 1;
                case "missing_translation": return 2;
                case "matched": return 3;
                default: throw new FormatException("unknown plural attempt outcome");
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

        private static ReciteDialogueService ServiceFromContext(IntPtr userdata)
        {
            if (userdata == IntPtr.Zero)
            {
                return null;
            }
            return GCHandle.FromIntPtr(userdata).Target as ReciteDialogueService;
        }

        private static ReciteNativeBridge.ReciteConditionResult InvalidConditionCallbackResult()
        {
            return new ReciteNativeBridge.ReciteConditionResult { Ok = 0 };
        }

        private static ReciteNativeBridge.ReciteLocaleResult InvalidLocaleCallbackResult()
        {
            return new ReciteNativeBridge.ReciteLocaleResult { Ok = 0 };
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
