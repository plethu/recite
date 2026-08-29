using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
using Recite.Unity;

namespace Recite.Unity.Native
{
    internal static class ReciteNativeBridge
    {
        internal const uint AbiMajor = 0;
        internal const uint AbiMinor = 5;
        internal const uint AbiPatch = 0;
        private const string LibraryName = "recite_ffi";

        [StructLayout(LayoutKind.Sequential)]
        internal struct ReciteBuffer
        {
            internal IntPtr Data;
            internal UIntPtr Len;
        }

        [StructLayout(LayoutKind.Sequential)]
        internal struct ReciteConditionQuery
        {
            internal IntPtr FunctionName;
            internal IntPtr ArgsMsgpack;
            internal UIntPtr ArgsLen;
        }

        [StructLayout(LayoutKind.Sequential)]
        internal struct ReciteConditionResult
        {
            internal byte Ok;
            internal IntPtr ValueMsgpack;
            internal UIntPtr ValueLen;
            internal IntPtr ErrorMessage;
        }

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        internal delegate ReciteConditionResult ReciteConditionFn(IntPtr query, IntPtr userdata);

        [StructLayout(LayoutKind.Sequential)]
        internal struct ReciteLocaleQuery
        {
            internal uint Kind;
            internal IntPtr Id;
            internal IntPtr SourceText;
            internal IntPtr PluralSourceText;
            internal long Count;
            internal uint Domain;
            internal IntPtr Locale;
            internal IntPtr Variant;
        }

        [StructLayout(LayoutKind.Sequential)]
        internal struct ReciteLocaleAttempt
        {
            internal IntPtr Locale;
            internal IntPtr Context;
            internal IntPtr Key;
            internal int SelectedArm;
            internal uint Outcome;
        }

        [StructLayout(LayoutKind.Sequential)]
        internal struct ReciteLocaleResult
        {
            internal byte Ok;
            internal IntPtr Text;
            internal int SelectedArm;
            internal IntPtr MatchedLocale;
            internal IntPtr MatchedContext;
            internal IntPtr MatchedKey;
            internal IntPtr Attempts;
            internal UIntPtr AttemptsLen;
            internal IntPtr ErrorMessage;
        }

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        internal delegate ReciteLocaleResult ReciteLocaleFn(IntPtr query, IntPtr userdata);

        internal sealed class InterpolationValueBuffer : IDisposable
        {
            private readonly List<IntPtr> stringPointers = new List<IntPtr>();
            private IntPtr records;
            private bool disposed;

            internal InterpolationValueBuffer(IReadOnlyList<ReciteInterpolationValue> values)
            {
                if (values == null)
                {
                    throw new ArgumentNullException(nameof(values));
                }

                if (values.Count == 0)
                {
                    return;
                }

                var ordered = new List<ReciteInterpolationValue>(values);
                ordered.Sort((left, right) => StringComparer.Ordinal.Compare(left.Name, right.Name));
                Count = ordered.Count;
                var size = Marshal.SizeOf<ReciteInterpolationValueNative>();
                records = Marshal.AllocHGlobal(checked(size * ordered.Count));
                try
                {
                    for (var index = 0; index < ordered.Count; index++)
                    {
                        var value = ordered[index] ?? throw new ArgumentException(
                            "interpolation values cannot contain null entries",
                            nameof(values));
                        if (index > 0 && StringComparer.Ordinal.Equals(value.Name, ordered[index - 1].Name))
                        {
                            throw new ArgumentException(
                                "interpolation value names must be unique",
                                nameof(values));
                        }

                        var native = new ReciteInterpolationValueNative
                        {
                            Name = AllocateUtf8(ReciteStringValidation.Validate(value.Name, nameof(values)), stringPointers),
                            Kind = (uint)value.Kind,
                            StringValue = IntPtr.Zero,
                            IntegerValue = value.IntegerValue,
                            FloatValue = value.FloatValue,
                            BooleanValue = value.BooleanValue ? (byte)1 : (byte)0
                        };
                        if (value.Kind == ReciteInterpolationValueKind.String)
                        {
                            native.StringValue = AllocateUtf8(ReciteStringValidation.Validate(value.StringValue, nameof(values)), stringPointers);
                        }

                        Marshal.StructureToPtr(native, IntPtr.Add(records, index * size), false);
                    }
                }
                catch
                {
                    Dispose();
                    throw;
                }
            }

            internal IntPtr Pointer => records;

            internal UIntPtr Length => new UIntPtr((ulong)Count);

            internal int Count { get; }

            public void Dispose()
            {
                if (disposed)
                {
                    return;
                }

                disposed = true;
                if (records != IntPtr.Zero)
                {
                    Marshal.FreeHGlobal(records);
                    records = IntPtr.Zero;
                }

                foreach (var pointer in stringPointers)
                {
                    Marshal.FreeHGlobal(pointer);
                }
                stringPointers.Clear();
            }

            private static IntPtr AllocateUtf8(string value, ICollection<IntPtr> allocated)
            {
                value = ReciteStringValidation.Validate(value, nameof(value));

                var bytes = Encoding.UTF8.GetBytes(value);
                var pointer = Marshal.AllocHGlobal(checked(bytes.Length + 1));
                Marshal.Copy(bytes, 0, pointer, bytes.Length);
                Marshal.WriteByte(pointer, bytes.Length, 0);
                allocated.Add(pointer);
                return pointer;
            }
        }

        [StructLayout(LayoutKind.Sequential)]
        private struct ReciteInterpolationValueNative
        {
            internal IntPtr Name;
            internal uint Kind;
            internal IntPtr StringValue;
            internal long IntegerValue;
            internal double FloatValue;
            internal byte BooleanValue;
        }

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "recite_asset_load")]
        internal static extern ReciteStatus AssetLoad(byte[] bytes, UIntPtr len, out ulong assetHandle);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "recite_asset_free")]
        internal static extern void AssetFree(ulong assetHandle);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "recite_session_create")]
        internal static extern ReciteStatus SessionCreate(ulong assetHandle, byte[] startBlock, byte[] locale, out ulong sessionHandle);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "recite_session_create_with_values")]
        internal static extern ReciteStatus SessionCreateWithValues(ulong assetHandle, byte[] startBlock, byte[] locale, IntPtr values, UIntPtr valuesLen, out ulong sessionHandle);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "recite_session_begin")]
        internal static extern ReciteStatus SessionBegin(ulong sessionHandle, out ReciteBuffer batch);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "recite_session_start")]
        internal static extern ReciteStatus SessionStart(ulong assetHandle, byte[] startBlock, byte[] locale, out ulong sessionHandle, out ReciteBuffer batch);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "recite_session_start_with_values")]
        internal static extern ReciteStatus SessionStartWithValues(ulong assetHandle, byte[] startBlock, byte[] locale, IntPtr values, UIntPtr valuesLen, out ulong sessionHandle, out ReciteBuffer batch);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "recite_session_start_with_locale_provider")]
        internal static extern ReciteStatus SessionStartWithLocaleProvider(ulong assetHandle, byte[] startBlock, byte[] locale, ReciteLocaleFn callback, IntPtr userdata, out ulong sessionHandle, out ReciteBuffer batch);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "recite_session_start_with_locale_provider_and_variant")]
        internal static extern ReciteStatus SessionStartWithLocaleProviderAndVariant(ulong assetHandle, byte[] startBlock, byte[] locale, byte[] localeVariant, ReciteLocaleFn callback, IntPtr userdata, out ulong sessionHandle, out ReciteBuffer batch);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "recite_session_start_with_values_and_locale_provider")]
        internal static extern ReciteStatus SessionStartWithValuesAndLocaleProvider(ulong assetHandle, byte[] startBlock, byte[] locale, IntPtr values, UIntPtr valuesLen, ReciteLocaleFn callback, IntPtr userdata, out ulong sessionHandle, out ReciteBuffer batch);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "recite_session_start_with_values_and_locale_provider_and_variant")]
        internal static extern ReciteStatus SessionStartWithValuesAndLocaleProviderAndVariant(ulong assetHandle, byte[] startBlock, byte[] locale, byte[] localeVariant, IntPtr values, UIntPtr valuesLen, ReciteLocaleFn callback, IntPtr userdata, out ulong sessionHandle, out ReciteBuffer batch);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "recite_session_register_condition")]
        internal static extern ReciteStatus SessionRegisterCondition(ulong sessionHandle, byte[] name, ReciteConditionFn handler, IntPtr userdata);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "recite_session_choose")]
        internal static extern ReciteStatus SessionChoose(ulong sessionHandle, byte[] choiceId, out ReciteBuffer batch);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "recite_session_acknowledge_effect")]
        internal static extern ReciteStatus SessionAcknowledgeEffect(ulong sessionHandle, byte[] effectRequestId, byte ackCompleted, byte[] failureReason, out ReciteBuffer batch);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "recite_session_snapshot")]
        internal static extern ReciteStatus SessionSnapshot(ulong sessionHandle, out ReciteBuffer snapshot);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "recite_session_restore")]
        internal static extern ReciteStatus SessionRestore(ulong assetHandle, byte[] snapshotBytes, UIntPtr snapshotLen, out ulong sessionHandle, out ReciteBuffer batch);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "recite_session_restore_with_values")]
        internal static extern ReciteStatus SessionRestoreWithValues(ulong assetHandle, byte[] snapshotBytes, UIntPtr snapshotLen, IntPtr values, UIntPtr valuesLen, out ulong sessionHandle, out ReciteBuffer batch);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "recite_session_restore_with_values_and_locale_provider")]
        internal static extern ReciteStatus SessionRestoreWithValuesAndLocaleProvider(ulong assetHandle, byte[] snapshotBytes, UIntPtr snapshotLen, IntPtr values, UIntPtr valuesLen, ReciteLocaleFn callback, IntPtr userdata, out ulong sessionHandle, out ReciteBuffer batch);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "recite_session_restore_with_values_and_locale_provider_and_variant")]
        internal static extern ReciteStatus SessionRestoreWithValuesAndLocaleProviderAndVariant(ulong assetHandle, byte[] snapshotBytes, UIntPtr snapshotLen, IntPtr values, UIntPtr valuesLen, byte[] localeVariant, ReciteLocaleFn callback, IntPtr userdata, out ulong sessionHandle, out ReciteBuffer batch);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "recite_session_set_interpolation_values")]
        internal static extern ReciteStatus SessionSetInterpolationValues(ulong sessionHandle, IntPtr values, UIntPtr valuesLen);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "recite_session_set_locale_provider")]
        internal static extern ReciteStatus SessionSetLocaleProvider(ulong sessionHandle, ReciteLocaleFn callback, IntPtr userdata);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "recite_session_set_locale_variant")]
        internal static extern ReciteStatus SessionSetLocaleVariant(ulong sessionHandle, byte[] variant);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "recite_session_clear_locale_provider")]
        internal static extern ReciteStatus SessionClearLocaleProvider(ulong sessionHandle);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "recite_session_free")]
        internal static extern void SessionFree(ulong sessionHandle);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "recite_buffer_free")]
        internal static extern void BufferFree(ref ReciteBuffer buffer);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "recite_last_error_message")]
        private static extern IntPtr LastErrorMessagePtr();

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "recite_locale_validate_plural_rule")]
        private static extern ReciteStatus ValidatePluralRuleNative(byte[] header, out UIntPtr nplurals);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "recite_locale_evaluate_plural_rule")]
        private static extern ReciteStatus EvaluatePluralRuleNative(byte[] header, long count, out UIntPtr arm);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "recite_locale_validate_translation_placeholders")]
        private static extern ReciteStatus ValidateTranslationPlaceholdersNative(byte[] source, byte[] translation);

        internal static byte[] ToUtf8NullTerminated(string value)
        {
            value = ReciteStringValidation.Validate(value, nameof(value), allowNull: true, allowEmpty: true);
            if (string.IsNullOrEmpty(value))
            {
                return null;
            }

            var bytes = Encoding.UTF8.GetBytes(value);
            var terminated = new byte[bytes.Length + 1];
            Buffer.BlockCopy(bytes, 0, terminated, 0, bytes.Length);
            return terminated;
        }

        internal static int ValidatePluralRule(string header)
        {
            header = ReciteStringValidation.Validate(header, nameof(header));

            var status = ValidatePluralRuleNative(ToUtf8NullTerminated(header), out var nplurals);
            ThrowIfError(status);
            return checked((int)nplurals.ToUInt64());
        }

        internal static int EvaluatePluralRule(string header, long count, int expectedArmCount)
        {
            header = ReciteStringValidation.Validate(header, nameof(header));
            var status = EvaluatePluralRuleNative(ToUtf8NullTerminated(header), count, out var arm);
            ThrowIfError(status);
            var selected = checked((int)arm.ToUInt64());
            if (selected < 0 || selected >= expectedArmCount)
            {
                throw new ReciteAdapterException(ReciteStatus.Localisation, "native plural rule returned an invalid arm");
            }
            return selected;
        }

        internal static void ValidateTranslationPlaceholders(string source, string translation)
        {
            source = ReciteStringValidation.Validate(source, nameof(source));
            translation = ReciteStringValidation.Validate(translation, nameof(translation), allowEmpty: true);
            if (string.IsNullOrEmpty(translation))
            {
                return;
            }
            ThrowIfError(ValidateTranslationPlaceholdersNative(
                ToUtf8NullTerminated(source), ToUtf8NullTerminated(translation)));
        }

        internal static byte[] CopyAndFree(ref ReciteBuffer buffer)
        {
            try
            {
                var len = checked((int)buffer.Len.ToUInt64());
                if (buffer.Data == IntPtr.Zero || len == 0)
                {
                    return Array.Empty<byte>();
                }

                var bytes = new byte[len];
                Marshal.Copy(buffer.Data, bytes, 0, len);
                return bytes;
            }
            finally
            {
                BufferFree(ref buffer);
            }
        }

        internal static string LastErrorMessage()
        {
            var ptr = LastErrorMessagePtr();
            return ptr == IntPtr.Zero ? string.Empty : Marshal.PtrToStringUTF8(ptr) ?? string.Empty;
        }

        internal static void ThrowIfError(ReciteStatus status)
        {
            if (status != ReciteStatus.Ok)
            {
                throw new ReciteAdapterException(status, LastErrorMessage());
            }
        }

        internal static IReadOnlyList<object> ReadConditionArgs(IntPtr data, UIntPtr len)
        {
            var typed = ReadTypedConditionArgs(data, len);
            var args = new List<object>(typed.Count);
            foreach (var argument in typed)
            {
                args.Add(argument.LegacyValue);
            }

            return args;
        }

        internal static IReadOnlyList<ReciteConditionArgument> ReadTypedConditionArgs(IntPtr data, UIntPtr len)
        {
            var rawLength = len.ToUInt64();
            if (rawLength > int.MaxValue)
            {
                throw new FormatException("condition argument payload is too large");
            }

            var length = (int)rawLength;
            if (data == IntPtr.Zero)
            {
                throw new FormatException("condition argument payload pointer is null");
            }

            if (length == 0)
            {
                throw new FormatException("condition argument payload is empty");
            }

            var bytes = new byte[length];
            Marshal.Copy(data, bytes, 0, bytes.Length);
            return ReciteMessagePack.DecodeTypedConditionArgs(bytes);
        }
    }
}
