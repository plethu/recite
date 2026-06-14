using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;

namespace Recite.Unity.Native
{
    internal static class ReciteNativeBridge
    {
        internal const uint AbiMajor = 0;
        internal const uint AbiMinor = 0;
        internal const uint AbiPatch = 1;
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

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "recite_asset_load")]
        internal static extern ReciteStatus AssetLoad(byte[] bytes, UIntPtr len, out ulong assetHandle);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "recite_asset_free")]
        internal static extern void AssetFree(ulong assetHandle);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "recite_session_create")]
        internal static extern ReciteStatus SessionCreate(ulong assetHandle, byte[] startBlock, byte[] locale, out ulong sessionHandle);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "recite_session_begin")]
        internal static extern ReciteStatus SessionBegin(ulong sessionHandle, out ReciteBuffer batch);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "recite_session_start")]
        internal static extern ReciteStatus SessionStart(ulong assetHandle, byte[] startBlock, byte[] locale, out ulong sessionHandle, out ReciteBuffer batch);

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

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "recite_session_free")]
        internal static extern void SessionFree(ulong sessionHandle);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "recite_buffer_free")]
        internal static extern void BufferFree(ref ReciteBuffer buffer);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "recite_last_error_message")]
        private static extern IntPtr LastErrorMessagePtr();

        internal static byte[] ToUtf8NullTerminated(string value)
        {
            if (string.IsNullOrEmpty(value))
            {
                return null;
            }

            var bytes = Encoding.UTF8.GetBytes(value);
            var terminated = new byte[bytes.Length + 1];
            Buffer.BlockCopy(bytes, 0, terminated, 0, bytes.Length);
            return terminated;
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
            if (data == IntPtr.Zero || len == UIntPtr.Zero)
            {
                return Array.Empty<object>();
            }

            var bytes = new byte[checked((int)len.ToUInt64())];
            Marshal.Copy(data, bytes, 0, bytes.Length);
            return ReciteMessagePack.DecodeConditionArgs(bytes);
        }
    }
}
